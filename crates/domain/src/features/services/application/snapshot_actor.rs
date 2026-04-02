use crate::features::services::{application::actor::ServiceActor, scanner};
use app_contracts::features::agents::ScanTick;
use app_contracts::features::navigation::PageActivated;
use app_contracts::features::services::{ServiceEntryDto, ServiceSnapshot, ServicesUiPort};
use app_core::actor::addr::Addr;
use app_core::actor::traits::{Context, Handler, Message, NoOp};
use app_core::app::Window;
use app_core::messages;
use context::page_status::PageId;

messages! {
    ServiceSnapshotReady(ServiceSnapshotResult)
}
#[derive(Clone, Debug)]
pub enum ServiceSnapshotResult {
    NoOp(NoOp),
    Snapshot(Vec<ServiceEntryDto>),
}
impl Message for ServiceSnapshotResult {}

pub struct ServiceSnapshotActor<P: ServicesUiPort, TWindow: Window> {
    pub target: Addr<ServiceActor<P>, TWindow>,
    pub page_id: PageId,
    pub is_active: bool,
}

impl<P, TWindow> Handler<PageActivated, TWindow> for ServiceSnapshotActor<P, TWindow>
where
    P: ServicesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: PageActivated, _ctx: &Context<Self, TWindow>) {
        self.is_active = msg.page_id == self.page_id;
    }
}

impl<P, TWindow> Handler<ScanTick, TWindow> for ServiceSnapshotActor<P, TWindow>
where
    P: ServicesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, _: ScanTick, ctx: &Context<Self, TWindow>) {
        if !self.is_active {
            return;
        }

        ctx.spawn_bg(async move {
            #[cfg(target_os = "windows")]
            if let Ok(d) = scanner::windows::scan_services() {
                ServiceSnapshotResult::Snapshot(d)
            } else {
                ServiceSnapshotResult::NoOp(NoOp)
            }
        });
    }
}

impl<P, TWindow> Handler<ServiceSnapshotResult, TWindow> for ServiceSnapshotActor<P, TWindow>
where
    P: ServicesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, result: ServiceSnapshotResult, ctx: &Context<Self, TWindow>) {
        if let ServiceSnapshotResult::Snapshot(services) = result {
            self.target.send(ServiceSnapshot { services })
        }
    }
}
