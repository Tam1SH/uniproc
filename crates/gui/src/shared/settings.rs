use crate::features::settings::SettingsStore;
use serde::Serialize;
use serde::de::DeserializeOwned;

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
