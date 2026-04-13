use macros::slint_bindings;

#[slint_bindings(global = "EnvironmentsFeatureGlobal")]
pub trait UiEnvironmentsBindings: 'static {
    #[tracing(target = "agent")]
    fn on_install_agent<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;
}
