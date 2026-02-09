use crate::features::context_menu::actors::SystemFocusChanged;
use crate::features::context_menu::{ACTOR_ADDR, HOOK_HANDLE};
use crate::ProcessContextMenu;
use i_slint_backend_winit::WinitWindowAccessor;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use windows::core::BOOL;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn configure_window_styles(menu: &ProcessContextMenu, main_hwnd: isize) {
    menu.window().with_winit_window(|winit_win| {
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

            SetWindowPos(
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

pub fn start_win_event_hook() {
    stop_win_event_hook();
    unsafe {
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(focus_change_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        HOOK_HANDLE.with(|h| *h.borrow_mut() = Some(hook));
    }
}

pub fn stop_win_event_hook() {
    HOOK_HANDLE.with(|h| {
        if let Some(handle) = h.borrow_mut().take() {
            unsafe { UnhookWinEvent(handle) };
        }
    });
}

pub fn get_window_hwnd(window: &slint::Window) -> isize {
    window
        .with_winit_window(
            |winit_window| match winit_window.window_handle().map(|h| h.as_raw()) {
                Ok(RawWindowHandle::Win32(handle)) => handle.hwnd.get() as isize,
                _ => 0,
            },
        )
        .unwrap_or(0)
}

unsafe extern "system" fn focus_change_callback(
    _h: HWINEVENTHOOK,
    _e: u32,
    hwnd: HWND,
    _id: i32,
    _child: i32,
    _t: u32,
    _ms: u32,
) {
    ACTOR_ADDR.with(|addr| {
        if let Some(a) = addr.borrow().as_ref() {
            a.send(SystemFocusChanged(hwnd.0 as isize));
        }
    });
}
