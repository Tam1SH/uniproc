use windows_reactor::{
    border, hstack, text_block, tokens, Element, ElementExt, HorizontalAlignment, ProgressRing,
    Thickness, VerticalAlignment,
};

use crate::l10n::L10n;
use crate::theme::{radius, size, space};

pub(crate) fn disconnected_overlay(l10n: &L10n) -> Element {
    border(
        hstack((
            ProgressRing::indeterminate()
                .width(size::NavIcon)
                .height(size::NavIcon),
            text_block(l10n.processes_connecting()),
        ))
        .spacing(space::Header),
    )
    .background(tokens::LayerFill)
    .corner_radius(radius::Overlay)
    .padding(Thickness::xy(space::Card, space::Header))
    .horizontal_alignment(HorizontalAlignment::Center)
    .vertical_alignment(VerticalAlignment::Top)
    .margin(Thickness::xy(0.0, space::Header))
    .into()
}
