use guinea::widgets::chart::{line_chart_with_options, Interpolation, LineChartOptions, Series};
use guinea::widgets::color::hex;
use guinea_core::Load;
use windows_canvas::ColorF;
use windows_reactor::{
    border, grid, hstack, text_block, vstack, Color, Element, ElementExt, GridLength,
    HorizontalAlignment, RenderCx, ThemeRef, Thickness, VerticalAlignment,
};

use crate::table_styles;

#[derive(Clone, Copy, Debug)]
pub enum MetricChartKind {
    Cpu,
    Memory,
}

impl MetricChartKind {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
        }
    }

    pub fn color(&self) -> ColorF {
        match self {
            Self::Cpu => hex(0x60a5fa),
            Self::Memory => hex(0x34d399),
        }
    }

    /// Same accent as [`color`](Self::color), as a `windows_reactor::Color`
    /// for widgets outside the D2D chart surface (e.g. [`metric_mini_bar`]).
    fn color_direct(&self) -> Color {
        match self {
            Self::Cpu => Color { a: 255, r: 0x60, g: 0xa5, b: 0xfa },
            Self::Memory => Color { a: 255, r: 0x34, g: 0xd3, b: 0x99 },
        }
    }
}

/// How a [`metric_chart`] should present itself - the same label+sparkline
/// pairing works both as the Processes page's full chart and as a compact
/// sidebar tile, but the tile has no room (and no need) for a grid or its
/// own card framing once it's already sitting inside the nav pane footer.
#[derive(Clone, Copy, Debug)]
pub struct MetricChartStyle {
    pub show_grid: bool,
    /// Wraps the chart in its own `CardBackground`/`CardStroke` surface.
    /// Off for the sidebar tile, which already sits inside the pane
    /// footer's own background - a second nested card would be redundant
    /// framing around framing.
    pub card: bool,
}

impl MetricChartStyle {
    pub const CARD: Self = Self { show_grid: true, card: true };
    pub const SPARKLINE: Self = Self { show_grid: false, card: false };
}

/// Width/height of the vertical mini-bar shown in the collapsed (icon-only)
/// pane - a sparkline has no room to be legible at that width, so the trend
/// line collapses down to a single filled bar tracking the current value.
const MINI_BAR_WIDTH: f64 = 6.0;
const MINI_BAR_HEIGHT: f64 = 34.0;

const WARNING_THRESHOLD: f32 = 65.0;
const CRITICAL_THRESHOLD: f32 = 90.0;
/// At and above this, the whole bar (fill included, not just the track)
/// goes critical - the metric itself is the alarm, not just "approaching".
const CRITICAL_FILL_THRESHOLD: f32 = 95.0;

// Kept as `Color` (Direct), not `ThemeRef` - these swap in and out on every
// render as the metric crosses its thresholds, and a prop that has ever been
// set with a Direct value can never be won back by a `ThemeRef` Style
// Setter afterward (WinUI's local-value-beats-style-setter precedence is
// permanent, not per-render). Mixing Direct and Theme on the same prop
// across renders silently and irrecoverably breaks the Theme side.
const WARNING_COLOR: Color = Color { a: 255, r: 0xFF, g: 0xB9, b: 0x00 };
const CRITICAL_COLOR: Color = Color { a: 255, r: 0xE7, g: 0x48, b: 0x56 };
/// Direct-color stand-in for `ThemeRef::SubtleFill`, for the same reason -
/// the track alternates with the washes below.
const TRACK_NORMAL_COLOR: Color = Color { a: 26, r: 255, g: 255, b: 255 };
/// The track behind the fill reads as a status wash, not a second solid
/// block of color competing with the fill itself - same hues as
/// `WARNING_COLOR`/`CRITICAL_COLOR`, dropped to a fraction of their alpha.
const TRACK_WARNING_COLOR: Color = Color { a: 130, ..WARNING_COLOR };
const TRACK_CRITICAL_COLOR: Color = Color { a: 130, ..CRITICAL_COLOR };

