use crate::{AppWindow, Theme};
use app_contracts::features::cosmetics::{AccentColor, CosmeticsPort};
use macros::ui_adapter;
use slint::ComponentHandle;

#[derive(Clone)]
pub struct CosmeticsAdapter {
    ui: slint::Weak<AppWindow>,
}

impl CosmeticsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}

#[ui_adapter]
impl CosmeticsPort for CosmeticsAdapter {
    fn get_system_accent_color(&self) -> Option<AccentColor> {
        #[cfg(target_os = "windows")]
        {
            use ::windows::UI::ViewManagement::{UIColorType, UISettings};
            let settings = UISettings::new().ok()?;
            let color = settings.GetColorValue(UIColorType::AccentLight2).ok()?;
            Some(AccentColor {
                a: color.A,
                r: color.R,
                g: color.G,
                b: color.B,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }

    fn set_main_window_accent(&self, ui: &AppWindow, accent: AccentColor) {
        ui.global::<Theme>().set_accent(slint::Color::from_argb_u8(
            accent.a, accent.r, accent.g, accent.b,
        ));
    }

    fn apply_main_window_effects(&self, ui: &AppWindow) {
        #[cfg(target_os = "windows")]
        {
            let ui_weak = ui.as_weak();
            let _ = slint::spawn_local(async move {
                if let Some(ui) = ui_weak.upgrade() {
                    windows::apply_native_win11_style(ui.window(), windows::WindowTexture::Mica)
                        .await;
                }
            });
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    #[allow(dead_code)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum WindowTexture {
        Mica,
        Acrylic,
        None,
    }

    pub(crate) async fn apply_native_win11_style(
        slint_window: &slint::Window,
        texture: WindowTexture,
    ) {
        use i_slint_backend_winit::WinitWindowAccessor;
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use std::ptr::null_mut;
        use window_vibrancy::{apply_acrylic, apply_mica};
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmExtendFrameIntoClientArea,
            DwmSetWindowAttribute,
        };
        use windows::Win32::UI::Controls::MARGINS;

        if slint_window.winit_window().await.is_err() {
            return;
        }

        slint_window.with_winit_window(|winit_window| {
            let hwnd = HWND({
                let handle = match winit_window.window_handle() {
                    Ok(h) => h.as_raw(),
                    Err(_) => return,
                };
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
                if texture != WindowTexture::None {
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
