use crate::core::actor::addr::Addr;
use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::features::envs::wsl::actors::{Init, InstallAgent, Ping, WslActor};
use crate::features::settings::{SettingsStore, settings_from};
use crate::shared::settings::{FeatureSettings, SettingsScope};
use crate::{AppWindow, EnvironmentsFeatureGlobal};
use app_core::SharedState;
use slint::ComponentHandle;
use std::time::Duration;

mod actors;
mod agent;
mod domain;
mod state;

pub use actors::WslAgentRuntimeEvent;
pub use agent::WslClient;

const CONNECT_TIMEOUT_SECS: &str = "connect_timeout_secs";
const PING_INTERVAL_MS: &str = "ping_interval_ms";

struct WslSettings;

impl SettingsScope for WslSettings {
    const PREFIX: &'static str = "wsl";
}

impl FeatureSettings for WslSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, CONNECT_TIMEOUT_SECS, 8u64)?;
        Self::ensure_default(settings, PING_INTERVAL_MS, 500u64)?;
        Ok(())
    }
}

pub struct WslFeature;

impl Feature for WslFeature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &AppWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = settings_from(shared);

        WslSettings::ensure_defaults(&settings)?;

        let ping_interval_ms = WslSettings::get_or(&settings, PING_INTERVAL_MS, 500u64).max(1);
        let connect_timeout_secs =
            WslSettings::get_or(&settings, CONNECT_TIMEOUT_SECS, 8u64).max(1);

        let addr = Addr::new(WslActor::new(connect_timeout_secs), ui.as_weak());

        let a = addr.clone();
        reactor.add_loop(Duration::from_millis(ping_interval_ms), move || {
            a.send(Ping)
        });

        addr.send(Init);

        let global = ui.global::<EnvironmentsFeatureGlobal>();
        global.on_install_agent(addr.handler_with(InstallAgent));

        Ok(())
    }
}
