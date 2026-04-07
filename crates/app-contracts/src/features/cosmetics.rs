use slint::RgbaColor;

pub trait CosmeticsPort: Clone + 'static {
    fn set_main_window_accent(&self, accent: RgbaColor<u8>);
    fn apply_main_window_effects(&self);
}
