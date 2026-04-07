use super::{NativeWindowConfig, NativeWindowTexture};
use i_slint_backend_winit::{winit, WinitWindowAccessor};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{ComponentHandle, RgbaColor};
use std::ptr::null_mut;
use window_vibrancy::{apply_acrylic, apply_mica};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DEFAULT,
    DWMWCP_ROUND,
};
use windows::Win32::UI::Controls::MARGINS;

pub fn apply_to_component<T: ComponentHandle + 'static>(
    component: slint::Weak<T>,
    config: NativeWindowConfig,
) {
    let _ = slint::spawn_local(async move {
        let Some(component) = component.upgrade() else {
            return;
        };
        let window = component.window();
        if window.winit_window().await.is_err() {
            return;
        }

        window.with_winit_window(|winit_window| {
            let hwnd = hwnd_from_winit(winit_window);
            if hwnd.0.is_null() {
                return;
            }

            unsafe {
                let corners = if config.rounded_corners {
                    DWMWCP_ROUND
                } else {
                    DWMWCP_DEFAULT
                };
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &corners as *const _ as *const _,
                    std::mem::size_of::<i32>() as u32,
                );

                if config.texture != NativeWindowTexture::None {
                    let margins = MARGINS {
                        cxLeftWidth: 1,
                        cxRightWidth: 1,
                        cyTopHeight: 1,
                        cyBottomHeight: 1,
                    };
                    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                }
            }

            match config.texture {
                NativeWindowTexture::Mica => {
                    let _ = apply_mica(winit_window, Some(true));
                }
                NativeWindowTexture::Acrylic => {
                    let _ = apply_acrylic(winit_window, Some((20, 20, 20, 150)));
                }
                NativeWindowTexture::None => {}
            }
        });
    });
}

pub fn get_system_accent() -> anyhow::Result<RgbaColor<u8>> {
    use ::windows::UI::ViewManagement::{UIColorType, UISettings};
    let settings = UISettings::new()?;
    let color = settings.GetColorValue(UIColorType::AccentLight2)?;
    Ok(RgbaColor {
        alpha: color.A,
        red: color.R,
        green: color.G,
        blue: color.B,
    })
}

fn hwnd_from_winit(window: &winit::window::Window) -> HWND {
    let handle = match window.window_handle() {
        Ok(handle) => handle.as_raw(),
        Err(_) => return HWND(null_mut()),
    };

    match handle {
        RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as _),
        _ => HWND(null_mut()),
    }
}
