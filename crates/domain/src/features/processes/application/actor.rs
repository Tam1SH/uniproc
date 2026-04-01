use crate::features::processes::domain::table::ProcessTable;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::processes_impl::application::process_snapshot_actor::ProcessSnapshotReady;
use crate::processes_impl::domain::snapshot::BridgeSnapshot;
use app_contracts::features::navigation::{tab_ids, PageActivated};
use app_contracts::features::processes::ProcessesUiPort;
use app_core::actor::traits::{Context, Handler, Message, NoOp};
use app_core::app::Window;
use app_core::messages;
use context::page_status::{PageId, PageStatus, PageStatusChanged, PageStatusRegistry};
use context::settings::SettingSubscription;
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

pub struct ProcessActor<P: ProcessesUiPort> {
    pub page_id: PageId,
    pub table: ProcessTable,
    pub metadata: ProcessMetadataService,
    pub page_status: Arc<PageStatusRegistry>,
    pub is_active: bool,
    pub ui_port: P,
    #[allow(unused)]
    pub subs: Vec<SettingSubscription>,
}

impl<P: ProcessesUiPort> ProcessActor<P> {
    fn push_batch(&self) {
        let batch = self.table.batch();
        self.ui_port
            .set_process_rows_window(batch.total_rows, batch.start, batch.rows);
    }
}

impl<P, TWindow> Handler<ProcessSnapshotReady, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    #[instrument(name = "process-actor", level="trace", skip(self, _ctx, msg), fields(count = msg.total_count))]
    fn handle(&mut self, msg: ProcessSnapshotReady, _ctx: &Context<Self, TWindow>) {
        let processes = msg.processes.lock().unwrap().clone();

        let snapshot = BridgeSnapshot {
            column_defs: msg.column_defs,
            processes,
        };

        let _ = self.table.handle_snapshot(snapshot, &mut self.metadata);

        self.ui_port
            .set_column_defs(self.table.get_header_columns());
        self.ui_port.set_loading(false);
        self.ui_port.set_column_widths(self.table.column_widths());
        self.ui_port
            .set_column_metadata(self.table.column_metadata());
        self.ui_port.set_total_processes_count(msg.total_count);

        self.page_status.report_page(PageStatusChanged {
            tab_id: tab_ids::MAIN, // TODO no only main, i think
            page_id: self.page_id,
            status: PageStatus::Ready,
            error: None,
        });

        self.push_batch();
    }
}

impl<P, TWindow> Handler<PageActivated, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: PageActivated, _ctx: &Context<Self, TWindow>) {
        self.is_active = msg.page_id == self.page_id;
    }
}

impl<P, TWindow> Handler<Sort, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: Sort, _ctx: &Context<Self, TWindow>) {
        self.table.toggle_sort(msg.0.clone());
        let sort = self.table.sort_state();
        self.ui_port.set_sort_state(msg.0, sort.descending);
        self.table.refresh(&mut self.metadata).ok();
        self.push_batch();
    }
}

impl<P, TWindow> Handler<ToggleExpand, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: ToggleExpand, _ctx: &Context<Self, TWindow>) {
        self.table.toggle_expand(msg.0);
        self.table.refresh(&mut self.metadata).ok();
        self.push_batch();
    }
}

impl<P, TWindow> Handler<ViewportChanged, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: ViewportChanged, _ctx: &Context<Self, TWindow>) {
        self.table.set_viewport(msg.start, msg.count.max(1));
        self.push_batch();
    }
}

impl<P, TWindow> Handler<Select, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: Select, _ctx: &Context<Self, TWindow>) {
        self.table.select(msg.pid, msg.idx);
        self.ui_port.set_selected_pid(msg.pid as i32);
        if let Some(name) = self.table.selected_name_for_pid(msg.pid) {
            self.ui_port.set_selected_name(name);
        }
    }
}

impl<P, TWindow> Handler<TerminateSelected, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, _: TerminateSelected, ctx: &Context<Self, TWindow>) {
        let pid = self.ui_port.get_selected_pid();
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

        self.table.clear_selection();
    }
}

impl<P, TWindow> Handler<ResizeColumn, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: ResizeColumn, _ctx: &Context<Self, TWindow>) {
        if let Err(e) = self.table.resize_column(msg.id, msg.width as u64) {
            tracing::warn!("resize_column failed: {e}");
            return;
        }
        self.ui_port.set_column_widths(self.table.column_widths());
    }
}

impl<P, TWindow> Handler<GroupClicked, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, _msg: GroupClicked, _ctx: &Context<Self, TWindow>) {
        info!("clicked");
        static mut LOL: bool = true;
        unsafe {
            LOL = !LOL;
        }
        unsafe {
            self.ui_port.set_is_grouped(LOL);
        }
    }
}
