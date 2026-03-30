use dashmap::DashMap;
use macros::feature_settings;
use serde::{Deserialize, Serialize};

#[feature_settings(prefix = "process")]
pub struct ProcessSettings {
    #[setting(default = 1500u64)]
    scan_interval_ms: u64,

    #[setting(default = true)]
    show_icons: bool,

    #[setting(default = 5000u64)]
    terminate_timeout_ms: u64,

    #[setting(nested)]
    columns: ColumnsSettings,
}

#[feature_settings]
pub struct ColumnsSettings {
    #[setting(default = 70u64)]
    default_width_px: u64,

    #[setting(default = serde_json::json!({
        "name": 200u64,
        "cpu": 90u64,
        "memory": 120u64,
    }))]
    widths_px: DashMap<String, u64>,

    #[setting(default = serde_json::json!({
        "name": { "is-text": true },
        "cpu": { "is-metric": true },
        "memory": { "is-metric": true },
    }))]
    column_metadata: DashMap<String, ColumnMetadata>,

    #[setting(default = serde_json::json!({
        "name": 120u64,
        "cpu": 90u64,
        "memory": 120u64,
    }))]
    min_widths_px: DashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ColumnMetadata {
    #[serde(default)]
    pub is_text: bool,

    #[serde(default)]
    pub is_metric: bool,
}
