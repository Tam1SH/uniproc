use crate::features::processes::domain::table::ProcessTable;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::processes_impl::application::process_snapshot_actor::{
    ActiveStatus, ProcessSnapshotReady,
};
use crate::processes_impl::domain::snapshot::BridgeSnapshot;
#[cfg(target_os = "windows")]
use app_contracts::features::agents::WindowsAgentRuntimeEvent;
use app_contracts::features::agents::{AgentConnectionState, WslAgentRuntimeEvent};
use app_contracts::features::processes::ProcessesPartialBinder;
use app_contracts::features::processes::{UiProcessesPort, UiProcessesPortMsg};
use app_contracts::features::processes::{ProcessesBinder, UiProcessesBindings};
use forsl_core::actor::ManagedActor;
use forsl_core::actor::event_bus::EventBus;
use forsl_core::actor::{Context, Handler, Message, NoOp};
use context::page_status::{PageStatus, RouteStatusChanged, RouteStatusRegistry};
use forsl::feature::{Events, FeatureComponent, FeatureContextState};
use forsl::uri::AppUri;
use forsl_macros::handler;
use macros::actor_manifest;
use slint::SharedString;
use std::borrow::Cow;
use std::sync::Arc;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tracing::{info, instrument};

pub struct ProcessActor<P: UiProcessesPort> {
    pub table: ProcessTable,
    pub metadata: ProcessMetadataService,
    pub route_status: Arc<RouteStatusRegistry>,
    pub is_active: bool,
    pub active_context_key: Cow<'static, str>,
    pub is_grouped: bool,
    pub ui_port: P,
    pub has_snapshot_data: bool,
    pub ctx: FeatureContextState,
}

#[actor_manifest(binder = ProcessesBinder)]
impl<P: UiProcessesPort> ManagedActor for ProcessActor<P> {
    type Bus = Events<
        bus!(
            WslAgentRuntimeEvent,
            #[cfg(target_os = "windows")]
            WindowsAgentRuntimeEvent,
        ),
    >;
    type Handlers = handlers!(
        @WslAgentRuntimeEvent,
        @WindowsAgentRuntimeEvent,
        bind {
            GroupClicked,
            SortBy(SharedString),
            ToggleExpandGroup(SharedString),
            RowsViewportChanged {
                start: i32,
                count: i32
            },
            SelectProcess {
                pid: i32,
                idx: i32
            },
            Terminate,
            ColumnResized {
                id: SharedString,
                width: f32
            }
        },
    );
    type Signals = bus!(ActiveStatus);
}

impl<P: UiProcessesPort> FeatureComponent for ProcessActor<P> {
    fn context_state(&mut self) -> &mut FeatureContextState {
        &mut self.ctx
    }

    fn on_activated(&mut self, uri: &AppUri, ctx: &Context<Self>) {
        self.is_active = true;
        ctx.publish(ActiveStatus(true));
        self.active_context_key = uri.context_name.clone();
    }

    fn on_deactivated(&mut self, _: &AppUri, ctx: &Context<Self>) {
        self.is_active = false;
        ctx.publish(ActiveStatus(false));
    }
}

impl<P: UiProcessesPort> ProcessActor<P> {
    fn push_batch(&self) {
        let batch = self.table.batch();
        self.ui_port.send(UiProcessesPortMsg::SetProcessRowsWindow {
            total_rows: batch.total_rows,
            start: batch.start,
            rows: batch.rows.to_vec(),
        });
    }

    fn set_empty_state(&self, visible: bool, title: &str, message: &str) {
        self.ui_port
            .send(UiProcessesPortMsg::SetEmptyStateVisible(visible));
        self.ui_port
            .send(UiProcessesPortMsg::SetEmptyStateTitle(title.into()));
        self.ui_port
            .send(UiProcessesPortMsg::SetEmptyStateMessage(message.into()));
    }

    fn set_agent_waiting_state(&self) {
        self.set_empty_state(
            true,
            "Waiting For Process Data",
            "The process list will appear after the agent connects and sends its first snapshot.",
        );
    }
}

