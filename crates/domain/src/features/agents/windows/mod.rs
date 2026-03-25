mod actor;
mod agent;

pub use actor::{Init, Ping, WindowsAgentActor};

use app_core::SharedState;
use app_core::actor::addr::Addr;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use app_core::settings::{FeatureSettings, SettingsScope, SettingsStore, settings_from};
use slint::ComponentHandle;
use std::time::Duration;

const CONNECT_TIMEOUT_SECS: &str = "connect_timeout_secs";
const PING_INTERVAL_MS: &str = "ping_interval_ms";

struct WindowsAgentSettings;

impl SettingsScope for WindowsAgentSettings {
    const PREFIX: &'static str = "windows_agent";
}

impl FeatureSettings for WindowsAgentSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, CONNECT_TIMEOUT_SECS, 8u64)?;
        Self::ensure_default(settings, PING_INTERVAL_MS, 500u64)?;
        Ok(())
    }
}

pub struct WindowsAgentFeature;

impl<TWindow: ComponentHandle + 'static> Feature<TWindow> for WindowsAgentFeature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = settings_from(shared);
        WindowsAgentSettings::ensure_defaults(&settings)?;

        let connect_timeout_secs =
            WindowsAgentSettings::get_or(&settings, CONNECT_TIMEOUT_SECS, 8u64).max(1);
        let ping_interval_ms =
            WindowsAgentSettings::get_or(&settings, PING_INTERVAL_MS, 500u64).max(1);

        let addr = Addr::new(WindowsAgentActor::new(connect_timeout_secs), ui.as_weak());

        let a = addr.clone();
        reactor.add_loop(Duration::from_millis(ping_interval_ms), move || {
            a.send(Ping)
        });

        addr.send(Init);
        Ok(())
    }
}
