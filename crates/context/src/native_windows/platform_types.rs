use slint::RgbaColor;

#[derive(Clone, Copy, Debug)]
pub struct AccentPalette {
    pub accent: RgbaColor<u8>,
    pub accent_light_1: RgbaColor<u8>,
    pub accent_light_2: RgbaColor<u8>,
    pub accent_light_3: RgbaColor<u8>,
    pub accent_dark_1: RgbaColor<u8>,
    pub accent_dark_2: RgbaColor<u8>,
    pub accent_dark_3: RgbaColor<u8>,
}
