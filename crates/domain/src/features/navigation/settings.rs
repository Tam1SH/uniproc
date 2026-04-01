use context::page_status::{PageId, TabId};
use macros::feature_settings;

#[feature_settings(prefix = "navigation")]
pub struct NavigationSettings {
    #[setting(default = 120u64)]
    pub switch_hide_delay_ms: u64,

    #[setting(default = 40u64)]
    pub switch_show_delay_ms: u64,

    #[setting(default = 260u64)]
    pub side_bar_width: u64,

    #[setting(default = (TabId(0), PageId(0)))]
    pub default_page: (TabId, PageId),
}
