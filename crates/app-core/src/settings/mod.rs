mod store;

use std::path::PathBuf;
use std::sync::Arc;

use crate::SharedState;
use crate::app::Feature;
use crate::reactor::Reactor;
use serde::Serialize;
use serde::de::DeserializeOwned;
use slint::ComponentHandle;

pub use store::SettingEvent;
pub use store::SettingOp;
pub use store::SettingsCallback;
pub use store::SettingsStore;
pub use store::SubscriptionId;
pub use store::SubscriptionKind;

const SAVE_DEBOUNCE_MS: &str = "save_debounce_ms";

pub trait SettingsScope {
    const PREFIX: &'static str;
}

pub trait FeatureSettings: SettingsScope {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()>;

    fn ensure_default<TValue>(
        settings: &SettingsStore,
        key: &str,
        value: TValue,
    ) -> anyhow::Result<()>
    where
        Self: Sized,
        TValue: Serialize,
    {
        ensure_default::<Self, TValue>(settings, key, value)
    }

    fn get_or<TValue>(settings: &SettingsStore, key: &str, default: TValue) -> TValue
    where
        Self: Sized,
        TValue: Serialize + DeserializeOwned,
    {
        get_or::<Self, TValue>(settings, key, default)
    }
}

pub fn scoped_path<T: SettingsScope>(key: &str) -> String {
    if key.trim().is_empty() {
        return T::PREFIX.to_string();
    }

    format!(
        "{}.{}",
        T::PREFIX.trim_end_matches('.'),
        key.trim_start_matches('.')
    )
}

pub fn ensure_default<TScope, TValue>(
    settings: &SettingsStore,
    key: &str,
    value: TValue,
) -> anyhow::Result<()>
where
    TScope: SettingsScope,
    TValue: Serialize,
{
    let path = scoped_path::<TScope>(key);

    if settings.get(&path).is_none() {
        settings.set(&path, serde_json::to_value(value)?)?;
    }

    Ok(())
}

pub fn get_or<TScope, TValue>(settings: &SettingsStore, key: &str, default: TValue) -> TValue
where
    TScope: SettingsScope,
    TValue: Serialize + DeserializeOwned,
{
    let path = scoped_path::<TScope>(key);

    settings
        .get(&path)
        .and_then(|value| serde_json::from_value::<TValue>(value).ok())
        .unwrap_or(default)
}

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

impl<TWindow> Feature<TWindow> for SettingsFeature
where
    TWindow: ComponentHandle + 'static,
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
