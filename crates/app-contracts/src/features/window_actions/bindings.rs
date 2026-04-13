use macros::slint_bindings;

use super::model::{ResizeEdge, WindowBreakpoint};

#[slint_bindings(global = "TitleBarActions")]
pub trait UiWindowActionsBindings: 'static {
    #[manual]
    #[tracing(target = "edge")]
    fn on_start_resize<F>(&self, handler: F)
    where
        F: Fn(ResizeEdge) + 'static;
    #[manual]
    #[slint(global = "WindowAdapter")]
    #[tracing(target = "breakpoint,width")]
    fn on_config_changed<F>(&self, handler: F)
    where
        F: Fn(WindowBreakpoint, u64) + 'static;

    fn on_drag<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_close<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_minimize<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_maximize<F>(&self, handler: F)
    where
        F: Fn() + 'static;
}
