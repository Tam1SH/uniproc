use amethystate::MapChange;
use app_contracts2::features::processes::{ColumnConfig, MachineSummary, ProcessRow, ProcessesReducer};
use guicons::icon;
use guinea::router::PageCx;
use guinea::widgets::table::{table, ColumnSpec, SortState};
use guinea_core::signal::{Signal, SignalSubscription};
use guinea_core::Load;
use uuid::Uuid;
use windows_reactor::{
    button, grid, hstack, text_block, title, vstack, Element, ElementExt, GridLength, ProgressBar,
    SetState, Shape, ThemeRef,
};

use crate::table_styles;

pub fn processes_view<S: amethystate::Store>(
    cx: &mut PageCx,
    map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
) -> Element {
        let (state, dispatch) = cx.use_reducer::<ProcessesReducer>();

        let selected_name = state
            .selected
            .and_then(|pid| state.rows().iter().find(|r| r.pid == pid))
            .map(|r| r.name.clone());

        let terminate_dispatch = dispatch.clone();
        let header = hstack((
            title("Processes"),
            text_block(selected_name.unwrap_or_default()),
            button("End task")
                .icon(icon!(prohibited).size(table_styles::TABLE_STYLES.terminate_icon_size))
                .enabled(state.selected.is_some())
                .on_click(move || terminate_dispatch.emit_on_terminate()),
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

                let machine = state.machine_summary().cloned();
                let layout = cx.use_ref(ColumnLayout::new(map));
                let layout = layout.borrow();
                let columns = build_columns(&layout, machine);

                let selected_index = state
                    .selected
                    .and_then(|pid| rows.iter().position(|r| r.pid == pid))
                    .map(|i| i as i32)
                    .unwrap_or(-1);
                let rows_for_select = rows.clone();
                let select_dispatch = dispatch.clone();
                let on_selection_changed = SetState::new(move |idx: i32| {
                    if idx >= 0
                        && let Some(row) = rows_for_select.get(idx as usize)
                    {
                        select_dispatch.emit_on_select(row.pid);
                    }
                });

                table(
                    cx,
                    rows.clone(),
                    columns,
                    |r: &ProcessRow| r.pid.to_string(),
                    Some((sort, on_sort)),
                    Some((selected_index, on_selection_changed)),
                )
            }
            Load::Failed(err) => text_block(format!("Failed to load processes: {err}")).into(),
            _ => text_block("Waiting for process data...").into(),
        };

        let body_separator = separator();
        let status_bar = text_block(format!("Processes: {}", state.total()))
            .padding(windows_reactor::Thickness::xy(0.0, 8.0));

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
        .row_spacing(16.0)
        .into()
}

fn separator() -> Element {
    Shape::rectangle()
        .fill(table_styles::TABLE_STYLES.separator_color)
        .height(1.0)
        .into()
}

fn column(
    id: &'static str,
    header: &'static str,
    width: Signal<u64>,
    min_width: f64,
    fmt: impl Fn(&ProcessRow) -> String + 'static,
) -> ColumnSpec<ProcessRow> {
    ColumnSpec::new(id, header, width, move |row: &ProcessRow| {
        table_styles::TABLE_STYLES.text_cell(fmt(row))
    })
    .min_width(min_width)
    .sortable()
}

fn cpu_column(
    width: Signal<u64>,
    min_width: f64,
    machine: Option<MachineSummary>,
) -> ColumnSpec<ProcessRow> {
    let header = move || {
        machine
            .as_ref()
            .map(|m| {
                let subtitle = text_block(format!(
                    "{:.1} / {:.1} GHz",
                    m.cpu_current_mhz as f64 / 1000.0,
                    m.cpu_max_mhz as f64 / 1000.0
                ))
                .foreground(ThemeRef::TertiaryText);
                let right = vstack((
                    text_block(format!("{:.1}%", m.cpu_percent)),
                    text_block("CPU"),
                ))
                .spacing(2.0);
                vstack((
                    hstack((subtitle, right)).spacing(6.0),
                    Element::from(ProgressBar::new(m.cpu_percent as f64).range(0.0, 100.0)),
                ))
                .spacing(2.0)
                .into()
            })
            .unwrap_or_else(|| text_block("CPU").into())
    };
    ColumnSpec::new_with_header("cpu", header, width, move |row: &ProcessRow| {
        table_styles::TABLE_STYLES.text_cell(format!("{:.1}%", row.cpu_percent))
    })
    .min_width(min_width)
    .sortable()
}

