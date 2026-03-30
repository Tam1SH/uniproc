use macros::feature_settings;

#[feature_settings(prefix = "context_menu")]
pub struct ContextMenuSettings {
    #[setting(default = 20u64)]
    pub reveal_delay_ms: u64,
}
