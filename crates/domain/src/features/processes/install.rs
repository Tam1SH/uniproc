use app_contracts::features::agents::{ScanTick, WindowsReportMessage};
use app_contracts::features::processes::ProcessesReducer;
use guinea::feature::FeatureInitContext;
use guinea_core::actor::event_bus::GlobalEventBus;

use super::actor::ProcessesActor;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let addr = ctx.spawn_actor(ProcessesActor::new(ctx.port::<ProcessesReducer>()));

    ctx.subscribe_on_global_bus::<ProcessesActor<_>, WindowsReportMessage>(addr.clone());
    ctx.wire::<ProcessesReducer, _>(&addr);

    GlobalEventBus::publish(ScanTick);

    Ok(())
}
