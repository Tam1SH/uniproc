use amethystate::MapChange;
use app_contracts2::features::metrics::MetricsReducer;
use app_contracts2::features::processes::{
    ColumnConfig, MachineSummary, ProcessRow, ProcessesReducer,
};
use guicons::icon;
use guinea::router::PageCx;
use guinea::widgets::table::{table_with_sort_indicator, ColumnSpec, SortState};
use guinea_core::signal::{Signal, SignalSubscription};
use guinea_core::Load;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use uuid::Uuid;
use windows_reactor::{
    body_large, border, button, grid, hstack, text_block, Color, Element,
    ElementExt, GridLength, HookRef, HorizontalAlignment, Icon, Image, ProgressRing, SetState, Shape,
    ThemeRef, VerticalAlignment,
};

use crate::table_styles;

/// A family of same-named processes with their metrics summed - a fact
/// about the system, independent of how (or whether) it gets rendered as a
/// collapsible row. No `depth`/`is_expanded`/chevron concept belongs here;
/// those exist only because of how a table widget draws a group, which is
/// [`flatten_for_display`]'s job, not this one's.
struct ProcessGroup {
    /// Identity fields (pid/name/exe_path/package_full_name) come from the
    /// leader; CPU/memory/disk/net are summed across every member.
    leader: ProcessRow,
    /// All members, including the leader at `[0]`.
    members: Vec<ProcessRow>,
}

/// Groups `rows` (already sorted by the active column) by process name.
/// Group order follows each group's first-seen position in `rows`, not a
/// separate alphabetical pass - the old Slint app's `ProcessTreeBuilder`
/// always regrouped alphabetically regardless of the active sort, which
/// would fight the table's own per-column sorting here.
///
/// `leader_pid`, if it names a member of a group, is promoted to that
/// group's leader instead of the default lowest-pid choice - otherwise
/// selecting (or pinning) any instance but the lowest-pid one would vanish
/// from view the instant its group collapses in the UI, since only the
/// leader row renders at all when collapsed.
fn group_by_name(rows: &[ProcessRow], leader_pid: Option<u32>) -> Vec<ProcessGroup> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<ProcessRow>> = HashMap::new();
    for row in rows {
        groups
            .entry(row.name.clone())
            .or_insert_with(|| {
                order.push(row.name.clone());
                Vec::new()
            })
            .push(row.clone());
    }

    order
        .into_iter()
        .filter_map(|name| {
            let mut members = groups.remove(&name)?;
            if members.is_empty() {
                return None;
            }
            // Base order is lowest-pid-first, stable across snapshots as
            // long as that process stays alive; the requested leader (if a
            // member) then takes the slot on top of that.
            members.sort_unstable_by_key(|r| r.pid);
            let leader_idx = leader_pid
                .and_then(|pid| members.iter().position(|r| r.pid == pid))
                .unwrap_or(0);
            members.swap(0, leader_idx);

            let leader = if members.len() > 1 {
                ProcessRow {
                    cpu_percent: members.iter().map(|r| r.cpu_percent).sum(),
                    memory_bytes: members.iter().map(|r| r.memory_bytes).sum(),
                    disk_bytes: members.iter().map(|r| r.disk_bytes).sum(),
                    net_bytes: members.iter().map(|r| r.net_bytes).sum(),
                    ..members[0].clone()
                }
            } else {
                members[0].clone()
            };

            Some(ProcessGroup { leader, members })
        })
        .collect()
}

