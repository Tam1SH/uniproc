use context::page_status::PageStatus;
use macros::slint_port;

use super::model::{AvailableContextDescriptor, TabDescriptor};
use super::TabContextKey;

#[slint_port(global = "Tabs")]
pub trait UiTabsPort: 'static {
    #[manual]
    fn set_tabs(&self, tabs: Vec<TabDescriptor>);
    #[manual]
    fn set_available_contexts(&self, contexts: Vec<AvailableContextDescriptor>);
    #[manual]
    fn set_active_context(&self, context_key: TabContextKey);
    #[manual]
    fn set_active_page(&self, context_key: TabContextKey, route_segment: String);
    #[manual]
    fn set_route_status(&self, context_key: TabContextKey, route_segment: String, status: PageStatus);
    #[manual]
    fn set_route_error(&self, context_key: TabContextKey, route_segment: String, msg: String);
}
