use super::{platform_types::AccentPalette, NativeWindowConfig};
use slint::{ComponentHandle, RgbaColor};

pub fn apply_to_component<T: ComponentHandle + 'static>(
    _component: slint::Weak<T>,
    _config: NativeWindowConfig,
) {
}

pub fn get_system_accent_palette() -> anyhow::Result<AccentPalette> {
    Ok(AccentPalette {
        accent: rgba(15, 108, 189),
        accent_light_1: rgba(17, 94, 163),
        accent_light_2: rgba(15, 108, 189),
        accent_light_3: rgba(71, 158, 245),
        accent_dark_1: rgba(15, 84, 140),
        accent_dark_2: rgba(12, 59, 94),
        accent_dark_3: rgba(8, 36, 59),
    })
}

fn rgba(red: u8, green: u8, blue: u8) -> RgbaColor<u8> {
    RgbaColor {
        alpha: 255,
        red,
        green,
        blue,
    }
}
