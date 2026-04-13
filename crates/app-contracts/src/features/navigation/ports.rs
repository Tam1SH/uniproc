use context::page_status::{PageId, PageStatus, TabId};
use macros::slint_port;

use super::model::{AvailableContextDescriptor, TabDescriptor};

#[slint_port(global = "Navigation")]
pub trait UiNavigationPort: 'static {
    #[manual]
    fn set_navigation_tree(&self, tabs: Vec<TabDescriptor>);
    #[manual]
    fn set_available_contexts(&self, contexts: Vec<AvailableContextDescriptor>);
    #[manual]
    fn set_active_tab(&self, tab_id: TabId);
    #[manual]
    fn set_active_page(&self, tab_id: TabId, page_id: PageId);
    #[manual]
    fn set_page_status(&self, tab_id: TabId, page_id: PageId, status: PageStatus);
    #[manual]
    fn set_page_error(&self, tab_id: TabId, page_id: PageId, msg: String);
    #[manual]
    fn set_tab_status(&self, tab_id: TabId, status: PageStatus);
    #[manual]
    fn set_tab_error(&self, tab_id: TabId, msg: String);
    #[manual]
    fn set_switch_transition(&self, from_index: i32, to_index: i32, progress: f32);
    #[manual]
    fn set_side_bar_width(&self, width: u64);
    fn set_switch_progress(&self, progress: f32);
    fn set_content_visible(&self, visible: bool);
}
