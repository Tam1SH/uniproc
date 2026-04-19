use macros::feature_settings;

#[feature_settings(prefix = "sidebar")]
pub struct SidebarSettings {
    #[setting(default = 120u64)]
    pub switch_hide_delay_ms: u64,

    #[setting(default = 40u64)]
    pub switch_show_delay_ms: u64,

    #[setting(default = 260u64)]
    pub width: u64,
}
