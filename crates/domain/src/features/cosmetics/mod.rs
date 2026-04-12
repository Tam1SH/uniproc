use app_contracts::features::cosmetics::UiCosmeticsPort;
use app_core::app::Feature;
use app_core::app::Window;
use app_core::reactor::Reactor;
use app_core::SharedState;

#[derive(Clone, Copy, Debug)]
pub struct AccentState(pub context::native_windows::platform_types::AccentPalette);

pub struct CosmeticsFeature<F> {
    make_port: F,
}

impl<F> CosmeticsFeature<F> {
    pub fn new(make_port: F) -> Self {
        Self { make_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for CosmeticsFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: UiCosmeticsPort,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let port = (self.make_port)(ui);

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
            shared.insert(AccentState(accent_palette));
        }
        port.apply_main_window_effects();
        Ok(())
    }
}
