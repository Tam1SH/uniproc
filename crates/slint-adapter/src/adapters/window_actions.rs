use crate::{AppWindow, TitleBarActions};
use app_contracts::features::window_actions::{ResizeEdge, WindowActionsPort};
use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;
use winit::window::ResizeDirection;

#[derive(Clone)]
pub struct WindowActionsAdapter {
    ui: slint::Weak<AppWindow>,
}

impl WindowActionsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }

    fn with_ui<F>(&self, f: F)
    where
        F: FnOnce(&AppWindow),
    {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui);
        }
    }
}

impl WindowActionsPort for WindowActionsAdapter {
    fn on_drag<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_ui(|ui| ui.global::<TitleBarActions>().on_drag(handler));
    }

    fn on_close<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_ui(|ui| ui.global::<TitleBarActions>().on_close(handler));
    }

    fn on_minimize<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_ui(|ui| ui.global::<TitleBarActions>().on_minimize(handler));
    }

    fn on_maximize<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_ui(|ui| ui.global::<TitleBarActions>().on_maximize(handler));
    }

    fn on_start_resize<F>(&self, handler: F)
    where
        F: Fn(ResizeEdge) + 'static,
    {
        self.with_ui(|ui| {
            ui.on_start_resize(move |v| {
                let edge = match v {
                    0 => ResizeEdge::North,
                    1 => ResizeEdge::South,
                    2 => ResizeEdge::West,
                    3 => ResizeEdge::East,
                    4 => ResizeEdge::NorthWest,
                    5 => ResizeEdge::NorthEast,
                    6 => ResizeEdge::SouthWest,
                    7 => ResizeEdge::SouthEast,
                    _ => return,
                };
                handler(edge);
            });
        });
    }

    fn drag_window(&self) {
        self.with_ui(|ui| {
            ui.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        });
    }

    fn close_window(&self) {
        self.with_ui(|ui| {
            let _ = ui.hide();
        });
    }

    fn minimize_window(&self) {
        self.with_ui(|ui| ui.window().set_minimized(true));
    }

    fn toggle_maximize_window(&self) {
        self.with_ui(|ui| {
            let window = ui.window();
            window.set_maximized(!window.is_maximized());
        });
    }

    fn resize_window(&self, edge: ResizeEdge) {
        self.with_ui(|ui| {
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
        });
    }
}
