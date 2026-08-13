use app_contracts::features::wsl::{DistroRow, WslReducer};
use guinea::router::PageCx;
use guinea::widgets::table::table;
use guinea_core::Load;
use windows_reactor::{
    body_large, border, grid, hstack, text_block, Element, ElementExt, GridLength,
    HorizontalAlignment, ProgressRing, VerticalAlignment,
};

use crate::l10n::use_tr;
use crate::theme::space;
use crate::widgets::separator;
use super::components::columns::build_columns;

pub fn wsl_view(cx: &mut PageCx) -> Element {
    let l10n = use_tr(cx);
    let (state, _dispatch) = cx.use_reducer::<WslReducer>();

    let header = hstack((body_large(l10n.wsl_title()).padding(space::Header),))
        .spacing(space::Header);

    let body: Element = match &state.distros {
        Load::Ready(rows) => table(
            cx,
            rows.to_vec(),
            build_columns(&l10n),
            |row: &DistroRow| row.name.clone(),
            None,
            None,
        ),
        Load::Failed(err) => text_block(l10n.wsl_failed(err.to_string())).into(),
        _ => ProgressRing::indeterminate()
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };

    let status_bar = text_block(l10n.wsl_status(state.total() as i64, state.running() as i64))
        .padding(space::Control);

    grid((
        header.grid_row(0),
        border(body).grid_row(1),
        separator().grid_row(2),
        status_bar.grid_row(3),
    ))
    .rows([
        GridLength::Auto,
        GridLength::Star(1.0),
        GridLength::Auto,
        GridLength::Auto,
    ])
    .into()
}
