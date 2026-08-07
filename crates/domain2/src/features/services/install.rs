use app_contracts2::features::agents::ScanTick;
use app_contracts2::features::services::{ServicesBinder, ServicesReducer};
use guinea::feature::FeatureInitContext;
use guinea::reactor::Reactor;
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::signal::Signal;

use super::actor::{Command, Deselect, Select, ServicesActor, Sort};
use super::settings::ServicesSettings;

const MIN_SCAN_INTERVAL_MS: u64 = 100;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let settings = ServicesSettings::new_with(&ctx.store)?;

    let interval = Signal::new(settings.scan_interval_ms().get().max(MIN_SCAN_INTERVAL_MS));
    let heartbeat = Reactor::new().add_heartbeat(interval, || {
        GlobalEventBus::publish(ScanTick);
    });

    ctx.scope.own(heartbeat);

    let addr = ctx.spawn_actor(ServicesActor::new(ctx.port::<ServicesReducer>()));

    ctx.subscribe_on_global_bus::<ServicesActor<_>, ScanTick>(addr.clone());

    ServicesBinder::new(&addr, &ctx.actions::<ServicesReducer>())
        .on_sort::<Sort>()
        .on_select::<Select>()
        .on_deselect::<Deselect>()
        .on_command::<Command>()
        .build();

    // First scan immediately; the heartbeat re-arms from here on.
    GlobalEventBus::publish(ScanTick);

    Ok(())
}
