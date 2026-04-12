use macros::slint_port;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccentPalette {
    pub accent: slint::Color,
    pub accent_light_1: slint::Color,
    pub accent_light_2: slint::Color,
    pub accent_light_3: slint::Color,
    pub accent_dark_1: slint::Color,
    pub accent_dark_2: slint::Color,
    pub accent_dark_3: slint::Color,
}

#[slint_port(global = "Theme")]
pub trait UiCosmeticsPort: Clone + 'static {
    #[manual]
    fn apply_main_window_effects(&self);
    #[manual]
    fn set_accent_palette(&self, palette: AccentPalette);
}
