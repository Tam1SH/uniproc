use macros::feature_settings;

#[feature_settings(prefix = "agents")]
pub struct AgentSettings {
    #[setting(default = 8u64)]
    pub connect_timeout_secs: u64,

    #[setting(default = 2000u64)]
    pub ping_interval_ms: u64,
}
