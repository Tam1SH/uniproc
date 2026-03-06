use super::Feature;
use crate::core::reactor::Reactor;
use crate::{AppWindow, Theme};
use anyhow::Context;
use app_core::SharedState;
use i_slint_backend_winit::WinitWindowAccessor;
use slint::{Color, ComponentHandle};

use crate::features::cosmetics::utils::{WindowTexture, apply_native_win11_style};
use windows::UI::ViewManagement::{UIColorType, UISettings};

pub mod utils;
pub struct CosmeticsFeature;

impl CosmeticsFeature {
    pub fn get_system_accent_color() -> Option<Color> {
        let settings = UISettings::new().ok()?;
        let color = settings.GetColorValue(UIColorType::AccentLight2).ok()?;

        Some(Color::from_argb_u8(color.A, color.R, color.G, color.B))
    }
}

impl Feature for CosmeticsFeature {
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &AppWindow,
        _shared: &SharedState,
    ) -> anyhow::Result<()> {
        let ui_weak = ui.as_weak();

        if let Some(accent) = Self::get_system_accent_color() {
            ui.global::<Theme>().set_accent(accent);
        }

        slint::spawn_local(async move {
            if let Some(app) = ui_weak.upgrade() {
                apply_native_win11_style(app.window(), WindowTexture::Mica).await;
            }
        })
        .context("Failed to setup native cosmetics")?;

        Ok(())
    }
}
