mod settings;

use crate::features::settings::settings::SettingsPersistenceSettings;
use framework::feature::{AppFeature, AppFeatureInitContext};
pub use framework::settings::*;
use std::path::PathBuf;
use std::sync::Arc;

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

impl AppFeature for SettingsFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let path = self
            .path_override
            .map(Ok)
            .unwrap_or_else(SettingsStore::default_settings_path)?;
        let store = Arc::new(SettingsStore::load_or_default(path)?);

        ctx.shared.insert_arc(Arc::clone(&store));

        let _ = SettingsPersistenceSettings::new(ctx.shared)?;

        Ok(())
    }
}
