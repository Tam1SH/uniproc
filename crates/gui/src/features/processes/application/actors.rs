use crate::core::actor::traits::{Context, Handler, Message, NoOp};
use crate::features::envs::wsl::WslAgentRuntimeEvent;
use crate::features::navigation::TabChanged;
use crate::features::processes::domain::process_flow::ProcessFlowState;
use crate::features::processes::scanner::base::{ProcessScanner, ScanResult};
use crate::features::processes::scanner::wsl::SharedWslClient;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::features::processes::ui::slint_bridge::{
    BridgeSnapshot, ColumnWidthConfig, VisitorSharedState, build_snapshot,
};
use crate::{AppWindow, MainBodyState, ProcessField, ProcessGroup, ProcessesFeatureGlobal};
use app_core::messages;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
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

    Sort(SharedString),
    ToggleExpand(SharedString),
    Select { pid: u32, idx: usize },
    TerminateSelected,
}

pub struct ProcessActor {
    pub flow: ProcessFlowState,
    pub metadata: ProcessMetadataService,
    pub ui_model: Rc<VecModel<ProcessGroup>>,

    pub scanners: Option<Vec<Box<dyn ProcessScanner>>>,
    pub widths_by_schema: HashMap<&'static str, VisitorSharedState>,
    pub width_config: ColumnWidthConfig,
    pub is_active: bool,
    pub wsl_client: SharedWslClient,
}

impl ProcessActor {
    fn refresh_ui_model(&mut self, ui: &AppWindow) {
        if !self.flow.has_snapshot() {
            return;
        }

        let bridge = ui.global::<ProcessesFeatureGlobal>();
        bridge.set_column_defs(Rc::new(VecModel::from(self.flow.column_defs())).into());

        let Some(mut groups) = self.flow.build_groups(&mut self.metadata) else {
            return;
        };

        for (i, mut group) in groups.drain(..).enumerate() {
            stabilize_fields(&mut group.parent.fields);

            if i < self.ui_model.row_count() {
                if let Some(existing) = self.ui_model.row_data(i) {
                    if existing != group {
                        self.ui_model.set_row_data(i, group);
                    }
                }
            } else {
                self.ui_model.push(group);
            }
        }
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

fn stabilize_fields(fields: &mut ModelRc<ProcessField>) {
    let vec = fields
        .iter()
        .map(|mut f| {
            if f.numeric >= 0.0 {
                f.numeric = (f.numeric * 10.0).round() / 10.0;
            }
            f
        })
        .collect::<Vec<_>>();
    *fields = Rc::new(VecModel::from(vec)).into();
}

impl Handler<TabChanged, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: TabChanged, ctx: &Context<Self, AppWindow>) {
        self.is_active = msg.name == "Processes";
        if self.is_active {
            ctx.addr().send(ScanTick);
        }
    }
}

impl Handler<ScanTick, AppWindow> for ProcessActor {
    fn handle(&mut self, _: ScanTick, ctx: &Context<Self, AppWindow>) {
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

impl Handler<WslAgentRuntimeEvent, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: WslAgentRuntimeEvent, ctx: &Context<Self, AppWindow>) {
        let _state = msg.state;
        if let Ok(mut client) = self.wsl_client.write() {
            *client = msg.client;
        }

        if self.is_active {
            ctx.addr().send(ScanTick);
        }
    }
}

impl Handler<ScanResponse, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: ScanResponse, ctx: &Context<Self, AppWindow>) {
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

        ctx.with_ui(|ui| {
            ui.global::<MainBodyState>()
                .set_is_loading(!self.flow.has_snapshot());
            self.refresh_ui_model(ui);
        });
    }
}

impl Handler<Sort, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: Sort, ctx: &Context<Self, AppWindow>) {
        let sort = self.flow.toggle_sort(msg.0.as_str());

        ctx.with_ui(|ui| {
            let bridge = ui.global::<ProcessesFeatureGlobal>();
            bridge.set_current_sort(msg.0.into());
            bridge.set_current_sort_descending(sort.metric_descending);
            self.refresh_ui_model(ui);
        });
    }
}

impl Handler<TerminateSelected, AppWindow> for ProcessActor {
    fn handle(&mut self, _msg: TerminateSelected, ctx: &Context<Self, AppWindow>) {
        let pid = ctx.with_ui(|ui| ui.global::<ProcessesFeatureGlobal>().get_selected_pid());
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

impl Handler<Select, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: Select, ctx: &Context<Self, AppWindow>) {
        self.flow.select(msg.pid, msg.idx);
        ctx.with_ui(|ui| {
            let bridge = ui.global::<ProcessesFeatureGlobal>();
            bridge.set_selected_pid(msg.pid as i32);

            if let Some(name) = self.flow.selected_name_for_pid(msg.pid) {
                bridge.set_selected_name(name.into());
            }
        });
    }
}

impl Handler<ToggleExpand, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: ToggleExpand, ctx: &Context<Self, AppWindow>) {
        self.flow.toggle_expand(msg.0);
        ctx.with_ui(|ui| self.refresh_ui_model(ui));
    }
}