/// Flattens `groups` into the single sequence the table actually renders:
/// leaders at `depth: 0`, members of any group named in `expanded`
/// following at `depth: 1`. Purely presentation - `depth`/`is_expanded`/
/// `has_children` exist because `guinea::widgets::table` wants a flat
/// `Vec<T>` with per-row chevron/indent flags, not because they mean
/// anything about the process itself.
///
/// This runs fresh every render; `pin_display_row` (applied by the caller
/// afterward) is what actually keeps the selected row from jumping to a
/// new position on every re-sort.
fn flatten_for_display(groups: &[ProcessGroup], expanded: &HashSet<String>) -> Vec<DisplayRow> {
    let mut out = Vec::new();
    for group in groups {
        let has_children = group.members.len() > 1;
        let is_expanded = has_children && expanded.contains(&group.leader.name);

        out.push(DisplayRow {
            row: group.leader.clone(),
            depth: 0,
            has_children,
            is_expanded,
            group_size: group.members.len(),
        });

        if is_expanded {
            out.extend(group.members[1..].iter().map(|r| DisplayRow {
                row: r.clone(),
                depth: 1,
                has_children: false,
                is_expanded: false,
                group_size: 1,
            }));
        }
    }
    out
}

#[derive(Clone)]
struct DisplayRow {
    row: ProcessRow,
    depth: u8,
    has_children: bool,
    is_expanded: bool,
    group_size: usize,
}

/// Keeps `target_pid`'s row at its previous on-screen position, the same
/// way `domain2::ProcessesActor::sort_rows_pinned` stabilizes the raw list
/// before grouping - but that pin operates on individual `ProcessRow`s by
/// index, and grouping can collapse several of those into one displayed
/// row, or promote a different member to the leader slot. Once the rows
/// reaching the table are `DisplayRow`s, the *rendered* row's position is
/// the only stability guarantee that still means anything, so it has to be
/// re-applied here on the post-grouping list.
///
/// Returns the row's new position (`None` if `target_pid` isn't rendered
/// at all right now, e.g. a non-leader member of a collapsed group) - feed
/// that back in as `prev_pos` on the next call to keep the pin anchored.
fn pin_display_row(
    rows: &mut Vec<DisplayRow>,
    target_pid: Option<u32>,
    prev_pos: Option<usize>,
) -> Option<usize> {
    let pid = target_pid?;
    let current = rows.iter().position(|d| d.row.pid == pid)?;
    let insert_at = prev_pos.unwrap_or(current).min(rows.len() - 1);
    if insert_at != current {
        let row = rows.remove(current);
        rows.insert(insert_at, row);
    }
    Some(insert_at)
}

