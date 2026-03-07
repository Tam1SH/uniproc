use crate::features::processes::domain::process_flow::ProcessFlowState;
use crate::features::processes::scanner::base::{ProcessScanner, ScanResult};
use crate::features::processes::scanner::wsl::SharedWslClient;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::features::processes::ui::slint_bridge::{
    BridgeSnapshot, ColumnWidthConfig, VisitorSharedState, build_snapshot,
};
use app_contracts::features::environments::WslAgentRuntimeEvent;
use app_contracts::features::navigation::TabChanged;
use app_contracts::features::processes::ProcessesUiPort;
use app_core::actor::traits::{Context, Handler, Message, NoOp};
use app_core::messages;
use slint::ComponentHandle;
use std::collections::{HashMap, HashSet};
use sysinfo::{Pid, ProcessesToUpdate, System};

struct ScannedResult {
    schema_id: &'static str,
    result: Box<dyn ScanResult>,
}

pub struct ScanResponse {
    scanned: Vec<ScannedResult>,
    scanners: Vec<Box<dyn ProcessScanner>>,
}
impl Message for ScanResponse {}

messages! {
    ScanTick,

    Sort(String),
    ToggleExpand(String),
    Select { pid: u32, idx: usize },
    TerminateSelected,
}

pub struct ProcessActor<P: ProcessesUiPort> {
    pub flow: ProcessFlowState,
    pub metadata: ProcessMetadataService,
    pub scanners: Option<Vec<Box<dyn ProcessScanner>>>,
    pub widths_by_schema: HashMap<&'static str, VisitorSharedState>,
    pub width_config: ColumnWidthConfig,
    pub is_active: bool,
    pub wsl_client: SharedWslClient,
    pub ui_port: P,
}

impl<P: ProcessesUiPort> ProcessActor<P> {
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

        self.ui_port.set_process_groups(groups);
    }

    fn merge_snapshots(snapshots: Vec<BridgeSnapshot>) -> BridgeSnapshot {
        let mut column_defs = Vec::new();
        let mut seen_column_ids = HashSet::<String>::new();
        let mut processes = Vec::new();

        for snapshot in snapshots {
            for def in snapshot.column_defs {
                let key = def.id.to_string();
                if seen_column_ids.insert(key) {
                    column_defs.push(def);
                }
            }
            processes.extend(snapshot.processes);
        }

        BridgeSnapshot {
            column_defs,
            processes,
        }
    }
}

fn stabilize_fields(fields: &mut Vec<app_contracts::features::processes::ProcessFieldDto>) {
    for f in fields.iter_mut() {
        if f.numeric >= 0.0 {
            f.numeric = (f.numeric * 10.0).round() / 10.0;
        }
    }
}

impl<P, TWindow> Handler<TabChanged, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: TabChanged, ctx: &Context<Self, TWindow>) {
        self.is_active = msg.name == "Processes";
        if self.is_active {
            ctx.addr().send(ScanTick);
        }
    }
}

impl<P, TWindow> Handler<ScanTick, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, _: ScanTick, ctx: &Context<Self, TWindow>) {
        if !self.is_active {
            return;
        }

        if let Some(mut scanners) = self.scanners.take() {
            ctx.spawn_task(
                async move {
                    let mut scanned = Vec::with_capacity(scanners.len());

                    for scanner in scanners.iter_mut() {
                        let schema_id = scanner.schema_id();
                        let result = scanner.scan().await;
                        scanned.push(ScannedResult { schema_id, result });
                    }

                    ScanResponse { scanned, scanners }
                },
                move |_, _| {},
            );
        }
    }
}

impl<P, TWindow> Handler<WslAgentRuntimeEvent, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: WslAgentRuntimeEvent, ctx: &Context<Self, TWindow>) {
        let _state = msg.state;
        if let Ok(mut client) = self.wsl_client.write() {
            *client = msg.client;
        }

        if self.is_active {
            ctx.addr().send(ScanTick);
        }
    }
}

impl<P, TWindow> Handler<ScanResponse, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: ScanResponse, _ctx: &Context<Self, TWindow>) {
        self.scanners = Some(msg.scanners);

        let mut snapshots = Vec::new();
        for item in msg.scanned {
            let shared = self
                .widths_by_schema
                .entry(item.schema_id)
                .or_insert_with(|| VisitorSharedState::with_config(&self.width_config))
                .clone();
            snapshots.push(build_snapshot(item.result.as_ref(), &shared));
        }

        if !snapshots.is_empty() {
            self.flow.set_snapshot(Self::merge_snapshots(snapshots));
        }

        self.ui_port.set_loading(!self.flow.has_snapshot());
        self.refresh_ui_model();
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

impl<P, TWindow> Handler<TerminateSelected, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, _msg: TerminateSelected, ctx: &Context<Self, TWindow>) {
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
