use crate::{AppWindow, Theme};
use app_contracts::features::cosmetics::CosmeticsPort;
use context::native_windows::{apply_to_component, NativeWindowConfig};
use macros::ui_adapter;
use slint::{Color, ComponentHandle, RgbaColor};

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
    fn set_main_window_accent(&self, ui: &AppWindow, accent: RgbaColor<u8>) {
        ui.global::<Theme>().set_accent(accent.into());
    }

    fn apply_main_window_effects(&self, ui: &AppWindow) {
        #[cfg(target_os = "windows")]
        {
            apply_to_component(ui.as_weak(), NativeWindowConfig::win11_dialog());
        }
    }
}
