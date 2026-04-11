use macros::slint_port;


#[slint_port(global = "Theme")]
pub trait UiCosmeticsPort: Clone + 'static {
    #[manual]
    fn apply_main_window_effects(&self);
    fn set_accent(&self, accent: slint::Color);
}
