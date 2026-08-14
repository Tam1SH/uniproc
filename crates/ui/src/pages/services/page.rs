use app_contracts::features::services::{
    Command, Deselect, Select, ServiceActionKind, ServiceRow, ServicesReducer, Sort,
};
use guinea::router::PageCx;
use guinea::widgets::table::{table_with_sort_indicator, SortState};
use guinea_core::Load;
use windows_reactor::{
    body_large, border, button, grid, hstack, text_block, Element, ElementExt, GridLength,
    HorizontalAlignment, ProgressRing, SetState, VerticalAlignment,
};

use crate::l10n::use_tr;
use crate::theme::space;
use crate::widgets::separator;
use super::components::columns::build_columns;

pub fn services_view(cx: &mut PageCx) -> Element {
    let l10n = use_tr(cx);
    let (state, dispatch) = cx.use_reducer::<ServicesReducer>();

    let has_selection = state.selected.is_some();
    let start_dispatch = dispatch.clone();
    let stop_dispatch = dispatch.clone();
    let restart_dispatch = dispatch.clone();

    let header = hstack((
        body_large(l10n.services_title()).padding(space::Header),
        button(l10n.services_start())
            .enabled(has_selection)
            .on_click(move || start_dispatch.emit(Command(ServiceActionKind::Start))),
        button(l10n.services_stop())
            .enabled(has_selection)
            .on_click(move || stop_dispatch.emit(Command(ServiceActionKind::Stop))),
        button(l10n.services_restart())
            .enabled(has_selection)
            .on_click(move || restart_dispatch.emit(Command(ServiceActionKind::Restart))),
    ))
    .spacing(space::Header);

    let body: Element = match &state.rows {
        Load::Ready(rows) => {
            let sort = SortState {
                field_id: Some(state.sort_column.clone()),
                descending: state.descending,
            };
            let sort_dispatch = dispatch.clone();
            let on_sort = SetState::new(move |col: String| sort_dispatch.emit(Sort(col)));

            let rows: Vec<ServiceRow> = rows.to_vec();

            let selected_index = state
                .selected
                .as_deref()
                .and_then(|name| rows.iter().position(|r| r.name == name))
                .map(|i| i as i32)
                .unwrap_or(-1);
            let names_for_select: Vec<String> = rows.iter().map(|r| r.name.clone()).collect();
            let select_dispatch = dispatch.clone();
            let on_selection_changed = SetState::new(move |idx: i32| {
                if idx >= 0
                    && let Some(name) = names_for_select.get(idx as usize)
                {
                    select_dispatch.emit(Select(name.clone()));
                }
            });

            table_with_sort_indicator(
                cx,
                rows,
                build_columns(&l10n),
                |r: &ServiceRow| r.name.clone(),
                Some((sort, on_sort)),
                Some((selected_index, on_selection_changed)),
                None,
            )
        }
        Load::Failed(err) => text_block(l10n.services_failed(err.to_string())).into(),
        _ => ProgressRing::indeterminate()
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };

    let status_bar = text_block(l10n.services_status(state.total() as i64)).padding(space::Control);
    let body = border(body).on_tapped(|| {});
    let deselect_dispatch = dispatch.clone();

    grid((
        header.grid_row(0),
        body.grid_row(1),
        separator().grid_row(2),
        status_bar.grid_row(3),
    ))
    .rows([
        GridLength::Auto,
        GridLength::Star(1.0),
        GridLength::Auto,
        GridLength::Auto,
    ])
    .on_tapped(move || deselect_dispatch.emit(Deselect))
    .into()
}
