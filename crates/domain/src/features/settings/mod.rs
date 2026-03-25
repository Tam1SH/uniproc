use std::path::PathBuf;

use app_core::SharedState;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;

pub use app_core::settings::*;

#[derive(Default)]
pub struct SettingsFeature {
    inner: app_core::settings::SettingsFeature,
}

impl SettingsFeature {
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            inner: app_core::settings::SettingsFeature::with_path(path),
        }
    }
}

impl<TWindow> Feature<TWindow> for SettingsFeature
where
    TWindow: ComponentHandle + 'static,
{
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        self.inner.install(reactor, ui, shared)
    }
}