fn memory_column(
    width: Signal<u64>,
    min_width: f64,
    machine: Option<MachineSummary>,
) -> ColumnSpec<ProcessRow> {
    let header = move || {
        machine
            .as_ref()
            .map(|m| {
                let percent = if m.memory_total_bytes > 0 {
                    (m.memory_used_bytes as f64 / m.memory_total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                let subtitle = text_block(format!(
                    "{} / {}",
                    format_bytes(m.memory_used_bytes),
                    format_bytes(m.memory_total_bytes)
                ))
                .foreground(ThemeRef::TertiaryText);
                let right = vstack((text_block(format!("{:.1}%", percent)), text_block("Memory")))
                    .spacing(2.0);
                vstack((
                    hstack((subtitle, right)).spacing(6.0),
                    Element::from(ProgressBar::new(percent).range(0.0, 100.0)),
                ))
                .spacing(2.0)
                .into()
            })
            .unwrap_or_else(|| text_block("Memory").into())
    };
    ColumnSpec::new_with_header("memory", header, width, move |row: &ProcessRow| {
        table_styles::TABLE_STYLES.text_cell(format_bytes(row.memory_bytes))
    })
    .min_width(min_width)
    .sortable()
}

fn format_bytes(v: u64) -> String {
    const KIB: f64 = 1024.0;
    let f = v as f64;
    if f >= KIB.powi(3) {
        format!("{:.1} GiB", f / KIB.powi(3))
    } else if f >= KIB.powi(2) {
        format!("{:.1} MiB", f / KIB.powi(2))
    } else if f >= KIB {
        format!("{:.0} KiB", f / KIB)
    } else {
        format!("{v} B")
    }
}

const COLUMN_IDS: &[&'static str] = &["name", "cpu", "memory", "net", "disk"];
const COLUMN_DEFAULT: ColumnConfig = ColumnConfig {
    width: 110,
    min_width: 80,
    visible: true,
};

/// Persisted column state as live cells over the `processes.columns` ReactiveMap
/// entries: widths are two-way synced with the table, visibility filters which
/// columns render. Held in a `use_ref` so the sync subscriptions live as long as
/// the page.
struct ColumnLayout {
    entries: Vec<ColumnLayoutEntry>,
}

struct ColumnLayoutEntry {
    id: &'static str,
    width: Signal<u64>,
    min_width: Signal<u64>,
    visible: Signal<bool>,
    _subs: (SignalSubscription, SignalSubscription, SignalSubscription),
}

impl ColumnLayout {
    fn new<S: amethystate::Store>(
        map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
    ) -> Self {
        let entries = COLUMN_IDS
            .iter()
            .map(|&id| ColumnLayoutEntry::new(id, map))
            .collect();
        Self { entries }
    }
}

impl ColumnLayoutEntry {
    fn new<S: amethystate::Store>(
        id: &'static str,
        map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
    ) -> Self {
        let initial = map
            .get(&id.to_string())
            .ok()
            .flatten()
            .unwrap_or(COLUMN_DEFAULT);
        let width = Signal::new(initial.width);
        let min_width = Signal::new(initial.min_width);
        let visible = Signal::new(initial.visible);

        let store_source = Uuid::new_v4();

        // store -> ui
        let width_for_read = width.clone();
        let min_width_for_read = min_width.clone();
        let visible_for_read = visible.clone();
        let read = map.subscribe_key(id.to_string(), move |change| {
            let next = match change {
                MapChange::Insert { value, .. }
                | MapChange::Update {
                    new_value: value, ..
                } => *value,
                MapChange::Remove { .. } | MapChange::Clear { .. } => COLUMN_DEFAULT,
            };
            width_for_read.set(next.width, Some(store_source));
            min_width_for_read.set(next.min_width, Some(store_source));
            visible_for_read.set(next.visible, Some(store_source));
        });

        // ui width -> store
        let map_for_write_w = map.clone();
        let key_w = id.to_string();
        let write_w = width.subscribe_with_source(move |w, source| {
            if source != Some(store_source) {
                let current = map_for_write_w
                    .get(&key_w)
                    .ok()
                    .flatten()
                    .unwrap_or(COLUMN_DEFAULT);
                let next = ColumnConfig {
                    width: *w,
                    ..current
                };
                let _ = map_for_write_w.set_or_create(key_w.clone(), &next);
            }
        });

        // ui visible -> store
        let map_for_write_v = map.clone();
        let key_v = id.to_string();
        let write_v = visible.subscribe_with_source(move |v, source| {
            if source != Some(store_source) {
                let current = map_for_write_v
                    .get(&key_v)
                    .ok()
                    .flatten()
                    .unwrap_or(COLUMN_DEFAULT);
                let next = ColumnConfig {
                    visible: *v,
                    ..current
                };
                let _ = map_for_write_v.set_or_create(key_v.clone(), &next);
            }
        });

        Self {
            id,
            width,
            min_width,
            visible,
            _subs: (read, write_w, write_v),
        }
    }
}

fn build_columns(
    layout: &ColumnLayout,
    machine: Option<MachineSummary>,
) -> Vec<ColumnSpec<ProcessRow>> {
    layout
        .entries
        .iter()
        .filter(|e| e.visible.get())
        .map(|e| build_column(e, machine.clone()))
        .collect()
}

fn build_column(
    entry: &ColumnLayoutEntry,
    machine: Option<MachineSummary>,
) -> ColumnSpec<ProcessRow> {
    let width = entry.width.clone();
    let min_width = entry.min_width.get() as f64;
    match entry.id {
        "name" => column("name", "Name", width, min_width, |r| r.name.clone()),
        "cpu" => cpu_column(width, min_width, machine),
        "memory" => memory_column(width, min_width, machine),
        "net" => column("net", "Net", width, min_width, |r| format_bytes(r.net_bytes)),
        "disk" => column("disk", "Disk", width, min_width, |r| format_bytes(r.disk_bytes)),
        _ => unreachable!("unknown column id"),
    }
}
