use context::page_status::{PageId, TabId};
use macros::slint_bindings;

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
