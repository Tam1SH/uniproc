use guinea::widgets::chart::{line_chart_with_options, Interpolation, LineChartOptions, Series};
use guinea::widgets::color::hex;
use guinea_core::Load;
use windows_canvas::ColorF;
use windows_reactor::{
    border, text_block, vstack, Element, ElementExt, RenderCx, ThemeRef, Thickness,
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
}

pub fn metric_chart(
    cx: &mut RenderCx,
    kind: MetricChartKind,
    history: &Load<Vec<(u64, f32)>>,
    height: f64,
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
        show_grid: true,
        y_range: Some((0.0, 100.0)),
    };
    let chart = line_chart_with_options(cx, vec![series], |_| {}, options).height(height);

    let content = vstack((
        text_block(format!("{} {:.1}%", kind.title(), current))
            .font_size(table_styles::TABLE_STYLES.font_size),
        chart,
    ))
    .spacing(4.0);

    border(content)
        .background(ThemeRef::CardBackground)
        .border_brush(ThemeRef::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(8.0)
        .padding(Thickness::uniform(8.0))
        .into()
}
