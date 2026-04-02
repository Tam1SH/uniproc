use dashmap::DashMap;
use macros::feature_settings;
use serde::{Deserialize, Serialize};

#[feature_settings(prefix = "services")]
pub struct ServiceSettings {
    #[setting(default = 2000u64)]
    scan_interval_ms: u64,

    #[setting(nested)]
    columns: ServiceColumnsSettings,
}

#[feature_settings]
pub struct ServiceColumnsSettings {
    #[setting(default = 70u64)]
    default_width_px: u64,

    #[setting(default = serde_json::json!({
        "name": 150u64,
        "pid": 80u64,
        "status": 100u64,
        "group": 120u64,
        "description": 100u64,
    }))]
    widths_px: DashMap<String, u64>,

    #[setting(default = serde_json::json!({
        "name": 20u64,
        "pid": 20u64,
        "status": 20u64,
        "group": 20u64,
        "description": 20u64,
    }))]
    min_widths_px: DashMap<String, u64>,

    #[setting(default = serde_json::json!({
        "display_name": { "is_text": true },
        "status": { "is_text": true },
        "description": { "is_text": true },
        "pid": { "is_text": true },
        "group": { "is_text": true },
    }))]
    column_metadata: DashMap<String, ServiceColumnMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceColumnMetadata {
    #[serde(default)]
    pub is_text: bool,
}
