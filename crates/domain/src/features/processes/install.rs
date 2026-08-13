use app_contracts::features::agents::{ScanTick, WindowsReportMessage};
use app_contracts::features::processes::{ProcessesBinder, ProcessesReducer};
use guinea::feature::FeatureInitContext;
use guinea::reactor::Reactor;
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::signal::Signal;

use super::actor::{Deselect, ProcessesActor, Select, Sort, Terminate};
use super::settings::ProcessesSettings;

const MIN_SCAN_INTERVAL_MS: u64 = 100;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let settings = ProcessesSettings::new_with(&ctx.store)?;

    let interval = Signal::new(settings.scan_interval_ms().get().max(MIN_SCAN_INTERVAL_MS));
    let heartbeat = Reactor::new().add_heartbeat(interval, || {
        GlobalEventBus::publish(ScanTick);
    });

    ctx.scope.own(heartbeat);

    let addr = ctx.spawn_actor(ProcessesActor::new(ctx.port::<ProcessesReducer>()));

    ctx.subscribe_on_global_bus::<ProcessesActor<_>, WindowsReportMessage>(addr.clone());

    ProcessesBinder::new(&addr, &ctx.actions::<ProcessesReducer>())
        .on_sort::<Sort>()
        .on_select::<Select>()
        .on_deselect::<Deselect>()
        .on_terminate::<Terminate>()
        .build();

    GlobalEventBus::publish(ScanTick);

    Ok(())
}
