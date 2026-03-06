mod store;

use std::path::PathBuf;
use std::sync::Arc;

use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::shared::settings::{FeatureSettings, SettingsScope};
use crate::AppWindow;
use app_core::SharedState;

pub use store::SettingsStore;

const SAVE_DEBOUNCE_MS: &str = "save_debounce_ms";

struct SettingsPersistenceSettings;

impl SettingsScope for SettingsPersistenceSettings {
    const PREFIX: &'static str = "settings.persistence";
}

impl FeatureSettings for SettingsPersistenceSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, SAVE_DEBOUNCE_MS, 300u64)
    }
}

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

impl Feature for SettingsFeature {
    fn install(
        self,
        _reactor: &mut Reactor,
        _ui: &AppWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let path = self
            .path_override
            .unwrap_or_else(SettingsStore::default_settings_path);
        let store = Arc::new(SettingsStore::load_or_default(path)?);

        SettingsPersistenceSettings::ensure_defaults(&store)?;
        shared.insert_arc(Arc::clone(&store));

        Ok(())
    }
}

pub fn settings_from(shared: &SharedState) -> Arc<SettingsStore> {
    shared
        .get::<SettingsStore>()
        .expect("SettingsStore must be installed in SharedState before usage")
}
