use context::page_status::{PageId, TabId};
use macros::feature_settings;

#[feature_settings(prefix = "navigation")]
pub struct NavigationSettings {
    #[setting(default = (TabId(0), PageId(0)))]
    pub default_page: (TabId, PageId),
}
