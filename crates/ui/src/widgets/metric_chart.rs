use guinea::widgets::chart::{line_chart_with_options, Interpolation, LineChartOptions, Series};
use guinea::widgets::color::hex;
use guinea_core::Load;
use windows_canvas::ColorF;
use windows_reactor::{
    border, caption, grid, hstack, vstack, Color, ColorScheme, Element, ElementExt, GridLength,
    HorizontalAlignment, RenderCx, Thickness, VerticalAlignment, tokens,
};

use crate::l10n::tr;
use crate::theme::{radius, space, Palette};

#[derive(Clone, Copy, Debug)]
pub enum MetricChartKind {
    Cpu,
    Memory,
}

impl MetricChartKind {
    pub fn title(&self) -> String {
        match self {
            Self::Cpu => tr().metric_chart_cpu(),
            Self::Memory => tr().metric_chart_memory(),
        }
    }

    pub fn color(&self) -> ColorF {
        match self {
            Self::Cpu => hex(0x60a5fa),
            Self::Memory => hex(0x34d399),
        }
    }

    fn color_direct(&self) -> Color {
        match self {
            Self::Cpu => Color { a: 255, r: 0x60, g: 0xa5, b: 0xfa },
            Self::Memory => Color { a: 255, r: 0x34, g: 0xd3, b: 0x99 },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MetricChartStyle {
    pub show_grid: bool,
    pub card: bool,
}

impl MetricChartStyle {
    pub const CARD: Self = Self { show_grid: true, card: true };
    pub const SPARKLINE: Self = Self { show_grid: false, card: false };
}

const MINI_BAR_WIDTH: f64 = 6.0;
const MINI_BAR_HEIGHT: f64 = 34.0;

const WARNING_THRESHOLD: f32 = 65.0;
const CRITICAL_THRESHOLD: f32 = 90.0;
const CRITICAL_FILL_THRESHOLD: f32 = 95.0;

const WARNING_COLOR: Color = Color { a: 255, r: 0xFF, g: 0xB9, b: 0x00 };
const CRITICAL_COLOR: Color = Color { a: 255, r: 0xE7, g: 0x48, b: 0x56 };
const TRACK_WARNING_COLOR: Color = Color { a: 130, ..WARNING_COLOR };
const TRACK_CRITICAL_COLOR: Color = Color { a: 130, ..CRITICAL_COLOR };

pub fn metric_mini_bar(
    scheme: ColorScheme,
    kind: MetricChartKind,
    history: &Load<Vec<(u64, f32)>>,
) -> Element {
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
        Palette::of(scheme).track_idle
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
        background: None,
        border: None,
        show_grid: style.show_grid,
        y_range: Some((0.0, 100.0)),
    };
    let chart = line_chart_with_options(cx, vec![series], |_| {}, options).height(height);

    let label: Element = match detail {
        Some(detail) => hstack((
            caption(kind.title()),
            caption(detail).foreground(tokens::TertiaryText),
        ))
        .spacing(space::Control)
        .into(),
        None => caption(kind.title()).into(),
    };
    let header = grid((
        label.grid_column(0),
        caption(format!("{current:.1}%"))
            .foreground(tokens::SecondaryText)
            .horizontal_alignment(HorizontalAlignment::Right)
            .grid_column(1),
    ))
    .columns([GridLength::Auto, GridLength::Star(1.0)]);

    let content = vstack((header, chart)).spacing(space::Compact);

    if style.card {
        border(content)
            .background(tokens::CardBackground)
            .border_brush(tokens::CardStroke)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(radius::Overlay)
            .padding(Thickness::uniform(space::Control))
            .into()
    } else {
        content.into()
    }
}
