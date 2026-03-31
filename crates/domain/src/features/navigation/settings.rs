use macros::feature_settings;

#[feature_settings(prefix = "navigation")]
pub struct NavigationSettings {
    #[setting(default = 60u64)]
    pub switch_hide_delay_ms: u64,

    #[setting(default = 20u64)]
    pub switch_show_delay_ms: u64,

    #[setting(default = 260u64)]
    pub side_bar_width: u64,
}
