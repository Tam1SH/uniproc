use windows_reactor::{
    body_strong, border, caption, grid, Color, Element, ElementExt, GridLength, HorizontalAlignment, TextBlock,
    TextTrimming, TextWrapping, Thickness, VerticalAlignment,
};

use crate::theme::{radius, size, space};

pub fn heat_alpha(intensity: f32) -> u8 {
    const HEAT_THRESHOLD: f32 = 0.01;

    let clamped = intensity.clamp(0.0, 1.0);
    if clamped < HEAT_THRESHOLD {
        return 0;
    }
    let normalized = (clamped - HEAT_THRESHOLD) / (1.0 - HEAT_THRESHOLD);
    (normalized.powf(0.6) * 230.0) as u8
}

pub fn no_wrap(mut block: TextBlock) -> TextBlock {
    block.text_wrapping = TextWrapping::NoWrap;
    block
}

pub fn cell_text(content: impl Into<String>) -> TextBlock {
    no_wrap(caption(content)).text_trimming(TextTrimming::CharacterEllipsis)
}

pub fn text_cell(content: impl Into<String>) -> Element {
    cell_text(content)
        .height(size::TableRow)
        .max_height(size::TableRow)
        .into()
}

pub fn section_cell(content: impl Into<String>) -> Element {
    let text = no_wrap(body_strong(content))
        .text_trimming(TextTrimming::CharacterEllipsis)
        .vertical_alignment(VerticalAlignment::Center);

    border(text)
        .padding(Thickness::xy(0.0, space::Compact))
        .into()
}

pub fn heat_cell(content: impl Into<String>, intensity: f32, accent: Color) -> Element {
    let wash = Color {
        a: heat_alpha(intensity),
        ..accent
    };
    border(text_cell(content))
        .background(wash)
        .corner_radius(radius::Control)
        .margin(Thickness::xy(space::Compact, 0.0))
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
    content: impl Into<String>,
    intensity: f32,
    accent: Color,
    muted: Color,
    left_share: u64,
    right_share: u64,
) -> Element {
    let alpha = heat_alpha(intensity);
    let left = Color { a: alpha, ..accent };
    let right = Color { a: alpha, ..muted };

    border(
        grid((
            border(Element::Empty)
                .background(right)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Stretch)
                .grid_column(1),
            text_cell(content).grid_column(0).grid_column_span(2),
        ))
        .columns([
            GridLength::Star(left_share.max(1) as f64),
            GridLength::Star(right_share.max(1) as f64),
        ]),
    )
    .background(left)
    .corner_radius(radius::Control)
    .margin(Thickness::xy(space::Compact, 0.0))
    .into()
}
