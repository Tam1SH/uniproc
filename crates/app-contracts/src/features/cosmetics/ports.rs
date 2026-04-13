use macros::slint_port;

use super::model::AccentPalette;

#[slint_port(global = "Theme")]
pub trait UiCosmeticsPort: Clone + 'static {
    #[manual]
    fn apply_main_window_effects(&self);
    #[manual]
    fn set_accent_palette(&self, palette: AccentPalette);
}
