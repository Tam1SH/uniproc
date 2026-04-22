use crate::features::services::{application::actor::ServiceActor, scanner};
use app_contracts::features::agents::ScanTick;
use app_contracts::features::navigation::RouteActivated;
use app_contracts::features::services::{ServiceEntryDto, ServiceSnapshot, UiServicesPort};
use app_core::actor::addr::Addr;
use app_core::actor::traits::{Context, Handler, Message, NoOp};
use app_core::messages;

messages! {
    ServiceSnapshotReady(ServiceSnapshotResult)
}
#[derive(Clone, Debug)]
pub enum ServiceSnapshotResult {
    NoOp(NoOp),
    Snapshot(Vec<ServiceEntryDto>),
}
impl Message for ServiceSnapshotResult {}

pub struct ServiceSnapshotActor<P: UiServicesPort> {
    pub target: Addr<ServiceActor<P>>,
    pub is_active: bool,
}

impl<P> Handler<RouteActivated> for ServiceSnapshotActor<P>
where
    P: UiServicesPort,
{
    fn handle(&mut self, msg: RouteActivated, _ctx: &Context<Self>) {
        self.is_active = msg.route_segment == "services";
    }
}

impl<P> Handler<ScanTick> for ServiceSnapshotActor<P>
where
    P: UiServicesPort,
{
    fn handle(&mut self, _: ScanTick, ctx: &Context<Self>) {
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

impl<P> Handler<ServiceSnapshotResult> for ServiceSnapshotActor<P>
where
    P: UiServicesPort,
{
    fn handle(&mut self, result: ServiceSnapshotResult, _ctx: &Context<Self>) {
        if let ServiceSnapshotResult::Snapshot(services) = result {
            self.target.send(ServiceSnapshot { services })
        }
    }
}
