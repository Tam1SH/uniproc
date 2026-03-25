use crate::features::processes::domain::process_flow::ProcessFlowState;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::features::processes::ui::slint_bridge::{
    BridgeSnapshot, ColumnWidthConfig, VisitorSharedState, build_snapshot,
};
use app_contracts::features::agents::RemoteScanResult;
use app_contracts::features::navigation::TabChanged;
use app_contracts::features::processes::{
    ProcessEntryVm, ProcessFieldDto, ProcessGroupVm, ProcessesUiPort,
};
use app_core::actor::traits::{Context, Handler, Message, NoOp};
use app_core::messages;
use app_core::windowed_rows::WindowedRows;
use slint::ComponentHandle;
use std::collections::{HashMap, HashSet};
use sysinfo::{Pid, ProcessesToUpdate, System};

#[cfg(target_os = "windows")]
use crate::features::processes::scanner::windows::WindowsReport;

use crate::features::processes::scanner::wsl::visitor::WslScanResult;

messages! {
    Sort(String),
    ToggleExpand(String),
    ViewportChanged { start: usize, count: usize },
    Select { pid: u32, idx: usize },
    TerminateSelected,
}

pub struct ProcessActor<P: ProcessesUiPort> {
    pub flow: ProcessFlowState,
    pub metadata: ProcessMetadataService,
    pub widths_by_schema: HashMap<&'static str, VisitorSharedState>,
    pub width_config: ColumnWidthConfig,
    pub is_active: bool,
    pub ui_port: P,
    pub rows_window: WindowedRows<ProcessEntryVm>,
    pub snapshots: HashMap<&'static str, BridgeSnapshot>,
}

impl<P: ProcessesUiPort> ProcessActor<P> {
    fn push_rows_window(&self) {
        let batch = self.rows_window.batch();
        self.ui_port
            .set_process_rows_window(batch.total_rows, batch.start, batch.rows);
    }

    fn shared_state_for(&mut self, schema_id: &'static str) -> VisitorSharedState {
        self.widths_by_schema
            .entry(schema_id)
            .or_insert_with(|| VisitorSharedState::with_config(&self.width_config))
            .clone()
    }

    fn apply_snapshot(&mut self, schema_id: &'static str, snapshot: BridgeSnapshot) {
        self.snapshots.insert(schema_id, snapshot);
        self.rebuild();
    }

    fn rebuild(&mut self) {
        let all: Vec<BridgeSnapshot> = self.snapshots.values().cloned().collect();
        if all.is_empty() {
            return;
        }

        let mut column_defs = Vec::new();
        let mut seen = HashSet::<String>::new();
        let mut processes = Vec::new();
        for s in all {
            for def in s.column_defs {
                if seen.insert(def.id.to_string()) {
                    column_defs.push(def);
                }
            }
            processes.extend(s.processes);
        }

        self.flow.set_snapshot(BridgeSnapshot {
            column_defs,
            processes,
        });
        self.ui_port.set_loading(false);
        self.refresh_ui_model();

        let total: usize = self.snapshots.values().map(|s| s.processes.len()).sum();
        self.ui_port.set_total_processes_count(total);
    }

    fn refresh_ui_model(&mut self) {
        if !self.flow.has_snapshot() {
            return;
        }
        self.ui_port.set_column_defs(self.flow.column_defs());
        let Some(mut groups) = self.flow.build_groups(&mut self.metadata) else {
            return;
        };
        for group in &mut groups {
            stabilize_fields(&mut group.parent.fields);
        }
        self.rows_window.set_items(flatten_groups(groups));
        self.push_rows_window();
    }
}

fn stabilize_fields(fields: &mut Vec<ProcessFieldDto>) {
    for f in fields.iter_mut() {
        if f.numeric >= 0.0 {
            f.numeric = (f.numeric * 10.0).round() / 10.0;
        }
    }
}

fn flatten_groups(groups: Vec<ProcessGroupVm>) -> Vec<ProcessEntryVm> {
    let mut rows = Vec::new();
    for group in groups {
        rows.push(group.parent);
        rows.extend(group.children);
    }
    rows
}

impl<P, TWindow> Handler<TabChanged, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: TabChanged, _ctx: &Context<Self, TWindow>) {
        self.is_active = msg.name == "Processes";
    }
}

impl<P, TWindow> Handler<RemoteScanResult, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: RemoteScanResult, _ctx: &Context<Self, TWindow>) {
        if !self.is_active {
            return;
        }
        let result = WslScanResult {
            processes: msg.processes,
            machine: msg.machine,
        };
        let shared = self.shared_state_for(msg.schema_id);
        let snapshot = build_snapshot(&result, &shared);
        self.apply_snapshot(msg.schema_id, snapshot);
    }
}

#[cfg(target_os = "windows")]
impl<P, TWindow> Handler<WindowsReport, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: WindowsReport, _ctx: &Context<Self, TWindow>) {
        if !self.is_active {
            return;
        }
        let shared = self.shared_state_for("windows");
        let snapshot = build_snapshot(&msg.0, &shared);
        self.apply_snapshot("windows", snapshot);
    }
}

impl<P, TWindow> Handler<Sort, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: Sort, _ctx: &Context<Self, TWindow>) {
        let sort = self.flow.toggle_sort(msg.0.as_str());
        self.ui_port.set_sort_state(msg.0, sort.metric_descending);
        self.refresh_ui_model();
    }
}

impl<P, TWindow> Handler<ViewportChanged, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: ViewportChanged, _ctx: &Context<Self, TWindow>) {
        self.rows_window.set_viewport(msg.start, msg.count.max(1));
        self.push_rows_window();
    }
}

impl<P, TWindow> Handler<TerminateSelected, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, _: TerminateSelected, ctx: &Context<Self, TWindow>) {
        let pid = Some(self.ui_port.get_selected_pid());
        let Some(pid) = pid.filter(|&p| p != -1).map(|p| p as u32) else {
            return;
        };
        ctx.spawn_bg(async move {
            let mut system = System::new();
            system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), false);
            if let Some(process) = system.process(Pid::from_u32(pid)) {
                process.kill();
            }
            NoOp
        });
        self.flow.clear_selection();
    }
}

impl<P, TWindow> Handler<Select, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: Select, _ctx: &Context<Self, TWindow>) {
        self.flow.select(msg.pid, msg.idx);
        self.ui_port.set_selected_pid(msg.pid as i32);
        if let Some(name) = self.flow.selected_name_for_pid(msg.pid) {
            self.ui_port.set_selected_name(name);
        }
    }
}

impl<P, TWindow> Handler<ToggleExpand, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: ToggleExpand, _ctx: &Context<Self, TWindow>) {
        self.flow.toggle_expand(msg.0);
        self.refresh_ui_model();
    }
}
