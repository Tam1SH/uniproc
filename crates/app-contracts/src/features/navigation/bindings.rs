use macros::slint_bindings;

#[slint_bindings(global = "Navigation")]
pub trait UiNavigationBindings: 'static {
    #[manual]
    #[tracing(target = "path")]
    fn on_push<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;
}
