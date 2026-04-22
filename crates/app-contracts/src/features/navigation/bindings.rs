use macros::slint_bindings;

#[slint_bindings(global = "Navigation")]
pub trait UiNavigationBindings: 'static {
    #[manual]
    #[tracing(target = "route_segment")]
    fn on_request_route_switch<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    #[manual]
    #[tracing(target = "context_key")]
    fn on_request_tab_switch<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    #[manual]
    #[tracing(target = "context_key")]
    fn on_request_tab_close<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    #[manual]
    #[tracing(target = "context_key")]
    fn on_request_tab_add<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;
}
