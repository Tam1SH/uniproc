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

    fn drag_window(&self);
    fn close_window(&self);
    fn minimize_window(&self);
    fn toggle_maximize_window(&self);
    fn resize_window(&self, edge: ResizeEdge);
}
