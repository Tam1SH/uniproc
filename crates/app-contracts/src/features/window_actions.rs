use app_core::actor::traits::Message;
use serde::{Deserialize, Serialize};
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

//TODO: extract Binding trait
pub trait WindowActionsPort: Clone + 'static {
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

    fn on_start_resize<F>(&self, handler: F)
    where
        F: Fn(ResizeEdge) + 'static;
    fn on_config_changed<F>(&self, handler: F)
    where
        F: Fn(WindowBreakpoint, u64) + 'static;

    fn drag_window(&self);
    fn close_window(&self);
    fn minimize_window(&self);
    fn toggle_maximize_window(&self);
    fn resize_window(&self, edge: ResizeEdge);
}
