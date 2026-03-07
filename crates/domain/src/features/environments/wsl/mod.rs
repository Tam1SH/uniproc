use crate::features::environments::wsl::actors::{Init, InstallAgent, Ping, WslActor};
use crate::features::settings::{SettingsStore, settings_from};
use crate::shared::settings::{FeatureSettings, SettingsScope};
use app_contracts::features::environments::{EnvironmentsUiBindings, EnvironmentsUiPort};
use app_core::SharedState;
use app_core::actor::addr::Addr;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;
use std::time::Duration;

mod actors;
mod agent;
mod domain;
mod state;

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

pub struct WslFeature<F> {
    make_ui_port: F,
}

impl<F> WslFeature<F> {
    pub fn new(make_ui_port: F) -> Self {
        Self { make_ui_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for WslFeature<F>
where
    TWindow: ComponentHandle + 'static,
    F: Fn(&TWindow) -> P + 'static,
    P: EnvironmentsUiPort + EnvironmentsUiBindings + Clone + 'static,
{
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = settings_from(shared);

        WslSettings::ensure_defaults(&settings)?;

        let ping_interval_ms = WslSettings::get_or(&settings, PING_INTERVAL_MS, 500u64).max(1);
        let connect_timeout_secs =
            WslSettings::get_or(&settings, CONNECT_TIMEOUT_SECS, 8u64).max(1);
        let ui_port = (self.make_ui_port)(ui);

        let addr = Addr::new(
            WslActor::new(connect_timeout_secs, ui_port.clone()),
            ui.as_weak(),
        );

        let a = addr.clone();
        reactor.add_loop(Duration::from_millis(ping_interval_ms), move || {
            a.send(Ping)
        });

        addr.send(Init);

        let a = addr.clone();
        ui_port.on_install_agent(move |distro| a.send(InstallAgent(distro)));

        Ok(())
    }
}
