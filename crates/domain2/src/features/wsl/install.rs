use app_contracts2::features::agents::RemoteScanResult;
use app_contracts2::features::wsl::{WslBinder, WslReducer};
use guinea::feature::FeatureInitContext;
use guinea::reactor::Reactor;
use guinea_core::signal::Signal;

use super::actor::{RefreshDistros, WslActor};
use crate::features::agents::settings::AgentSettings;

const DISTRO_SCAN_INTERVAL_MS: u64 = 3000;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let settings = AgentSettings::new_with(&ctx.store)?;
    let configured = settings.wsl_distro().get();

    let addr = ctx.spawn_actor(WslActor::new(ctx.port::<WslReducer>(), configured));

    let ticker = addr.clone();
    let heartbeat = Reactor::new().add_heartbeat(Signal::new(DISTRO_SCAN_INTERVAL_MS), move || {
        ticker.send(RefreshDistros);
    });
    ctx.scope.own(heartbeat);

    ctx.subscribe_on_global_bus::<WslActor<_>, RemoteScanResult>(addr.clone());

    addr.send(RefreshDistros);

    WslBinder::new(&addr, &ctx.actions::<WslReducer>()).build();

    Ok(())
}
