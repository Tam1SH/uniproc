use crate::adapters::cosmetics::CosmeticsAdapter;
use crate::AppWindow;
use app_contracts::features::cosmetics::UiCosmeticsPort;
use context::native_windows::platform::apply_to_component;
use context::native_windows::NativeWindowConfig;
use macros::slint_port_adapter;
use slint::ComponentHandle;

#[slint_port_adapter(window = AppWindow)]
impl UiCosmeticsPort for CosmeticsAdapter {
    fn apply_main_window_effects(&self, ui: &AppWindow) {
        #[cfg(target_os = "windows")]
        {
            apply_to_component(ui.as_weak(), NativeWindowConfig::win11_dialog());
        }
    }
}
