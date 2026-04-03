use macros::feature_settings;

#[feature_settings(prefix = "trace")]
pub struct TraceSettings {
    #[setting(default_json = serde_json::json!([]))]
    pub enable_scopes: Vec<String>,

    #[setting(default_json = serde_json::json!([]))]
    pub disable_scopes: Vec<String>,

    #[setting(default_json = serde_json::json!([]))]
    pub disable_messages: Vec<String>,

    #[setting(default_json = serde_json::json!([]))]
    pub disable_targets: Vec<String>,

    #[setting(default = 64u64)]
    pub dump_capacity: u64,
}