pub fn processes_view<S: amethystate::Store>(
    cx: &mut PageCx,
    map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
) -> Element {
    let (state, dispatch) = cx.use_reducer::<ProcessesReducer>();
    let (metrics_state, _) = cx.use_reducer::<MetricsReducer>();

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
            let layout = cx.use_ref(ColumnLayout::new(map));
            let layout = layout.borrow();
            let icons = cx.use_ref(context2::IconCache::new());

            // Expand/collapse is pure presentation state - it has no
            // domain meaning and nothing else needs to observe it, so it
            // lives here rather than round-tripping through the actor the
            // way selection/sort do.
            let (expanded, set_expanded) = cx.use_state(HashSet::<String>::new());
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

            let groups = group_by_name(rows, state.selected);
            let mut display_rows = flatten_for_display(&groups, &expanded);
            let columns = build_columns(&layout, machine, rows, icons.clone(), toggle_expanded);

            let pinned_display_pos = cx.use_ref(Option::<usize>::None);
            let prev_pos = *pinned_display_pos.borrow();
            let new_pos = pin_display_row(&mut display_rows, state.selected, prev_pos);
            *pinned_display_pos.borrow_mut() = new_pos;

            let selected_index = new_pos.map(|i| i as i32).unwrap_or(-1);
            let rows_for_select = display_rows.clone();
            let select_dispatch = dispatch.clone();
            let on_selection_changed = SetState::new(move |idx: i32| {
                if idx >= 0
                    && let Some(d) = rows_for_select.get(idx as usize)
                {
                    select_dispatch.emit_on_select(d.row.pid);
                }
            });

            table_with_sort_indicator(
                cx,
                display_rows,
                columns,
                |d: &DisplayRow| d.row.pid.to_string(),
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

/// Sort direction indicator for the table header, matching the old Slint
/// app's `SortArrow` rather than the plain text glyph the framework falls
/// back to when no indicator is supplied.
///
/// `guicons::Icon` only plugs into another control's icon slot (Button,
/// NavViewItem, ...) - there's no standalone icon-rendering widget in this
/// reactor fork. `Image` is standalone and its backend already dispatches
/// `.svg` URIs to `SvgImageSource` vs `BitmapImage` by extension, so this
/// pulls the already-resolved URI out of the `Icon::Svg` guicons built and
/// renders it directly instead.
fn sort_indicator_icon(descending: bool) -> Element {
    let resolved = if descending {
        icon!(chevron_down_regular).size(10.0).build()
    } else {
        icon!(chevron_up_regular).size(10.0).build()
    };
    match resolved {
        Icon::Svg { uri, width, height } => {
            Image::new_with_uri(uri).width(width).height(height).into()
        }
        other => unreachable!("guicons resolves iconify svg assets to Icon::Svg, got {other:?}"),
    }
}

fn separator() -> Element {
    Shape::rectangle()
        .fill(table_styles::TABLE_STYLES.separator_color)
        .height(1.0)
        .into()
}

/// Reserved width of the expand-chevron slot in the Name column, occupied
/// on every row regardless of whether that row actually has one - see
/// `name_column`.
const CHEVRON_SLOT_WIDTH: f64 = 14.0;

/// Expand/collapse chevron for a group leader - right-pointing when
/// collapsed (click to reveal), down when expanded, mirroring the sort
/// indicator's approach of pulling the resolved URI out of guicons's
/// `Icon::Svg` and rendering it as a standalone `Image`.
fn expand_chevron(expanded: bool) -> Element {
    let resolved = if expanded {
        icon!(chevron_down_regular).size(10.0).build()
    } else {
        icon!(chevron_right_regular).size(10.0).build()
    };
    match resolved {
        Icon::Svg { uri, width, height } => {
            Image::new_with_uri(uri).width(width).height(height).into()
        }
        other => unreachable!("guicons resolves iconify svg assets to Icon::Svg, got {other:?}"),
    }
}

/// Generic fallback for a process whose own icon couldn't be resolved
/// (no `exe_path`, extraction failed, ...) - `fluent-color:apps-24`, the
/// same "generic app" glyph the old Slint app's `IconProvider` fell back
/// to (`context::icons::keys::APP`). It's a full-color icon (its palette
/// is baked into the SVG, not tinted via a host Foreground), so unlike the
/// monochrome chevrons it needs no theme-color handling.
fn fallback_process_icon() -> Element {
    let resolved = icon!(app).size(16.0).build();
    match resolved {
        Icon::Svg { uri, width, height } => {
            Image::new_with_uri(uri).width(width).height(height).into()
        }
        other => unreachable!("guicons resolves iconify svg assets to Icon::Svg, got {other:?}"),
    }
}

/// The `Name` column: process icon, name, and - for a group leader - an
/// expand chevron and a "(N)" size badge. Expanded members render at
/// `depth: 1`, indented and without a chevron of their own.
///
/// Icons resolve through [`context2::IconCache`] from `exe_path`/
/// `package_full_name` (`cmdline[0]` from the agent report; `name` alone
/// isn't resolvable to a file). Cells with no resolvable icon fall back to
/// [`fallback_process_icon`] rather than leaving a gap.
fn name_column(
    width: Signal<u64>,
    min_width: f64,
    icons: HookRef<context2::IconCache>,
    toggle_expanded: SetState<String>,
) -> ColumnSpec<DisplayRow> {
    ColumnSpec::new("name", "Name", width, move |d: &DisplayRow| {
        let row = &d.row;
        let package = Some(row.package_full_name.as_str()).filter(|s| !s.is_empty());
        let icon_path = icons.borrow().icon_path(context2::IconRequest {
            path: &row.exe_path,
            package_full_name: package,
        });

        let icon: Element = match icon_path {
            Some(path) => {
                let uri = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
                Image::new_with_uri(uri).width(16.0).height(16.0).into()
            }
            None => fallback_process_icon(),
        };

        let name = if d.has_children {
            format!("{} ({})", row.name, d.group_size)
        } else {
            row.name.clone()
        };
        let text = table_styles::TABLE_STYLES.text_cell(name);

        // Reserved at a fixed width on every row, whether or not this row
        // has a chevron - otherwise rows with and without one would start
        // their icon/name at different x offsets, which is what actually
        // read as "overlapping" text: the eye expects one consistent left
        // edge for the whole Name column, not one that shifts per row.
        let chevron_content: Element = if d.has_children {
            let name_key = row.name.clone();
            let toggle = toggle_expanded.clone();
            border(expand_chevron(d.is_expanded))
                .on_tapped(move || toggle.call(name_key.clone()))
                .into()
        } else {
            Element::Empty
        };
        let chevron = border(chevron_content).width(CHEVRON_SLOT_WIDTH);

        let indent = (d.depth as f64) * 16.0;
        hstack((Element::from(chevron), icon, text))
            .spacing(6.0)
            .padding(windows_reactor::Thickness::xy(indent, 0.0))
            .into()
    })
    .min_width(min_width)
    .sortable()
}

/// Like [`name_column`]'s plain siblings, but washes each cell with the
/// accent color at an opacity proportional to `intensity(row)` (expected
/// `0.0..=1.0`) - the periphery-readable "who's using this resource" cue
/// agreed for the metric columns, without a bar control competing with the
/// text.
fn heat_column(
    id: &'static str,
    header: &'static str,
    width: Signal<u64>,
    min_width: f64,
    accent: Color,
    fmt: impl Fn(&ProcessRow) -> String + 'static,
    intensity: impl Fn(&ProcessRow) -> f32 + 'static,
) -> ColumnSpec<DisplayRow> {
    ColumnSpec::new(id, header, width, move |d: &DisplayRow| {
        table_styles::TABLE_STYLES.heat_cell(fmt(&d.row), intensity(&d.row), accent)
    })
    .min_width(min_width)
    .sortable()
}

/// Single-line column header: label, then the machine-wide aggregate for
/// that resource in a muted color. The sort indicator is appended by the
/// table widget itself, after this element.
fn metric_header(label: &'static str, value: String) -> Element {
    hstack((
        text_block(label),
        text_block(value).foreground(ThemeRef::SecondaryText),
    ))
    .spacing(6.0)
    .into()
}

fn cpu_column(
    width: Signal<u64>,
    min_width: f64,
    machine: Option<MachineSummary>,
    accent: Color,
) -> ColumnSpec<DisplayRow> {
    let header = move || {
        machine
            .as_ref()
            .map(|m| metric_header("CPU", format!("{:.1}%", m.cpu_percent)))
            .unwrap_or_else(|| text_block("CPU").into())
    };
    ColumnSpec::new_with_header("cpu", header, width, move |d: &DisplayRow| {
        // Already normalized to the whole machine (matches the aggregate
        // shown in the header), so it doubles as the heat intensity as-is.
        let cpu_percent = d.row.cpu_percent;
        let intensity = cpu_percent / 100.0;
        table_styles::TABLE_STYLES.heat_cell(format!("{cpu_percent:.1}%"), intensity, accent)
    })
    .min_width(min_width)
    .sortable()
}

fn memory_column(
    width: Signal<u64>,
    min_width: f64,
    machine: Option<MachineSummary>,
    accent: Color,
) -> ColumnSpec<DisplayRow> {
    let memory_total_bytes = machine.as_ref().map(|m| m.memory_total_bytes).unwrap_or(0);
    let header = move || {
        machine
            .as_ref()
            .map(|m| {
                let percent = if m.memory_total_bytes > 0 {
                    (m.memory_used_bytes as f64 / m.memory_total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                metric_header("Memory", format!("{:.1}%", percent))
            })
            .unwrap_or_else(|| text_block("Memory").into())
    };
    ColumnSpec::new_with_header("memory", header, width, move |d: &DisplayRow| {
        let intensity = if memory_total_bytes > 0 {
            d.row.memory_bytes as f32 / memory_total_bytes as f32
        } else {
            0.0
        };
        table_styles::TABLE_STYLES.heat_cell(
            table_styles::format_bytes(d.row.memory_bytes),
            intensity,
            accent,
        )
    })
    .min_width(min_width)
    .sortable()
}

const COLUMN_IDS: &[&str] = &["name", "cpu", "memory", "net", "disk"];
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
    rows: &[ProcessRow],
    icons: HookRef<context2::IconCache>,
    toggle_expanded: SetState<String>,
) -> Vec<ColumnSpec<DisplayRow>> {
    // Net/Disk have no machine-wide total to normalize against (unlike
    // CPU/Memory), so their heat is relative to the busiest row currently
    // on screen rather than a fabricated absolute percentage.
    let net_max = rows.iter().map(|r| r.net_bytes).max().unwrap_or(0).max(1) as f32;
    let disk_max = rows.iter().map(|r| r.disk_bytes).max().unwrap_or(0).max(1) as f32;
    let accent = table_styles::accent_color();
    layout
        .entries
        .iter()
        .filter(|e| e.visible.get())
        .map(|e| {
            build_column(
                e,
                machine.clone(),
                accent,
                net_max,
                disk_max,
                icons.clone(),
                toggle_expanded.clone(),
            )
        })
        .collect()
}

fn build_column(
    entry: &ColumnLayoutEntry,
    machine: Option<MachineSummary>,
    accent: Color,
    net_max: f32,
    disk_max: f32,
    icons: HookRef<context2::IconCache>,
    toggle_expanded: SetState<String>,
) -> ColumnSpec<DisplayRow> {
    let width = entry.width.clone();
    let min_width = entry.min_width.get() as f64;
    match entry.id {
        "name" => name_column(width, min_width, icons, toggle_expanded),
        "cpu" => cpu_column(width, min_width, machine, accent),
        "memory" => memory_column(width, min_width, machine, accent),
        "net" => heat_column(
            "net",
            "Net",
            width,
            min_width,
            accent,
            |r| table_styles::format_bytes(r.net_bytes),
            move |r| r.net_bytes as f32 / net_max,
        ),
        "disk" => heat_column(
            "disk",
            "Disk",
            width,
            min_width,
            accent,
            |r| table_styles::format_bytes(r.disk_bytes),
            move |r| r.disk_bytes as f32 / disk_max,
        ),
        _ => unreachable!("unknown column id"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, name: &str) -> ProcessRow {
        ProcessRow {
            pid,
            name: name.to_string(),
            cpu_percent: pid as f32, // distinct per row, handy for eyeballing failures
            memory_bytes: 0,
            disk_bytes: 0,
            net_bytes: 0,
            exe_path: String::new(),
            package_full_name: String::new(),
        }
    }

    fn pids(rows: &[DisplayRow]) -> Vec<u32> {
        rows.iter().map(|d| d.row.pid).collect()
    }

    fn group(leader: ProcessRow, rest: Vec<ProcessRow>) -> ProcessGroup {
        let mut members = vec![leader.clone()];
        members.extend(rest);
        ProcessGroup { leader, members }
    }

    // --- group_by_name: pure domain computation, no UI concepts ---

    #[test]
    fn ungrouped_rows_form_singleton_groups() {
        let rows = vec![row(1, "alpha"), row(2, "beta"), row(3, "gamma")];
        let groups = group_by_name(&rows, None);

        assert_eq!(groups.len(), 3);
        assert!(groups.iter().all(|g| g.members.len() == 1));
    }

    #[test]
    fn duplicate_names_group_with_lowest_pid_as_leader_by_default() {
        let rows = vec![
            row(5, "chrome.exe"),
            row(2, "chrome.exe"),
            row(9, "chrome.exe"),
        ];
        let groups = group_by_name(&rows, None);

        assert_eq!(groups.len(), 1, "same-named rows collapse into one group");
        assert_eq!(groups[0].leader.pid, 2, "leader defaults to the lowest pid");
        assert_eq!(groups[0].members.len(), 3);
    }

    #[test]
    fn selecting_a_non_lowest_pid_member_promotes_it_to_leader() {
        // Regression: before leader promotion, selecting/pinning any group
        // member but the lowest-pid one made it vanish the instant its
        // group collapsed in the UI, because only the (unrelated) leader
        // row rendered.
        let rows = vec![
            row(5, "chrome.exe"),
            row(2, "chrome.exe"),
            row(9, "chrome.exe"),
        ];
        let groups = group_by_name(&rows, Some(9));

        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].leader.pid, 9,
            "selected member becomes the leader, not the lowest pid"
        );
    }

    #[test]
    fn leader_sums_metrics_across_the_group() {
        let rows = vec![row(1, "chrome.exe"), row(2, "chrome.exe")];
        let groups = group_by_name(&rows, None);

        // cpu_percent was seeded as `pid as f32` in the `row` helper.
        assert_eq!(groups[0].leader.cpu_percent, 1.0 + 2.0);
    }

    // --- flatten_for_display: presentation only ---

    #[test]
    fn collapsed_groups_render_as_a_single_leader_row() {
        let groups = vec![group(
            row(2, "chrome.exe"),
            vec![row(5, "chrome.exe"), row(9, "chrome.exe")],
        )];
        let out = flatten_for_display(&groups, &HashSet::new());

        assert_eq!(pids(&out), vec![2]);
        assert!(out[0].has_children);
        assert_eq!(out[0].group_size, 3);
    }

    #[test]
    fn expanding_a_group_inserts_members_right_after_its_leader() {
        let groups = vec![
            group(row(10, "alpha"), vec![]),
            group(row(2, "chrome.exe"), vec![row(5, "chrome.exe")]),
            group(row(20, "zeta"), vec![]),
        ];
        let mut expanded = HashSet::new();
        expanded.insert("chrome.exe".to_string());
        let out = flatten_for_display(&groups, &expanded);

        // Leader (2) then its one member (5), flanked by the untouched
        // neighbors - nothing before/after the group moved.
        assert_eq!(pids(&out), vec![10, 2, 5, 20]);
        assert_eq!(out[1].depth, 0);
        assert_eq!(out[2].depth, 1);
        assert!(!out[2].has_children);
    }

    #[test]
    fn pin_keeps_the_selected_row_at_its_previous_position_across_a_resort() {
        let mut rows = vec![row(1, "a"), row(2, "b"), row(3, "c")];
        let display: Vec<DisplayRow> = rows
            .drain(..)
            .map(|row| DisplayRow {
                row,
                depth: 0,
                has_children: false,
                is_expanded: false,
                group_size: 1,
            })
            .collect();

        // First call: nothing to anchor to yet, just report where it is.
        let mut first = display.clone();
        let pos = pin_display_row(&mut first, Some(2), None);
        assert_eq!(pos, Some(1));
        assert_eq!(
            pids(&first),
            vec![1, 2, 3],
            "no prior position - nothing should move yet"
        );

        // Simulate a re-sort that pushed pid 2 to the front; the pin
        // should pull it back to its previously-reported index.
        let mut resorted = vec![display[1].clone(), display[0].clone(), display[2].clone()];
        let pos = pin_display_row(&mut resorted, Some(2), pos);
        assert_eq!(pos, Some(1));
        assert_eq!(
            pids(&resorted),
            vec![1, 2, 3],
            "pinned row pulled back to index 1"
        );
    }

    #[test]
    fn pin_reports_none_when_the_target_is_not_currently_rendered() {
        let mut display = vec![DisplayRow {
            row: row(1, "a"),
            depth: 0,
            has_children: false,
            is_expanded: false,
            group_size: 1,
        }];
        // pid 99 doesn't exist in this list - e.g. a non-leader member of
        // a collapsed group.
        let pos = pin_display_row(&mut display, Some(99), Some(0));
        assert_eq!(pos, None);
    }
}
