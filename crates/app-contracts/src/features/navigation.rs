use app_core::actor::traits::Message;
use context::page_status::{PageId, PageStatus, TabId};
use macros::{slint_bindings, slint_port};

pub mod page_ids {
    use super::PageId;
    pub const PROCESSES: PageId = PageId(1);
    pub const PERFORMANCE: PageId = PageId(2);
    pub const DISK: PageId = PageId(3);
    pub const STATISTICS: PageId = PageId(4);
    pub const STARTUP_APPS: PageId = PageId(5);
    pub const USERS: PageId = PageId(6);
    pub const SERVICES: PageId = PageId(7);
}

pub mod tab_ids {
    use super::TabId;
    pub const MAIN: TabId = TabId(0);
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageDescriptor {
    pub id: PageId,
    pub text: String,
    pub icon_key: String,
    pub status: PageStatus,
    pub error_msg: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabDescriptor {
    pub id: TabId,
    pub title: String,
    pub pages: Vec<PageDescriptor>,
    pub status: PageStatus,
    pub error_msg: String,
}

#[derive(Clone, Debug)]
pub struct PageActivated {
    pub tab_id: TabId,
    pub page_id: PageId,
}

impl Message for PageActivated {}

#[derive(Clone, Debug)]
pub struct TabActivated {
    pub tab_id: TabId,
}

impl Message for TabActivated {}

#[slint_port(global = "Navigation")]
pub trait NavigationUiPort: 'static {
    #[manual]
    fn set_navigation_tree(&self, tabs: Vec<TabDescriptor>);
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

#[slint_bindings(global = "Navigation")]
pub trait NavigationUiBindings: 'static {
    #[manual]
    #[tracing(target = "tab_id,page_id")]
    fn on_request_page_switch<F>(&self, handler: F)
    where
        F: Fn(TabId, PageId) + 'static;

    #[manual]
    #[tracing(target = "width")]
    fn on_side_bar_width_changed<F>(&self, handler: F)
    where
        F: Fn(u64) + 'static;

    #[manual]
    #[tracing(target = "tab_id")]
    fn on_request_tab_switch<F>(&self, handler: F)
    where
        F: Fn(TabId) + 'static;

    #[manual]
    #[tracing(target = "tab_id")]
    fn on_request_tab_close<F>(&self, handler: F)
    where
        F: Fn(TabId) + 'static;

    #[manual]
    fn on_request_tab_add<F>(&self, handler: F)
    where
        F: Fn() + 'static;
}
