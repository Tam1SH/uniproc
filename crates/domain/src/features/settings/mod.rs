mod settings;

use app_core::SharedState;
use app_core::app::Feature;
use app_core::app::Window;
use app_core::reactor::Reactor;
use std::path::PathBuf;
use std::sync::Arc;

use crate::features::settings::settings::SettingsPersistenceSettings;
pub use app_core::settings::*;

#[derive(Default)]
pub struct SettingsFeature {
    path_override: Option<PathBuf>,
}

impl SettingsFeature {
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            path_override: Some(path),
        }
    }
}

impl<TWindow> Feature<TWindow> for SettingsFeature
where
    TWindow: Window,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        _ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let path = self
            .path_override
            .unwrap_or_else(SettingsStore::default_settings_path);
        let store = Arc::new(SettingsStore::load_or_default(path)?);

        shared.insert_arc(Arc::clone(&store));

        let _ = SettingsPersistenceSettings::new(shared)?;

        Ok(())
    }
}
