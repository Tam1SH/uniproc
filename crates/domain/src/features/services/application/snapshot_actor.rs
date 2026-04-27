use crate::features::services::{application::actor::ServiceActor, scanner};
use app_contracts::features::agents::ScanTick;
use app_contracts::features::services::{ServiceEntryDto, ServiceSnapshot, UiServicesPort};
use app_core::actor::{Addr, Context, ManagedActor};
use app_core::actor::{Message, NoOp};
use app_core::messages;
use macros::{actor_manifest, handler};

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
fn handle_scan_tick<P: UiServicesPort>(
    this: &mut ServiceSnapshotActor<P>,
    _: ScanTick,
    ctx: &Context<ServiceSnapshotActor<P>>,
) {
    if !this.is_active {
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

#[handler]
fn active_status<P: UiServicesPort>(this: &mut ServiceSnapshotActor<P>, msg: ActiveStatus) {
    this.is_active = msg.0;
}

#[handler]
fn on_snapshot_result<P: UiServicesPort>(
    this: &mut ServiceSnapshotActor<P>,
    result: ServiceSnapshotResult,
) {
    if let ServiceSnapshotResult::Snapshot(services) = result {
        this.target.send(ServiceSnapshot { services })
    }
}
