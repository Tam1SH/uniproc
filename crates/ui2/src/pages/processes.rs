mod column_layout;
mod columns;
mod grouping;

use app_contracts2::features::agents::AgentConnectionState;
use app_contracts2::features::metrics::MetricsReducer;
use app_contracts2::features::processes::{ColumnConfig, ProcessCategory, ProcessesReducer};
use guicons::icon;
use guinea::router::PageCx;
use guinea::widgets::table::{table_with_sort_indicator, SortState};
use guinea_core::Load;
use std::collections::HashSet;
use std::rc::Rc;
use windows_reactor::{
    body_large, border, button, grid, hstack, text_block, Color, Element, ElementExt, GridLength,
    HorizontalAlignment, ProgressRing, SetState, Shape, Thickness, VerticalAlignment,
};

use crate::table_styles;
use column_layout::ColumnLayout;
use columns::{build_columns, sort_indicator_icon};
use grouping::{flatten_for_display, pin_display_row, GroupsCache};

fn disconnected_overlay() -> Element {

    border(
        hstack((
            ProgressRing::indeterminate().width(20.0).height(20.0),
            text_block("Connecting..."),
        ))
        .spacing(10.0),
    )
    .background(CARD_BACKGROUND)
    .corner_radius(8.0)
    .padding(Thickness::xy(16.0, 12.0))
    .horizontal_alignment(HorizontalAlignment::Center)
    .vertical_alignment(VerticalAlignment::Top)
    .margin(Thickness::xy(0.0, 12.0))
    .into()
}

const CARD_BACKGROUND: Color = Color { a: 240, r: 32, g: 32, b: 32 };

pub fn processes_view<S: amethystate::Store>(
    cx: &mut PageCx,
    map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
) -> Element {
    let (state, dispatch) = cx.use_reducer::<ProcessesReducer>();
    let (metrics_state, _) = cx.use_reducer::<MetricsReducer>();

    let layout = cx.use_ref(ColumnLayout::new(map));
    let icons = cx.use_ref(context2::IconCache::new());
    let (expanded, set_expanded) = cx.use_state(HashSet::<String>::new());
    let (collapsed_sections, set_collapsed_sections) =
        cx.use_state(HashSet::<ProcessCategory>::new());
    let groups_cache = cx.use_ref(GroupsCache::empty());
    let pinned_display_pos = cx.use_ref(Option::<usize>::None);

    let selected_name = state
        .selected
        .and_then(|pid| state.rows().iter().find(|r| r.pid == pid))
        .map(|r| r.name.clone());

    let terminate_dispatch = dispatch.clone();
    let header = hstack((
        body_large("Processes").padding(12.0),
        text_block(selected_name.unwrap_or_default()),
        button("End task")
            .icon(icon!(prohibited).size(table_styles::TABLE_STYLES.terminate_icon_size))
            .enabled(state.selected.is_some())
            .on_click(move || terminate_dispatch.emit_on_terminate()),
    ))
    .spacing(12.0);

    let cpu_detail = metrics_state.machine.ready().map(|m| {
        format!(
            "{:.1} / {:.1} GHz",
            m.cpu_current_mhz as f64 / 1000.0,
            m.cpu_max_mhz as f64 / 1000.0
        )
    });

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
                let expanded = expanded.clone();
                SetState::new(move |name: String| {
                    let mut next = expanded.clone();
                    if !next.remove(&name) {
                        next.insert(name);
                    }
                    set_expanded.call(next);
                })
            };

            let mut groups_cache = groups_cache.borrow_mut();
            let sections = groups_cache.get(rows, state.selected);
            let toggle_section = {
                let collapsed = collapsed_sections.clone();
                SetState::new(move |category: ProcessCategory| {
                    let mut next = collapsed.clone();
                    if !next.remove(&category) {
                        next.insert(category);
                    }
                    set_collapsed_sections.call(next);
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
        Load::Failed(err) => text_block(format!("Failed to load processes: {err}")).into(),
        _ => ProgressRing::indeterminate()
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };

    let body_separator = separator();
    let status_bar = text_block(format!("Processes: {}", state.total())).padding(8.0);

    let body = if matches!(state.agent_state, AgentConnectionState::Connected) {
        body
    } else {
        grid((body.grid_row(0), disconnected_overlay().grid_row(0)))
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

fn separator() -> Element {
    Shape::rectangle()
        .fill(table_styles::TABLE_STYLES.separator_color)
        .height(1.0)
        .into()
}
