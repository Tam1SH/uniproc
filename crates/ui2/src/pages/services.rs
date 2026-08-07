use app_contracts2::features::services::{ServiceActionKind, ServiceRow, ServicesReducer};
use guicons::icon;
use guinea::router::PageCx;
use guinea::widgets::table::{ColumnSpec, SortState, table_with_sort_indicator};
use guinea_core::Load;
use windows_reactor::{
    body_large, border, button, grid, hstack, text_block, Color, Element, ElementExt,
    GridLength, HorizontalAlignment, ProgressRing, SetState, Shape, TextTrimming,
    VerticalAlignment,
};

const RUNNING_GREEN: Color = Color {
    a: 255,
    r: 32,
    g: 166,
    b: 79,
};

/// Generic per-row service icon (a gear, matching the old Slint app's
/// service-list glyph) - services don't have a per-process exe/package to
/// extract a real icon from, so unlike `processes`' `NameCell` there's no
/// lookup here, just this one fixed icon for every row.
fn service_icon() -> Element {
    icon!(gears).size(16.0).build_element()
}

/// Stopped services are the common/expected state, not an error - dimming
/// (not recoloring) each cell keeps the row readable while visually
/// receding behind whatever's actually running. Applied per-cell, not once
/// on the row: `table_with_sort_indicator`'s row is assembled internally
/// from each column's own cell closure, there's no single row `Element` a
/// caller here gets to wrap.
fn maybe_dim(el: Element, running: bool) -> Element {
    if running { el } else { el.opacity(0.55) }
}

pub fn services_view(cx: &mut PageCx) -> Element {
    let (state, dispatch) = cx.use_reducer::<ServicesReducer>();

    let has_selection = state.selected.is_some();
    let start_dispatch = dispatch.clone();
    let stop_dispatch = dispatch.clone();
    let restart_dispatch = dispatch.clone();

    let header = hstack((
        body_large("Services").padding(12.0),
        button("Start")
            .enabled(has_selection)
            .on_click(move || start_dispatch.emit_on_command(ServiceActionKind::Start)),
        button("Stop")
            .enabled(has_selection)
            .on_click(move || stop_dispatch.emit_on_command(ServiceActionKind::Stop)),
        button("Restart")
            .enabled(has_selection)
            .on_click(move || restart_dispatch.emit_on_command(ServiceActionKind::Restart)),
    ))
    .spacing(12.0);

    let body: Element = match &state.rows {
        Load::Ready(rows) => {
            let sort = SortState {
                field_id: Some(state.sort_column.clone()),
                descending: state.descending,
            };
            let sort_dispatch = dispatch.clone();
            let on_sort = SetState::new(move |col: String| sort_dispatch.emit_on_sort(col));

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
                    select_dispatch.emit_on_select(name.clone());
                }
            });

            let columns = vec![
                ColumnSpec::new("name", "Name", 260u64, |row: &ServiceRow| {
                    let content = hstack((
                        service_icon(),
                        text_block(row.display_name.clone()).text_trimming(TextTrimming::CharacterEllipsis),
                    ))
                    .spacing(6.0);
                    maybe_dim(content.into(), row.status == "Running")
                })
                .sortable(),
                ColumnSpec::new("status", "Status", 90u64, |row: &ServiceRow| {
                    let running = row.status == "Running";
                    let el: Element = if running {
                        text_block(row.status.clone())
                            .text_trimming(TextTrimming::CharacterEllipsis)
                            .foreground(RUNNING_GREEN)
                            .into()
                    } else {
                        text_block(row.status.clone()).text_trimming(TextTrimming::CharacterEllipsis).into()
                    };
                    maybe_dim(el, running)
                })
                .sortable(),
                ColumnSpec::new("pid", "PID", 70u64, |row: &ServiceRow| {
                    let text = if row.pid == 0 { String::new() } else { row.pid.to_string() };
                    maybe_dim(text_block(text).into(), row.status == "Running")
                })
                .sortable(),
                ColumnSpec::new("group", "Group", 120u64, |row: &ServiceRow| {
                    maybe_dim(
                        text_block(row.group.clone()).text_trimming(TextTrimming::CharacterEllipsis).into(),
                        row.status == "Running",
                    )
                })
                .sortable(),
                ColumnSpec::new("description", "Description", 320u64, |row: &ServiceRow| {
                    maybe_dim(
                        text_block(row.description.clone()).text_trimming(TextTrimming::CharacterEllipsis).into(),
                        row.status == "Running",
                    )
                }),
            ];

            table_with_sort_indicator(
                cx,
                rows,
                columns,
                |r: &ServiceRow| r.name.clone(),
                Some((sort, on_sort)),
                Some((selected_index, on_selection_changed)),
                None,
            )
        }
        Load::Failed(err) => text_block(format!("Failed to load services: {err}")).into(),
        _ => ProgressRing::indeterminate()
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };

    let body_separator: Element = Shape::rectangle().fill(Color { a: 48, r: 128, g: 128, b: 128 }).height(1.0).into();
    let status_bar = text_block(format!("Services: {}", state.total())).padding(8.0);
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
