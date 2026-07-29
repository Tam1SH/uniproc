use std::rc::Rc;

use amethystate::MapChange;
use app_contracts2::features::processes::{ProcessRow, ProcessesDispatch, ProcessesReducer};
use domain2::features::processes::settings::ColumnConfig;
use guinea::feature::FeatureInitContext;
use guinea::router::{Page, PageCx};
use guinea::uri::AppUri;
use guinea::widgets::table::{ColumnSpec, SortState, table};
use guinea_core::Load;
use guinea_core::signal::{Signal, SignalSubscription};
use guicons::icon;
use uuid::Uuid;
use windows_reactor::{
    border, button, hstack, text_block, title, vstack, Color, Element, ElementExt, SetState,
};

const SELECTED_ROW_BG: Color = Color { a: 48, r: 128, g: 128, b: 128 };
const TERMINATE_ICON_SIZE: f64 = 16.0;

pub struct Processes;

impl Page for Processes {
    fn install(ctx: &FeatureInitContext, _uri: &AppUri) -> anyhow::Result<()> {
        domain2::features::processes::install(ctx)
    }

    fn view(cx: &mut PageCx) -> Element {
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
                .icon(icon!(prohibited).size(TERMINATE_ICON_SIZE))
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

            let selected = state.selected;
            let layout = cx.use_ref(ColumnLayout::new());
            let layout = layout.borrow();
            let columns = build_columns(&layout, selected, dispatch.clone());

            table(
                cx,
                rows.clone(),
                columns,
                |r: &ProcessRow| r.pid.to_string(),
                Some((sort, on_sort)),
            )
            }
            Load::Failed(err) => text_block(format!("Failed to load processes: {err}")).into(),
            _ => text_block("Waiting for process data...").into(),
        };

        let status_bar = text_block(format!("Processes: {}", state.total()));

        vstack((header, body, status_bar)).spacing(16.0).into()
    }
}

fn column(
    id: &'static str,
    header: &'static str,
    width: Signal<u64>,
    selected: Option<u32>,
    dispatch: Rc<ProcessesDispatch>,
    fmt: impl Fn(&ProcessRow) -> String + 'static,
) -> ColumnSpec<ProcessRow> {
    ColumnSpec::new(id, header, width, move |row: &ProcessRow| {
        let is_selected = selected == Some(row.pid);
        let pid = row.pid;
        let dispatch = dispatch.clone();
        let cell = border(text_block(fmt(row)));
        let cell = if is_selected { cell.background(SELECTED_ROW_BG) } else { cell };
        cell.on_tapped(move || dispatch.emit_on_select(pid)).into()
    })
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

const COLUMN_IDS: &[&'static str] = &["name", "cpu", "memory", "disk", "net"];
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
    visible: Signal<bool>,
    _subs: (SignalSubscription, SignalSubscription, SignalSubscription),
}

impl ColumnLayout {
    fn new() -> Self {
        let settings = domain2::features::processes::settings::ProcessesSettings::new_with(
            &amethystate::global_store(),
        )
        .expect("processes settings must construct");
        let map = settings.columns().configs();
        let entries = COLUMN_IDS
            .iter()
            .map(|&id| ColumnLayoutEntry::new(id, &map))
            .collect();
        Self { entries }
    }
}

impl ColumnLayoutEntry {
    fn new<S: amethystate::Store>(
        id: &'static str,
        map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
    ) -> Self {
        let initial = map.get(&id.to_string()).ok().flatten().unwrap_or(COLUMN_DEFAULT);
        let width = Signal::new(initial.width);
        let visible = Signal::new(initial.visible);

        let ui_source = Uuid::new_v4();
        let store_source = Uuid::new_v4();

        // store -> ui
        let width_for_read = width.clone();
        let visible_for_read = visible.clone();
        let read = map.subscribe_key(id.to_string(), move |change| {
            let next = match change {
                MapChange::Insert { value, .. } | MapChange::Update { new_value: value, .. } => {
                    *value
                }
                MapChange::Remove { .. } | MapChange::Clear { .. } => COLUMN_DEFAULT,
            };
            width_for_read.set(next.width, Some(store_source));
            visible_for_read.set(next.visible, Some(store_source));
        });

        // ui width -> store
        let map_for_write_w = map.clone();
        let key_w = id.to_string();
        let width_for_write = width.clone();
        let write_w = width.subscribe_with_source(move |w, source| {
            if source != Some(store_source) {
                let current = map_for_write_w.get(&key_w).ok().flatten().unwrap_or(COLUMN_DEFAULT);
                let next = ColumnConfig { width: *w, ..current };
                let _ = map_for_write_w.set_or_create(key_w.clone(), &next);
                width_for_write.set(*w, Some(ui_source));
            }
        });

        // ui visible -> store
        let map_for_write_v = map.clone();
        let key_v = id.to_string();
        let visible_for_write = visible.clone();
        let write_v = visible.subscribe_with_source(move |v, source| {
            if source != Some(store_source) {
                let current = map_for_write_v.get(&key_v).ok().flatten().unwrap_or(COLUMN_DEFAULT);
                let next = ColumnConfig { visible: *v, ..current };
                let _ = map_for_write_v.set_or_create(key_v.clone(), &next);
                visible_for_write.set(*v, Some(ui_source));
            }
        });

        Self {
            id,
            width,
            visible,
            _subs: (read, write_w, write_v),
        }
    }
}

fn build_columns(
    layout: &ColumnLayout,
    selected: Option<u32>,
    dispatch: Rc<ProcessesDispatch>,
) -> Vec<ColumnSpec<ProcessRow>> {
    layout
        .entries
        .iter()
        .filter(|e| e.visible.get())
        .map(|e| build_column(e, selected, dispatch.clone()))
        .collect()
}

fn build_column(
    entry: &ColumnLayoutEntry,
    selected: Option<u32>,
    dispatch: Rc<ProcessesDispatch>,
) -> ColumnSpec<ProcessRow> {
    let width = entry.width.clone();
    match entry.id {
        "name" => column("name", "Name", width, selected, dispatch, |r| r.name.clone()),
        "cpu" => column("cpu", "CPU", width, selected, dispatch, |r| {
            format!("{:.1}%", r.cpu_percent)
        }),
        "memory" => column("memory", "Memory", width, selected, dispatch, |r| {
            format_bytes(r.memory_bytes)
        }),
        "disk" => column("disk", "Disk", width, selected, dispatch, |r| {
            format_bytes(r.disk_bytes)
        }),
        "net" => column("net", "Network", width, selected, dispatch, |r| {
            format_bytes(r.net_bytes)
        }),
        _ => unreachable!("unknown column id"),
    }
}
