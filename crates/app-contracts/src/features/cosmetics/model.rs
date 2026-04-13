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