#[handler]
fn process_snapshot_ready<P: UiProcessesPort>(
    this: &mut ProcessActor<P>,
    msg: ProcessSnapshotReady,
) {
    let processes = msg.processes.lock().unwrap().clone();
    this.has_snapshot_data = msg.total_count > 0;

    let snapshot = BridgeSnapshot {
        column_defs: msg.column_defs,
        processes,
    };

    let _ = this.table.handle_snapshot(snapshot, &mut this.metadata);

    this.ui_port
        .send(UiProcessesPortMsg::SetColumnDefs(this.table.get_header_columns()));
    this.ui_port
        .send(UiProcessesPortMsg::SetColumnWidths(this.table.column_widths()));
    this.ui_port
        .send(UiProcessesPortMsg::SetColumnMetadata(this.table.column_metadata()));
    this.ui_port
        .send(UiProcessesPortMsg::SetTotalProcessesCount(msg.total_count));

    if msg.total_count == 0 {
        this.set_empty_state(
            true,
            "No Processes Available",
            "The page is active, but the current data source returned an empty process snapshot.",
        );
    } else {
        this.set_empty_state(false, "", "");
    }

    this.route_status.report_route(RouteStatusChanged {
        context_key: this.active_context_key.to_string(),
        route_segment: "processes".into(),
        status: PageStatus::Ready,
        error: None,
    });

    this.push_batch();
}

#[handler]
fn sync_wsl_agent_status<P: UiProcessesPort>(
    this: &ProcessActor<P>,
    msg: WslAgentRuntimeEvent,
) {
    if this.has_snapshot_data {
        return;
    }

    match msg.state {
        AgentConnectionState::Connected => this.set_empty_state(
            true,
            "Waiting For First Snapshot",
            "The WSL agent is connected. Waiting for it to publish the first process report.",
        ),
        AgentConnectionState::Connecting | AgentConnectionState::WaitingRetry { .. } => this
            .set_empty_state(
                true,
                "Connecting To WSL Agent",
                "Process data is unavailable until the WSL agent connection is established.",
            ),
        AgentConnectionState::Disconnected => this.set_agent_waiting_state(),
    }
}

#[cfg(target_os = "windows")]
#[handler]
fn sync_windows_agent_status<P: UiProcessesPort>(
    this: &ProcessActor<P>,
    msg: WindowsAgentRuntimeEvent,
) {
    if this.has_snapshot_data {
        return;
    }

    match msg.state {
        AgentConnectionState::Connected => this.set_empty_state(
            true,
            "Waiting For First Snapshot",
            "The Windows agent is connected. Waiting for it to publish the first process report.",
        ),
        AgentConnectionState::Connecting | AgentConnectionState::WaitingRetry { .. } => this
            .set_empty_state(
                true,
                "Connecting To Windows Agent",
                "Process data is unavailable until the Windows agent connection is established.",
            ),
        AgentConnectionState::Disconnected => this.set_agent_waiting_state(),
    }
}

#[handler]
fn sort_table<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: SortBy) {
    this.table.toggle_sort(msg.0.clone());
    let sort = this.table.sort_state();
    this.ui_port.send(UiProcessesPortMsg::SetSortState {
        field: msg.0,
        descending: sort.descending,
    });
    this.table.refresh(&mut this.metadata).ok();
    this.push_batch();
}

#[handler]
fn toggle_process_expand<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: ToggleExpandGroup) {
    this.table.toggle_expand(msg.0);
    this.table.refresh(&mut this.metadata).ok();
    this.push_batch();
}

#[handler]
fn change_viewport<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: RowsViewportChanged) {
    this.table
        .set_viewport(msg.start as usize, msg.count.max(1) as usize);
    this.push_batch();
}

#[handler]
fn select_process<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: SelectProcess) {
    this.table.select(msg.pid as u32, msg.idx as usize);
    this.ui_port.send(UiProcessesPortMsg::SetSelectedPid(msg.pid));
    if let Some(name) = this.table.selected_name_for_pid(msg.pid as u32) {
        this.ui_port.send(UiProcessesPortMsg::SetSelectedName(name));
    }
}

#[handler]
fn terminate_selected_process<P: UiProcessesPort>(this: &mut ProcessActor<P>, _: Terminate) {
    let pid = this.ui_port.get_selected_pid();
    let Some(pid) = (pid != -1).then_some(pid as u32) else {
        return;
    };

    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), false);
    if let Some(process) = system.process(Pid::from_u32(pid)) {
        process.kill();
    }

    this.table.clear_selection();
}

#[handler]
fn resize_process_column<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: ColumnResized) {
    if let Err(e) = this
        .table
        .resize_column(msg.id.to_string(), msg.width as u64)
    {
        tracing::warn!("resize_column failed: {e}");
        return;
    }
    this.ui_port
        .send(UiProcessesPortMsg::SetColumnWidths(this.table.column_widths()));
}

#[handler]
fn toggle_grouping<P: UiProcessesPort>(this: &mut ProcessActor<P>, _msg: GroupClicked) {
    info!("clicked");
    this.is_grouped = !this.is_grouped;
    this.ui_port
        .send(UiProcessesPortMsg::SetIsGrouped(this.is_grouped));
}