/// The collapsed-pane counterpart to [`metric_chart`] - same current-value
/// source, rendered as a vertical fill bar instead of a labeled sparkline,
/// since the compact pane has no width for either a label or a trend line.
pub fn metric_mini_bar(kind: MetricChartKind, history: &Load<Vec<(u64, f32)>>) -> Element {
    let current = history
        .ready()
        .and_then(|points| points.last())
        .map(|&(_, v)| v)
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);

    let fill_color = if current >= CRITICAL_FILL_THRESHOLD {
        CRITICAL_COLOR
    } else {
        kind.color_direct()
    };
    let track_color = if current > CRITICAL_THRESHOLD {
        TRACK_CRITICAL_COLOR
    } else if current > WARNING_THRESHOLD {
        TRACK_WARNING_COLOR
    } else {
        TRACK_NORMAL_COLOR
    };

    let fill = border(Element::Empty)
        .height(MINI_BAR_HEIGHT * (current as f64 / 100.0))
        .background(fill_color)
        .corner_radius(MINI_BAR_WIDTH / 2.0)
        .vertical_alignment(VerticalAlignment::Bottom);

    border(fill)
        .width(MINI_BAR_WIDTH)
        .height(MINI_BAR_HEIGHT)
        .corner_radius(MINI_BAR_WIDTH / 2.0)
        .background(track_color)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Bottom)
        .into()
}

pub fn metric_chart(
    cx: &mut RenderCx,
    kind: MetricChartKind,
    history: &Load<Vec<(u64, f32)>>,
    height: f64,
    style: MetricChartStyle,
    detail: Option<String>,
) -> Element {
    let points = history.ready().cloned().unwrap_or_default();
    let current = points.last().map(|&(_, v)| v).unwrap_or(0.0);

    let series = Series {
        color: kind.color(),
        interpolation: Interpolation::Linear,
        fill: None,
        points,
    };

    let options = LineChartOptions {
        // `None` leaves the D2D surface transparent after `clear()` instead
        // of painting an opaque backdrop rect over it - the surface is its
        // own composition layer, so any solid fill here (even a color
        // matched to the page background) would sit as a visible seam over
        // whatever's really behind it (flat fill, Mica, acrylic, ...).
        // Transparent is the only value that always matches, because it
        // doesn't try to guess.
        background: None,
        border: None,
        show_grid: style.show_grid,
        y_range: Some((0.0, 100.0)),
    };
    let chart = line_chart_with_options(cx, vec![series], |_| {}, options).height(height);

    // Label (+ optional hardware detail - clock speed, total memory) on the
    // left, value pinned to the right edge. `hstack` alone has no
    // "space-between" behavior, so the value would otherwise end up glued
    // right after the label's text instead of reading as a label/value pair.
    let label: Element = match detail {
        Some(detail) => hstack((
            text_block(kind.title()).font_size(table_styles::TABLE_STYLES.font_size),
            text_block(detail)
                .font_size(table_styles::TABLE_STYLES.font_size)
                .foreground(ThemeRef::TertiaryText),
        ))
        .spacing(6.0)
        .into(),
        None => text_block(kind.title())
            .font_size(table_styles::TABLE_STYLES.font_size)
            .into(),
    };
    let header = grid((
        label.grid_column(0),
        text_block(format!("{current:.1}%"))
            .font_size(table_styles::TABLE_STYLES.font_size)
            .foreground(ThemeRef::SecondaryText)
            .horizontal_alignment(HorizontalAlignment::Right)
            .grid_column(1),
    ))
    .columns([GridLength::Auto, GridLength::Star(1.0)]);

    let content = vstack((header, chart)).spacing(4.0);

    if style.card {
        border(content)
            .background(ThemeRef::CardBackground)
            .border_brush(ThemeRef::CardStroke)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(8.0)
            .padding(Thickness::uniform(8.0))
            .into()
    } else {
        content.into()
    }
}
