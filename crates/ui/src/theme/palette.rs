use windows::UI::ViewManagement::{UIColorType, UISettings};
use windows_reactor::{Color, ColorScheme};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Palette {
    pub heat_muted: Color,
    pub track_idle: Color,
}

impl Palette {
    pub fn of(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Dark => Self {
                heat_muted: Color {
                    a: 255,
                    r: 150,
                    g: 150,
                    b: 150,
                },
                track_idle: Color {
                    a: 26,
                    r: 255,
                    g: 255,
                    b: 255,
                },
            },
            ColorScheme::Light => Self {
                heat_muted: Color {
                    a: 255,
                    r: 110,
                    g: 110,
                    b: 110,
                },
                track_idle: Color {
                    a: 26,
                    r: 0,
                    g: 0,
                    b: 0,
                },
            },
        }
    }
}

pub fn accent_color() -> Color {
    const FALLBACK: Color = Color {
        a: 255,
        r: 0,
        g: 120,
        b: 212,
    };
    UISettings::new()
        .and_then(|s| s.GetColorValue(UIColorType::Accent))
        .map(|c| Color {
            a: c.A,
            r: c.R,
            g: c.G,
            b: c.B,
        })
        .unwrap_or(FALLBACK)
}
