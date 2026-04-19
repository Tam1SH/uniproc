use app_contracts::features::cosmetics::UiCosmeticsPort;
use app_core::app::Window;
use app_core::feature::{WindowFeature, WindowFeatureInitContext};
use macros::window_feature;

#[derive(Clone, Copy, Debug)]
pub struct AccentState(pub context::native_windows::platform_types::AccentPalette);

#[window_feature]
pub struct CosmeticsFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for CosmeticsFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiCosmeticsPort,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let port = (self.make_port)(ctx.ui);

        if let Ok(accent_palette) = context::native_windows::platform::get_system_accent_palette() {
            port.set_accent_palette(app_contracts::features::cosmetics::AccentPalette {
                accent: accent_palette.accent.into(),
                accent_light_1: accent_palette.accent_light_1.into(),
                accent_light_2: accent_palette.accent_light_2.into(),
                accent_light_3: accent_palette.accent_light_3.into(),
                accent_dark_1: accent_palette.accent_dark_1.into(),
                accent_dark_2: accent_palette.accent_dark_2.into(),
                accent_dark_3: accent_palette.accent_dark_3.into(),
            });
            ctx.shared.insert(AccentState(accent_palette));
        }
        port.apply_main_window_effects();
        Ok(())
    }
}
