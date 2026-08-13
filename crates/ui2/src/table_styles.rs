use windows::UI::ViewManagement::{UIColorType, UISettings};
use windows_reactor::{
    border, grid, text_block, Color, Element, ElementExt, GridLength, HorizontalAlignment,
    TextBlock, TextTrimming, TextWrapping, Thickness, VerticalAlignment,
};

const MUTED_TEXT_COLOR: Color = Color {
    a: 255,
    r: 140,
    g: 140,
    b: 140,
};

const HEAT_CORNER: f64 = 4.0;
const HEAT_MARGIN: f64 = 4.0;

pub const MUTED_HEAT_COLOR: Color = Color {
    a: 255,
    r: 150,
    g: 150,
    b: 150,
};

fn heat_alpha(intensity: f32) -> u8 {
    const HEAT_THRESHOLD: f32 = 0.01;

    let clamped = intensity.clamp(0.0, 1.0);
    if clamped < HEAT_THRESHOLD {
        return 0;
    }
    let normalized = (clamped - HEAT_THRESHOLD) / (1.0 - HEAT_THRESHOLD);
    (normalized.powf(0.6) * 230.0) as u8
}

const SEMIBOLD: u16 = 600;

#[derive(Clone, Copy, Debug)]
pub struct TableStyles {
    pub row_height: f64,
    pub section_row_height: f64,
    pub font_size: f64,
    pub separator_color: Color,
    pub terminate_icon_size: f64,
}

impl TableStyles {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            row_height: 16.0,
            section_row_height: 24.0,
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
        no_wrap(text_block(content))
            .font_size(self.font_size)
            .height(self.row_height)
            .max_height(self.row_height)
            .text_trimming(TextTrimming::CharacterEllipsis)
            .into()
    }

    pub fn section_cell(&self, content: impl Into<String>) -> Element {
        no_wrap(text_block(content))
            .font_size(self.font_size + 2.0)
            .font_weight(SEMIBOLD)
            .height(self.section_row_height)
            .max_height(self.section_row_height)
            .text_trimming(TextTrimming::CharacterEllipsis)
            .into()
    }

    pub fn heat_cell(&self, content: impl Into<String>, intensity: f32, accent: Color) -> Element {
        let wash = Color {
            a: heat_alpha(intensity),
            ..accent
        };
        border(self.text_cell(content))
            .background(wash)
            .corner_radius(HEAT_CORNER)
            .margin(Thickness::xy(HEAT_MARGIN, 0.0))
            .into()
    }

    // TODO: unfinished, do not build on it yet.
    // - corners: windows-reactor exposes one f64, so the outer corners cannot
    //   be rounded while the seam stays square (issue filed upstream).
    // - the seam is a bare colour change; it needs a divider once the corners
    //   are sorted, otherwise the two segments read as one pill with a stain.
    // - a small share renders as a sliver: no minimum width, and nothing tells
    //   the reader whether a thin band means "a little" or "almost none".
    // - the text sits over both segments, so it can straddle the seam and lose
    //   contrast on either side.
    pub fn split_heat_cell(
        &self,
        content: impl Into<String>,
        intensity: f32,
        accent: Color,
        left_share: u64,
        right_share: u64,
    ) -> Element {
        let alpha = heat_alpha(intensity);
        let left = Color { a: alpha, ..accent };
        let right = Color {
            a: alpha,
            ..MUTED_HEAT_COLOR
        };

        border(
            grid((
                border(Element::Empty)
                    .background(right)
                    .horizontal_alignment(HorizontalAlignment::Stretch)
                    .vertical_alignment(VerticalAlignment::Stretch)
                    .grid_column(1),
                self.text_cell(content).grid_column(0).grid_column_span(2),
            ))
            .columns([
                GridLength::Star(left_share.max(1) as f64),
                GridLength::Star(right_share.max(1) as f64),
            ]),
        )
        .background(left)
        .corner_radius(HEAT_CORNER)
        .margin(Thickness::xy(HEAT_MARGIN, 0.0))
        .into()
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

pub const TABLE_STYLES: TableStyles = TableStyles::new();

pub fn no_wrap(mut block: TextBlock) -> TextBlock {
    block.text_wrapping = TextWrapping::NoWrap;
    block
}

pub fn cell_text(content: impl Into<String>) -> TextBlock {
    no_wrap(text_block(content))
        .font_size(TABLE_STYLES.font_size)
        .text_trimming(TextTrimming::CharacterEllipsis)
}

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
