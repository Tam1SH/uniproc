use app_contracts::features::agents::{ScanTick, WindowsReportMessage};
use app_contracts::features::services::{ServicesReducer};
use guinea::feature::FeatureInitContext;
use guinea_core::actor::event_bus::GlobalEventBus;

use super::actor::{ServicesActor};

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let addr = ctx.spawn_actor(ServicesActor::new(ctx.port::<ServicesReducer>()));

    ctx.subscribe_on_global_bus::<ServicesActor<_>, WindowsReportMessage>(addr.clone());
    ctx.wire::<ServicesReducer, _>(&addr);

    GlobalEventBus::publish(ScanTick);

    Ok(())
}
