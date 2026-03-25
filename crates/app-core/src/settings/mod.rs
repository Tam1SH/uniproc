mod store;

use arc_swap::ArcSwap;
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
const WATCH_INTERVAL_MS: &str = "watch_interval_ms";

struct SettingSubscription {
    settings: Arc<SettingsStore>,
    id: SubscriptionId,
}

impl Drop for SettingSubscription {
    fn drop(&mut self) {
        self.settings.unsubscribe(self.id);
    }
}

pub struct ReactiveSetting<TValue> {
    value: Arc<ArcSwap<TValue>>,
    _subscription: Arc<SettingSubscription>,
}

impl<TValue> Clone for ReactiveSetting<TValue> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            _subscription: Arc::clone(&self._subscription),
        }
    }
}

impl<TValue> ReactiveSetting<TValue> {
    pub fn get_arc(&self) -> Arc<TValue> {
        self.value.load_full()
    }
}

impl<TValue: Clone> ReactiveSetting<TValue> {
    pub fn get(&self) -> TValue {
        self.value.load().as_ref().clone()
    }
}

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

    fn setting_or<TValue>(
        settings: &Arc<SettingsStore>,
        key: &str,
        default: TValue,
    ) -> anyhow::Result<ReactiveSetting<TValue>>
    where
        Self: Sized,
        TValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    {
        setting_or::<Self, TValue>(settings, key, default)
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

fn read_value_or_default<TValue>(settings: &SettingsStore, path: &str, default: &TValue) -> TValue
where
    TValue: DeserializeOwned + Clone,
{
    settings
        .get(path)
        .and_then(|value| serde_json::from_value::<TValue>(value).ok())
        .unwrap_or_else(|| default.clone())
}

pub fn setting_or<TScope, TValue>(
    settings: &Arc<SettingsStore>,
    key: &str,
    default: TValue,
) -> anyhow::Result<ReactiveSetting<TValue>>
where
    TScope: SettingsScope,
    TValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    ensure_default::<TScope, TValue>(settings, key, default.clone())?;
    let path = scoped_path::<TScope>(key);
    let current = read_value_or_default(settings, &path, &default);

    let value = Arc::new(ArcSwap::from_pointee(current));
    let watched = Arc::clone(&value);
    let watched_path = path.clone();
    let watched_default = default.clone();
    let watched_settings = Arc::clone(settings);

    let id = settings.on_state_changed(Arc::new(move |_| {
        let next = read_value_or_default(&watched_settings, &watched_path, &watched_default);
        watched.store(Arc::new(next));
    }));

    Ok(ReactiveSetting {
        value,
        _subscription: Arc::new(SettingSubscription {
            settings: Arc::clone(settings),
            id,
        }),
    })
}

struct SettingsPersistenceSettings;

impl SettingsScope for SettingsPersistenceSettings {
    const PREFIX: &'static str = "settings.persistence";
}

impl FeatureSettings for SettingsPersistenceSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, SAVE_DEBOUNCE_MS, 300u64)?;
        Self::ensure_default(settings, WATCH_INTERVAL_MS, 500u64)?;
        Ok(())
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
