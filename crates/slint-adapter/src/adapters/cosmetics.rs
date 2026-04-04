use crate::native_windows::{apply_to_component, NativeWindowConfig};
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
            apply_to_component(
                ui.as_weak(),
                NativeWindowConfig::win11_dialog(),
            );
        }
    }
}
