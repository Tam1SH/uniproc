use crate::features::cosmetics::UiCosmeticsAdapter;
use app_contracts::features::cosmetics::{AccentPalette, UiCosmeticsPort};

use framework::native_windows::{apply_to_component, NativeWindowConfig};
use macros::slint_port_adapter;
use slint::ComponentHandle;

#[slint_port_adapter(window = AppWindow)]
impl UiCosmeticsPort for UiCosmeticsAdapter {
    fn apply_main_window_effects(&self, ui: &AppWindow) {
        #[cfg(target_os = "windows")]
        {
            apply_to_component(ui.as_weak(), NativeWindowConfig::win11_dialog());
        }
    }

    fn set_accent_palette(&self, ui: &AppWindow, palette: AccentPalette) {
        let theme = ui.global::<crate::Theme>();
        theme.set_accent(palette.accent);
        theme.set_accent_light_1(palette.accent_light_1);
        theme.set_accent_light_2(palette.accent_light_2);
        theme.set_accent_light_3(palette.accent_light_3);
        theme.set_accent_dark_1(palette.accent_dark_1);
        theme.set_accent_dark_2(palette.accent_dark_2);
        theme.set_accent_dark_3(palette.accent_dark_3);
    }
}
