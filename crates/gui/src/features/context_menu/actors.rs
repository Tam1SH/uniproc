use crate::core::actor::traits::Context;
use crate::core::actor::traits::Handler;
use crate::core::actor::traits::Message;
use crate::features::context_menu::utils::{configure_window_styles, get_window_hwnd};
use crate::features::context_menu::utils::{start_win_event_hook, stop_win_event_hook};
use crate::ProcessContextMenu;
use crate::{messages, ContextMenuProxy};
use crate::{AppWindow, ProcessBridge};
use slint::{ComponentHandle, SharedString};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;

messages! {
    Show { x: f32, y: f32 },
    Hide,
    HandleAction(SharedString),
    SystemFocusChanged(isize),
}

pub struct ContextMenuActor {
    pub menu: ProcessContextMenu,
    pub main_hwnd: isize,
    pub menu_hwnd: isize,
}

impl Handler<Show, AppWindow> for ContextMenuActor {
    fn handle(&mut self, msg: Show, ctx: &Context<Self, AppWindow>) {
        ctx.with_ui(|ui| {
            if self.menu.window().is_visible() {
                let _ = self.menu.hide();
            }

            self.main_hwnd = get_window_hwnd(ui.window());
            self.menu_hwnd = get_window_hwnd(self.menu.window());

            self.menu.set_show_progress(0.0);

            let window_pos = ui.window().position();
            let screen_pos = slint::PhysicalPosition::new(
                window_pos.x + msg.x as i32,
                window_pos.y + msg.y as i32,
            );
            self.menu.window().set_position(screen_pos);

            configure_window_styles(&self.menu, self.main_hwnd);

            start_win_event_hook();
            self.menu.show().unwrap();
            ui.global::<ContextMenuProxy>().set_is_open(true);

            let menu_weak = self.menu.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(20), move || {
                if let Some(m) = menu_weak.upgrade() {
                    m.set_show_progress(1.0);
                }
            });
        });
    }
}

impl Handler<Hide, AppWindow> for ContextMenuActor {
    fn handle(&mut self, _msg: Hide, ctx: &Context<Self, AppWindow>) {
        let _ = self.menu.hide();
        stop_win_event_hook();
        ctx.with_ui(|ui| {
            ui.global::<ContextMenuProxy>().set_is_open(false);
        });
    }
}

impl Handler<HandleAction, AppWindow> for ContextMenuActor {
    fn handle(&mut self, msg: HandleAction, ctx: &Context<Self, AppWindow>) {
        let _ = self.menu.hide();
        stop_win_event_hook();

        ctx.with_ui(|ui| {
            let bridge = ui.global::<ProcessBridge>();

            ui.global::<ContextMenuProxy>().set_is_open(false);

            match msg.0.as_str() {
                "terminate" => bridge.invoke_terminate(),
                "open-location" => bridge.invoke_open_file_location(),
                "properties" => bridge.invoke_open_properties(),
                _ => {}
            }
        });
    }
}

impl Handler<SystemFocusChanged, AppWindow> for ContextMenuActor {
    fn handle(&mut self, msg: SystemFocusChanged, ctx: &Context<Self, AppWindow>) {
        let focused_hwnd = msg.0;

        if focused_hwnd == self.menu_hwnd {
            unsafe {
                SetActiveWindow(HWND(self.main_hwnd as _));
            }
        } else if focused_hwnd != self.main_hwnd {
            ctx.addr().send(Hide);
        }
    }
}
