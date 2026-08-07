use windows::UI::ViewManagement::{UIColorType, UISettings};
use windows_reactor::{border, text_block, Color, Element, ElementExt, TextTrimming, Thickness};

#[derive(Clone, Copy, Debug)]
pub struct TableStyles {
    pub row_height: f64,
    pub font_size: f64,
    pub separator_color: Color,
    pub terminate_icon_size: f64,
}

impl TableStyles {
    //const Default?, lol.
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            row_height: 16.0,
            font_size: 12.0,
            separator_color: Color {
                a: 48,
                r: 128,
                g: 128,
                b: 128,
            },
            terminate_icon_size: 16.0,
        }
    }

    pub fn text_cell(&self, content: impl Into<String>) -> Element {
        text_block(content)
            .font_size(self.font_size)
            .height(self.row_height)
            .max_height(self.row_height)
            .text_trimming(TextTrimming::CharacterEllipsis)
            .into()
    }

    /// A cell with a background wash whose opacity tracks `intensity`
    /// (`0.0..=1.0`) - the periphery-readable "heat" for a metric value,
    /// without a separate bar/progress control competing with the text.
    ///
    /// Below `HEAT_THRESHOLD` there's no wash at all - most rows sit at a
    /// fraction of a percent and a hairline tint on literally every row
    /// would just be noise, not signal. From the threshold up, the curve
    /// is `normalized.powf(0.6)`, not linear: a linear ramp made everything
    /// below ~80% read as barely-there, since the visually interesting
    /// cases (rows approaching the limit) are exactly what a linear map
    /// compresses hardest into the low end.
    ///
    /// The wash sits 4px in from the cell's left/right edges (`margin`,
    /// not `padding` - padding would shrink the *text* inside a
    /// still-edge-to-edge colored box; margin shrinks the colored box
    /// itself), so it reads as a floating pill rather than colored
    /// wall-to-wall banding between adjacent cells.
    pub fn heat_cell(&self, content: impl Into<String>, intensity: f32, accent: Color) -> Element {
        const HEAT_THRESHOLD: f32 = 0.01;

        let clamped = intensity.clamp(0.0, 1.0);
        let alpha = if clamped < HEAT_THRESHOLD {
            0
        } else {
            let normalized = (clamped - HEAT_THRESHOLD) / (1.0 - HEAT_THRESHOLD);
            (normalized.powf(0.6) * 230.0) as u8
        };
        let wash = Color { a: alpha, ..accent };
        border(self.text_cell(content))
            .background(wash)
            .corner_radius(4.0)
            .margin(Thickness::xy(4.0, 0.0))
            .into()
    }
}

/// Reads the live system accent color (`UIColorType::Accent`) once per call.
/// Callers driving per-row heat cells should resolve this a single time per
/// render pass and capture it by value, not call it from inside a per-row
/// closure - `UISettings` round-trips through the OS on every read.
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

pub const TABLE_STYLES: TableStyles = TableStyles::new();

pub fn format_bytes(v: u64) -> String {
    const KIB: f64 = 1024.0;
    let f = v as f64;
    if f >= KIB.powi(3) {
        format!("{:.1} GiB", f / KIB.powi(3))
    } else if f >= KIB.powi(2) {
        format!("{:.1} MiB", f / KIB.powi(2))
    } else if f >= KIB {
        format!("{:.0} KiB", f / KIB)
    } else {
        format!("{v} B")
    }
}
