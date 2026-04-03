use crate::{AppWindow, TitleBarActions, WindowAdapter, WindowSize};
use app_contracts::features::window_actions::{ResizeEdge, WindowActionsPort, WindowBreakpoint};
use i_slint_backend_winit::WinitWindowAccessor;
use macros::ui_adapter;
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
}

#[ui_adapter]
impl WindowActionsPort for WindowActionsAdapter {
    #[ui_action(scope = "ui.window.drag")]
    fn on_drag<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<TitleBarActions>().on_drag(handler);
    }

    #[ui_action(scope = "ui.window.close")]
    fn on_close<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<TitleBarActions>().on_close(handler);
    }

    #[ui_action(scope = "ui.window.minimize")]
    fn on_minimize<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<TitleBarActions>().on_minimize(handler);
    }

    #[ui_action(scope = "ui.window.maximize")]
    fn on_maximize<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<TitleBarActions>().on_maximize(handler);
    }

    #[ui_action(scope = "ui.window.start_resize", target = "edge")]
    fn on_start_resize<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(ResizeEdge) + 'static,
    {
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
    }

    fn on_config_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(WindowBreakpoint, u64) + 'static,
    {
        let adapter = ui.global::<WindowAdapter>();

        let breakpoint = match adapter.get_size() {
            WindowSize::Sm => WindowBreakpoint::Sm,
            WindowSize::Md => WindowBreakpoint::Md,
            WindowSize::Lg => WindowBreakpoint::Lg,
        };

        handler(breakpoint, adapter.get_window_width() as u64);
    }

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
