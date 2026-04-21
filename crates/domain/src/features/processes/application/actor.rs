use crate::features::processes::domain::table::ProcessTable;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::processes_impl::application::process_snapshot_actor::ProcessSnapshotReady;
use crate::processes_impl::domain::snapshot::BridgeSnapshot;
#[cfg(target_os = "windows")]
use app_contracts::features::environments::WindowsAgentRuntimeEvent;
use app_contracts::features::environments::{AgentConnectionState, WslAgentRuntimeEvent};
use app_contracts::features::navigation::{PageActivated, tab_ids};
use app_contracts::features::processes::UiProcessesPort;
use app_core::actor::traits::{Context, NoOp};
use app_core::messages;
use context::page_status::{PageId, PageStatus, PageStatusChanged, PageStatusRegistry};
use context::settings::SettingSubscription;
use macros::handler;
use slint::SharedString;
use std::sync::Arc;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tracing::{info, instrument};

messages! {
    Sort(SharedString),
    ToggleExpand(SharedString),
    ViewportChanged { start: usize, count: usize },
    Select { pid: u32, idx: usize },
    TerminateSelected,
    ResizeColumn { id: String, width: f32 },
    GroupClicked,
}

pub struct ProcessActor<P: UiProcessesPort> {
    pub page_id: PageId,
    pub table: ProcessTable,
    pub metadata: ProcessMetadataService,
    pub page_status: Arc<PageStatusRegistry>,
    pub is_active: bool,
    pub is_grouped: bool,
    pub ui_port: P,
    pub has_snapshot_data: bool,
    #[allow(unused)]
    pub subs: Vec<SettingSubscription>,
}

impl<P: UiProcessesPort> ProcessActor<P> {
    fn push_batch(&self) {
        let batch = self.table.batch();
        self.ui_port
            .set_process_rows_window(batch.total_rows, batch.start, batch.rows);
    }

    fn set_empty_state(&self, visible: bool, title: &str, message: &str) {
        self.ui_port.set_empty_state_visible(visible);
        self.ui_port.set_empty_state_title(title.into());
        self.ui_port.set_empty_state_message(message.into());
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
#[instrument(name = "process-actor", level = "trace", skip(this, msg), fields(count = msg.total_count))]
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
        .set_column_defs(this.table.get_header_columns());
    this.ui_port.set_column_widths(this.table.column_widths());
    this.ui_port
        .set_column_metadata(this.table.column_metadata());
    this.ui_port.set_total_processes_count(msg.total_count);

    if msg.total_count == 0 {
        this.set_empty_state(
            true,
            "No Processes Available",
            "The page is active, but the current data source returned an empty process snapshot.",
        );
    } else {
        this.set_empty_state(false, "", "");
    }

    this.page_status.report_page(PageStatusChanged {
        tab_id: tab_ids::MAIN,
        page_id: this.page_id,
        status: PageStatus::Ready,
        error: None,
    });

    this.push_batch();
}

#[handler]
fn activate_page<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: PageActivated) {
    this.is_active = msg.page_id == this.page_id;
}

#[handler]
fn sync_wsl_agent_status<P: UiProcessesPort>(
    this: &mut ProcessActor<P>,
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
    this: &mut ProcessActor<P>,
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
fn sort_table<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: Sort) {
    this.table.toggle_sort(msg.0.clone());
    let sort = this.table.sort_state();
    this.ui_port.set_sort_state(msg.0, sort.descending);
    this.table.refresh(&mut this.metadata).ok();
    this.push_batch();
}

#[handler]
fn toggle_process_expand<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: ToggleExpand) {
    this.table.toggle_expand(msg.0);
    this.table.refresh(&mut this.metadata).ok();
    this.push_batch();
}

#[handler]
fn change_viewport<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: ViewportChanged) {
    this.table.set_viewport(msg.start, msg.count.max(1));
    this.push_batch();
}

#[handler]
fn select_process<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: Select) {
    this.table.select(msg.pid, msg.idx);
    this.ui_port.set_selected_pid(msg.pid as i32);
    if let Some(name) = this.table.selected_name_for_pid(msg.pid) {
        this.ui_port.set_selected_name(name);
    }
}

#[handler]
fn terminate_selected_process<P: UiProcessesPort>(
    this: &mut ProcessActor<P>,
    _: TerminateSelected,
    ctx: &Context<ProcessActor<P>>,
) {
    let pid = this.ui_port.get_selected_pid();
    let Some(pid) = (pid != -1).then_some(pid as u32) else {
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

    this.table.clear_selection();
}

#[handler]
fn resize_process_column<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: ResizeColumn) {
    if let Err(e) = this.table.resize_column(msg.id, msg.width as u64) {
        tracing::warn!("resize_column failed: {e}");
        return;
    }
    this.ui_port.set_column_widths(this.table.column_widths());
}

#[handler]
fn toggle_grouping<P: UiProcessesPort>(this: &mut ProcessActor<P>, _msg: GroupClicked) {
    info!("clicked");
    this.is_grouped = !this.is_grouped;
    this.ui_port.set_is_grouped(this.is_grouped);
}
