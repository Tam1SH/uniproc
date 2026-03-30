use macros::feature_settings;

#[feature_settings(prefix = "settings.persistence")]
pub struct SettingsPersistenceSettings {
    #[setting(default = 300u64)]
    pub save_debounce_ms: u64,

    #[setting(default = 500u64)]
    pub watch_interval_ms: u64,
}
