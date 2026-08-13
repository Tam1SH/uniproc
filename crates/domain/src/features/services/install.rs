use app_contracts::features::agents::{ScanTick, WindowsReportMessage};
use app_contracts::features::services::{ServicesBinder, ServicesReducer};
use guinea::feature::FeatureInitContext;
use guinea_core::actor::event_bus::GlobalEventBus;

use super::actor::{Command, Deselect, Select, ServicesActor, Sort};

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let addr = ctx.spawn_actor(ServicesActor::new(ctx.port::<ServicesReducer>()));

    ctx.subscribe_on_global_bus::<ServicesActor<_>, WindowsReportMessage>(addr.clone());

    ServicesBinder::new(&addr, &ctx.actions::<ServicesReducer>())
        .on_sort::<Sort>()
        .on_select::<Select>()
        .on_deselect::<Deselect>()
        .on_command::<Command>()
        .build();

    GlobalEventBus::publish(ScanTick);

    Ok(())
}
