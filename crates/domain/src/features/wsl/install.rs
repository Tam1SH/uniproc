use app_contracts::features::agents::RemoteScanResult;
use app_contracts::features::wsl::WslReducer;
use guinea::feature::FeatureInitContext;
use guinea::reactor::Reactor;

use super::actor::{RefreshDistros, WslActor};
use crate::features::agents::settings::AgentSettings;

const DISTRO_SCAN_INTERVAL_MS: u64 = 3000;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let settings = AgentSettings::new()?;
    let configured = settings.wsl_distro().get();

    let addr = ctx.spawn_actor(WslActor::new(ctx.port::<WslReducer>(), configured));

    let ticker = addr.clone();
    let heartbeat = Reactor::new().add_heartbeat(
        || DISTRO_SCAN_INTERVAL_MS,
        move || {
            ticker.send(RefreshDistros);
        },
    );
    ctx.scope.own(heartbeat);

    ctx.subscribe_on_global_bus::<WslActor<_>, RemoteScanResult>(addr.clone());

    addr.send(RefreshDistros);

    Ok(())
}
