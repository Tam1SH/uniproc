use crate::features::services::{application::actor::ServiceActor, scanner};
use app_contracts::features::agents::ScanTick;
use app_contracts::features::services::{ServiceEntryDto, ServiceSnapshot, UiServicesPort};
use forsl_core::actor::{Addr, AsyncContext, ManagedActor};
use forsl_core::actor::{Message, NoOp};
use forsl_core::messages;
use forsl_macros::handler;
use macros::actor_manifest;

messages! {
    ServiceSnapshotReady(ServiceSnapshotResult)
}
#[derive(Clone, Debug)]
pub enum ServiceSnapshotResult {
    NoOp(NoOp),
    Snapshot(Vec<ServiceEntryDto>),
}
impl Message for ServiceSnapshotResult {}

#[actor_manifest]
impl<P: UiServicesPort> ManagedActor for ServiceSnapshotActor<P> {
    type Bus = bus!(ActiveStatus);
    type Handlers = handlers!(
        @ScanTick,
        ActiveStatus(bool),
    );
}

pub struct ServiceSnapshotActor<P: UiServicesPort> {
    pub target: Addr<ServiceActor<P>>,
    pub is_active: bool,
}

#[handler]
async fn handle_scan_tick<P: UiServicesPort>(
    ctx: AsyncContext<ServiceSnapshotActor<P>>,
    _: ScanTick,
) {
    let is_active = ctx.apply(|this, _| this.is_active).await;
    if !is_active {
        return;
    }

    let result = scanner::windows::scan_services();

    match result {
        Ok(data) => ctx.send(ServiceSnapshotResult::Snapshot(data)),
        Err(_) => ctx.send(ServiceSnapshotResult::NoOp(NoOp)),
    }
}

#[handler]
fn active_status<P: UiServicesPort>(this: &mut ServiceSnapshotActor<P>, msg: ActiveStatus) {
    this.is_active = msg.0;
}

#[handler]
fn on_snapshot_result<P: UiServicesPort>(
    this: &ServiceSnapshotActor<P>,
    result: ServiceSnapshotResult,
) {
    if let ServiceSnapshotResult::Snapshot(services) = result {
        this.target.send(ServiceSnapshot { services })
    }
}
