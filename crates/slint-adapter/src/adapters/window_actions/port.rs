use crate::adapters::window_actions::WindowActionsAdapter;
use crate::AppWindow;
use app_contracts::features::window_actions::{ResizeEdge, UiWindowActionsPort};
use i_slint_backend_winit::WinitWindowAccessor;
use macros::slint_port_adapter;
use slint::ComponentHandle;
use winit::window::ResizeDirection;

#[slint_port_adapter(window = AppWindow)]
impl UiWindowActionsPort for WindowActionsAdapter {
    fn drag_window(&self, ui: &AppWindow) {
        ui.window().with_winit_window(|w| {
            let _ = w.drag_window();
        });
    }

    fn close_window(&self, ui: &AppWindow) {
        let _ = ui.hide();
    }

    fn minimize_window(&self, ui: &AppWindow) {
        ui.window().set_minimized(true);
    }

    fn toggle_maximize_window(&self, ui: &AppWindow) {
        let window = ui.window();
        window.set_maximized(!window.is_maximized());
    }

    fn resize_window(&self, ui: &AppWindow, edge: ResizeEdge) {
        let direction = match edge {
            ResizeEdge::North => ResizeDirection::North,
            ResizeEdge::South => ResizeDirection::South,
            ResizeEdge::West => ResizeDirection::West,
            ResizeEdge::East => ResizeDirection::East,
            ResizeEdge::NorthWest => ResizeDirection::NorthWest,
            ResizeEdge::NorthEast => ResizeDirection::NorthEast,
            ResizeEdge::SouthWest => ResizeDirection::SouthWest,
            ResizeEdge::SouthEast => ResizeDirection::SouthEast,
        };
        ui.window().with_winit_window(|w| {
            let _ = w.drag_resize_window(direction);
        });
    }
}
