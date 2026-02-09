use i_slint_backend_winit::WinitWindowAccessor;
use std::ptr::null_mut;
use window_vibrancy::{apply_acrylic, apply_mica};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::UI::Controls::MARGINS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowTexture {
    Mica,
    Acrylic,
    None,
}

pub async fn apply_native_win11_style(slint_window: &slint::Window, texture: WindowTexture) {
    if let Ok(_) = slint_window.winit_window().await {
        slint_window.with_winit_window(|winit_window| {
            let hwnd = HWND(unsafe {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                let handle = winit_window.window_handle().unwrap().as_raw();
                if let RawWindowHandle::Win32(h) = handle {
                    h.hwnd.get() as _
                } else {
                    null_mut()
                }
            });

            if hwnd.0.is_null() {
                return;
            }

            unsafe {
                if let WindowTexture::None = texture {
                } else {
                    let _ = DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE,
                        &DWMWCP_ROUND as *const _ as *const _,
                        std::mem::size_of::<i32>() as u32,
                    );
                    let margins = MARGINS {
                        cxLeftWidth: 1,
                        cxRightWidth: 1,
                        cyTopHeight: 1,
                        cyBottomHeight: 1,
                    };
                    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                }
            }
            match texture {
                WindowTexture::Mica => {
                    let _ = apply_mica(winit_window, Some(true));
                }
                WindowTexture::Acrylic => {
                    let _ = apply_acrylic(winit_window, Some((20, 20, 20, 150)));
                }
                WindowTexture::None => {}
            }
        });
    }
}
