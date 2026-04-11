use crate::adapters::window_actions::WindowActionsAdapter;
use crate::{AppWindow, TitleBarActions, WindowAdapter, WindowSize};
use app_contracts::features::window_actions::{
    ResizeEdge, UiWindowActionsBindings, WindowBreakpoint,
};
use macros::slint_bindings_adapter;
use slint::ComponentHandle;

#[slint_bindings_adapter(window = AppWindow)]
impl UiWindowActionsBindings for WindowActionsAdapter {
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
        let ui_weak = self.ui.clone();
        ui.global::<WindowAdapter>().on_size_changed(move |size| {
            let Some(ui) = ui_weak.upgrade() else { return };
            let adapter = ui.global::<WindowAdapter>();
            let breakpoint = match size {
                WindowSize::Sm => WindowBreakpoint::Sm,
                WindowSize::Md => WindowBreakpoint::Md,
                WindowSize::Lg => WindowBreakpoint::Lg,
            };
            handler(breakpoint, adapter.get_window_width() as u64);
        });
    }
}
