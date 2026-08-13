use app_contracts::features::agents::AgentConnectionState;
use app_contracts::features::processes::{ColumnConfig, ProcessCategory, ProcessesReducer};
use guicons::icon;
use guinea::router::PageCx;
use guinea::widgets::table::{table_with_sort_indicator, SortState};
use guinea_core::Load;
use std::collections::HashSet;
use std::rc::Rc;
use windows_reactor::{
    body_large, border, button, grid, hstack, text_block, Element, ElementExt, GridLength,
    HorizontalAlignment, ProgressRing, SetState, VerticalAlignment,
};

use crate::l10n::use_tr;
use crate::theme::{size, space};
use crate::widgets::separator;
use super::components::column_layout::ColumnLayout;
use super::components::columns::{build_columns, sort_indicator_icon};
use super::components::grouping::{self, flatten_for_display, pin_display_row, GroupsCache};
use super::components::overlay::disconnected_overlay;

pub fn processes_view<S: amethystate::Store>(
    cx: &mut PageCx,
    map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
    expanded_groups: &amethystate::ReactiveMap<String, bool, S, amethystate::WritableMode>,
    collapsed_sections_store: &amethystate::ReactiveMap<String, bool, S, amethystate::WritableMode>,
) -> Element {
    let l10n = use_tr(cx);
    let scheme = cx.use_color_scheme();
    let (state, dispatch) = cx.use_reducer::<ProcessesReducer>();

    let layout = cx.use_ref(ColumnLayout::new(map));
    let icons = cx.use_ref(context::IconCache::new());
    let (revision, bump_revision) = cx.use_state(0u64);
    let _ = revision;

    let expanded: HashSet<String> = expanded_groups
        .entries()
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, on)| *on)
        .map(|(name, _)| name)
        .collect();

    let collapsed_sections: HashSet<ProcessCategory> = collapsed_sections_store
        .entries()
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, on)| *on)
        .filter_map(|(id, _)| ProcessCategory::from_id(&id))
        .collect();
    let groups_cache = cx.use_ref(GroupsCache::empty());
    let pinned_display_pos = cx.use_ref(Option::<usize>::None);

    let selected_name = state
        .selected
        .and_then(|pid| state.rows().iter().find(|r| r.pid == pid))
        .map(|r| r.name.clone());

    let terminate_dispatch = dispatch.clone();
    let header = hstack((
        body_large(l10n.processes_title()).padding(space::Header),
        text_block(selected_name.unwrap_or_default()),
        button(l10n.processes_end_task())
            .icon(icon!(prohibited).size(size::Icon))
            .enabled(state.selected.is_some())
            .on_click(move || terminate_dispatch.emit_on_terminate()),
    ))
    .spacing(space::Header);

    let body: Element = match &state.rows {
        Load::Ready(rows) => {
            let sort = SortState {
                field_id: Some(state.sort_column.clone()),
                descending: state.descending,
            };
            let sort_dispatch = dispatch.clone();
            let on_sort = SetState::new(move |col: String| sort_dispatch.emit_on_sort(col));

            let machine = state.machine_summary().cloned();
            let layout = layout.borrow();

            let toggle_expanded = {
                let store = expanded_groups.clone();
                let expanded = expanded.clone();
                let bump = bump_revision.clone();
                SetState::new(move |name: String| {
                    let now_on = !expanded.contains(&name);
                    let result = if now_on {
                        store.set_or_create(name.clone(), &true)
                    } else {
                        store.remove(name.clone()).map(|_| ())
                    };
                    if let Err(err) = result {
                        tracing::warn!(%name, ?err, "expanded_groups write failed");
                    }
                    bump.call(revision + 1);
                })
            };

            let mut groups_cache = groups_cache.borrow_mut();
            let sections = groups_cache.get(rows, state.selected);
            let toggle_section = {
                let store = collapsed_sections_store.clone();
                let collapsed = collapsed_sections.clone();
                let bump = bump_revision.clone();
                SetState::new(move |category: ProcessCategory| {
                    let label = category.id().to_string();
                    let result = if collapsed.contains(&category) {
                        store.remove(label.clone()).map(|_| ())
                    } else {
                        store.set_or_create(label.clone(), &true)
                    };
                    if let Err(err) = result {
                        tracing::warn!(%label, ?err, "collapsed_sections write failed");
                    }
                    bump.call(revision + 1);
                })
            };

            let mut display_rows =
                flatten_for_display(sections, &expanded, &collapsed_sections);
            let columns = build_columns(
                &layout,
                machine,
                rows,
                icons.clone(),
                toggle_expanded,
                toggle_section,
                scheme,
                &l10n,
            );

            let prev_pos = *pinned_display_pos.borrow();
            let new_pos = pin_display_row(&mut display_rows, state.selected, prev_pos);
            *pinned_display_pos.borrow_mut() = new_pos;

            let selected_index = new_pos.map(|i| i as i32).unwrap_or(-1);
            let pids_for_select: Vec<u32> = display_rows.iter().map(|d| d.row.pid).collect();
            let select_dispatch = dispatch.clone();
            let on_selection_changed = SetState::new(move |idx: i32| {
                if idx >= 0
                    && let Some(&pid) = pids_for_select.get(idx as usize)
                {
                    select_dispatch.emit_on_select(pid);
                }
            });

            table_with_sort_indicator(
                cx,
                display_rows,
                columns,
                |d: &grouping::DisplayRow| d.row.pid.to_string(),
                Some((sort, on_sort)),
                Some((selected_index, on_selection_changed)),
                Some(Rc::new(sort_indicator_icon)),
            )
        }
        Load::Failed(err) => text_block(l10n.processes_failed(err.to_string())).into(),
        _ => ProgressRing::indeterminate()
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };

    let body_separator = separator();
    let status_bar = text_block(l10n.processes_status(state.total() as i64)).padding(space::Control);

    let body = if matches!(state.agent_state, AgentConnectionState::Connected) {
        body
    } else {
        grid((body.grid_row(0), disconnected_overlay(&l10n).grid_row(0)))
            .rows([GridLength::Star(1.0)])
            .into()
    };
    let body = border(body).on_tapped(|| {});
    let deselect_dispatch = dispatch.clone();

    grid((
        header.grid_row(0),
        body.grid_row(1),
        body_separator.grid_row(2),
        status_bar.grid_row(3),
    ))
    .rows([
        GridLength::Auto,
        GridLength::Star(1.0),
        GridLength::Auto,
        GridLength::Auto,
    ])
    .on_tapped(move || deselect_dispatch.emit_on_deselect())
    .into()
}
