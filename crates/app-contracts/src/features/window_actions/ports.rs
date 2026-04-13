use macros::slint_port;

use super::model::ResizeEdge;

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
