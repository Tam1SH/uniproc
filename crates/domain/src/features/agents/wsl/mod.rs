mod actor;
mod agent;

use actor::{Init, Ping, WslAgentActor};
use app_contracts::features::agents::ScanTick;
use app_core::SharedState;
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EVENT_BUS;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use app_core::settings::{FeatureSettings, SettingsScope, SettingsStore, settings_from};
use slint::ComponentHandle;
use std::time::Duration;

const CONNECT_TIMEOUT_SECS: &str = "connect_timeout_secs";
const PING_INTERVAL_MS: &str = "ping_interval_ms";

struct WslAgentSettings;

impl SettingsScope for WslAgentSettings {
    const PREFIX: &'static str = "wsl_agent";
}

impl FeatureSettings for WslAgentSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, CONNECT_TIMEOUT_SECS, 8u64)?;
        Self::ensure_default(settings, PING_INTERVAL_MS, 2000u64)?;
        Ok(())
    }
}

pub struct WslAgentFeature;

impl<TWindow: ComponentHandle + 'static> Feature<TWindow> for WslAgentFeature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = settings_from(shared);
        WslAgentSettings::ensure_defaults(&settings)?;

        let connect_timeout_secs = WslAgentSettings::setting_or(&settings, CONNECT_TIMEOUT_SECS, 8u64)?;
        let ping_interval_ms = WslAgentSettings::setting_or(&settings, PING_INTERVAL_MS, 2000u64)?;

        let addr = Addr::new(WslAgentActor::new(connect_timeout_secs), ui.as_weak());

        let a = addr.clone();
        reactor.add_dynamic_loop(
            move || Duration::from_millis(ping_interval_ms.get().max(1)),
            move || {
            a.send(Ping)
            },
        );

        EVENT_BUS.with(|bus| {
            bus.subscribe::<WslAgentActor, ScanTick, TWindow>(addr.clone());
        });

        addr.send(Init);
        Ok(())
    }
}
