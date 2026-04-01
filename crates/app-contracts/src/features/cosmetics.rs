#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccentColor {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub trait CosmeticsPort: Clone + 'static {
    fn get_system_accent_color(&self) -> Option<AccentColor>;
    fn set_main_window_accent(&self, accent: AccentColor);
    fn apply_main_window_effects(&self);
}
