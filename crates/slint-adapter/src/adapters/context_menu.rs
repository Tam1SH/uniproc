use crate::{AppWindow, ContextMenuProxy, ProcessContextMenu, ProcessesFeatureGlobal, Theme};
use app_contracts::features::context_menu::{ContextMenuUiBindings, ContextMenuUiPort};
use app_contracts::features::cosmetics::AccentColor;
use macros::ui_adapter;
use slint::ComponentHandle;
use std::rc::Rc;

#[derive(Clone)]
pub struct ContextMenuUiAdapter {
    ui: slint::Weak<AppWindow>,
    menu: Rc<ProcessContextMenu>,
}

impl ContextMenuUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> anyhow::Result<Self> {
        let menu = ProcessContextMenu::new()?;
        platform::configure_window_styles(&menu, 0);
        Ok(Self {
            ui,
            menu: Rc::new(menu),
        })
    }
}

#[ui_adapter]
impl ContextMenuUiPort for ContextMenuUiAdapter {
    fn set_menu_open(&self, ui: &AppWindow, is_open: bool) {
        ui.global::<ContextMenuProxy>().set_is_open(is_open);
    }

    fn invoke_terminate_selected(&self, ui: &AppWindow) {
        ui.global::<ProcessesFeatureGlobal>().invoke_terminate();
    }

    fn on_action<F>(&self, handler: F)
    where
        F: Fn(String) + 'static,
    {
        self.menu
            .as_ref()
            .on_action(move |x| handler(x.to_string()));
    }

    fn show_menu(&self, ui: &AppWindow, x: f32, y: f32, reveal_delay_ms: u64) {
        if self.menu.window().is_visible() {
            let _ = self.menu.hide();
        }

        let main_hwnd = platform::get_window_hwnd(ui.window());
        self.menu.as_ref().set_show_progress(0.0);

        let window_pos = ui.window().position();
        let screen_pos =
            slint::PhysicalPosition::new(window_pos.x + x as i32, window_pos.y + y as i32);
        self.menu.as_ref().window().set_position(screen_pos);

        platform::configure_window_styles(self.menu.as_ref(), main_hwnd);
        let _ = self.menu.as_ref().show();

        let menu_weak = self.menu.as_ref().as_weak();
        slint::Timer::single_shot(
            std::time::Duration::from_millis(reveal_delay_ms.max(1)),
            move || {
                if let Some(m) = menu_weak.upgrade() {
                    m.set_show_progress(1.0);
                }
            },
        );
    }

    fn hide_menu(&self) {
        let _ = self.menu.hide();
    }

    fn set_menu_accent(&self, accent: AccentColor) {
        self.menu
            .as_ref()
            .global::<Theme>()
            .set_accent(slint::Color::from_argb_u8(
                accent.a, accent.r, accent.g, accent.b,
            ));
    }
}

#[ui_adapter]
impl ContextMenuUiBindings for ContextMenuUiAdapter {
    fn on_show_context_menu<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(f32, f32) + 'static,
    {
        ui.global::<ContextMenuProxy>()
            .on_show_context_menu(handler);
    }

    fn on_close_menu<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<ContextMenuProxy>().on_close_menu(handler);
    }
}

mod platform {
    use crate::ProcessContextMenu;
    use i_slint_backend_winit::WinitWindowAccessor;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use slint::ComponentHandle;

    #[cfg(target_os = "windows")]
    use {
        windows::Win32::Foundation::HWND,
        windows::Win32::Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute},
        windows::Win32::UI::WindowsAndMessaging::*,
        windows::core::BOOL,
    };

    pub fn configure_window_styles(menu: &ProcessContextMenu, main_hwnd: isize) {
        #[cfg(target_os = "windows")]
        menu.window().with_winit_window(|_winit_win| {
            let hwnd = HWND(get_window_hwnd(menu.window()) as _);
            if hwnd.0.is_null() {
                return;
            }

            unsafe {
                let mut ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                ex_style |=
                    WS_EX_TOOLWINDOW.0 as i32 | WS_EX_TOPMOST.0 as i32 | WS_EX_NOACTIVATE.0 as i32;
                ex_style &= !(WS_EX_APPWINDOW.0 as i32);

                let _ = SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);
                if main_hwnd != 0 {
                    SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, main_hwnd);
                }

                let _ = SetWindowPos(
                    hwnd,
                    Option::from(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );

                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_TRANSITIONS_FORCEDISABLED,
                    &BOOL::from(true) as *const _ as _,
                    4,
                );

                let style = GetClassLongPtrW(hwnd, GCL_STYLE) as u32;
                let _ = SetClassLongPtrW(hwnd, GCL_STYLE, (style & !CS_DROPSHADOW.0) as isize);
            }
        });
    }

    pub fn get_window_hwnd(window: &slint::Window) -> isize {
        window
            .with_winit_window(|winit_window| {
                match winit_window.window_handle().map(|h| h.as_raw()) {
                    Ok(RawWindowHandle::Win32(handle)) => handle.hwnd.get(),
                    _ => 0,
                }
            })
            .unwrap_or(0)
    }
}
