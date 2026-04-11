use app_core::actor::traits::Message;
use macros::{slint_bindings, slint_dto, slint_port};
use serde::{Deserialize, Serialize};

#[slint_dto]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WindowBreakpoint {
    Sm,
    Md,
    Lg,
}

impl Message for WindowConfigChanged {}

#[derive(Debug, Clone)]
pub struct WindowConfigChanged {
    pub breakpoint: WindowBreakpoint,
    pub width: u64,
}

#[slint_dto]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeEdge {
    North,
    South,
    West,
    East,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}

#[slint_port(global = "TitleBarActions")]
pub trait UiWindowActionsPort: Clone + 'static {
    #[manual]
    fn drag_window(&self);
    #[manual]
    fn close_window(&self);
    #[manual]
    fn minimize_window(&self);
    #[manual]
    fn toggle_maximize_window(&self);
    #[manual]
    fn resize_window(&self, edge: ResizeEdge);
}

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
