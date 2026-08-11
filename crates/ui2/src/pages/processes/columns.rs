use app_contracts2::features::processes::{MachineSummary, ProcessCategory, ProcessRow};
use guicons::icon;
use guinea::widgets::table::ColumnSpec;
use guinea_core::signal::Signal;
use windows_reactor::{
    border, component, hstack, text_block, Color, Component, Element, ElementExt, HookRef, Image,
    RenderCx, SetState, ThemeRef,
};

use crate::table_styles;

use super::column_layout::ColumnLayoutEntry;
use super::grouping::{DisplayRow, SectionRow};

/// Sort direction indicator for the table header, matching the old Slint
/// app's `SortArrow` rather than the plain text glyph the framework falls
/// back to when no indicator is supplied.
///
pub(super) fn sort_indicator_icon(descending: bool) -> Element {
    if descending {
        icon!(chevron_down_regular).size(10.0).build_element()
    } else {
        icon!(chevron_up_regular).size(10.0).build_element()
    }
}

/// Reserved width of the expand-chevron slot in the Name column, occupied
/// on every row regardless of whether that row actually has one - see
/// `name_column`.
/// Reserved on every row, with or without a chevron in it, so that the
/// chevrons of headings and of group leaders line up on one left edge -
/// which is also what keeps every icon and name starting at the same x.
const CHEVRON_SLOT_WIDTH: f64 = 14.0;
/// Gap between the chevron slot and what follows it in a Name cell.
const NAME_CELL_SPACING: f64 = 6.0;
/// The kernel's compressed-page store, as the OS names it. Not localized -
/// this is the image name, which is the same on every machine.
const MEMORY_COMPRESSION: &str = "Memory Compression";

/// The heat wash color for a row. Memory Compression gets grey instead of
/// the accent: its working set is routinely one of the largest on the
/// machine, and painting it the same alarming red as a runaway program says
/// "look here" about something that is the OS doing its job. Grey keeps the
/// magnitude readable without the call to action.
fn heat_color(row: &ProcessRow, accent: Color) -> Color {
    if row.name == MEMORY_COMPRESSION {
        MUTED_HEAT_COLOR
    } else {
        accent
    }
}

/// Neutral grey for a heat wash - opacity still tracks the value, so the
/// bar reads as "this much" rather than "this bad".
const MUTED_HEAT_COLOR: Color = Color {
    a: 255,
    r: 150,
    g: 150,
    b: 150,
};

/// Expand/collapse chevron for a group leader - right-pointing when
/// collapsed (click to reveal), down when expanded.
fn expand_chevron(expanded: bool) -> Element {
    if expanded {
        icon!(chevron_down_regular).size(10.0).build_element()
    } else {
        icon!(chevron_right_regular).size(10.0).build_element()
    }
}

/// Generic fallback for a process whose own icon couldn't be resolved
/// (no `exe_path`, extraction failed, ...) - `fluent-color:apps-24`, the
/// same "generic app" glyph the old Slint app's `IconProvider` fell back
/// to (`context::icons::keys::APP`). It's a full-color icon (its palette
/// is baked into the SVG, not tinted via a host Foreground), so unlike the
/// monochrome chevrons it needs no theme-color handling.
fn fallback_process_icon() -> Element {
    icon!(app).size(16.0).build_element()
}

/// Everything the Name cell needs to decide *what* to draw: icon identity,
/// display name, and this row's spot in the current group/expand topology.
/// Deliberately excludes `cpu_percent`/`memory_bytes`/... - the rest of
/// `ProcessRow` - which change on essentially every tick for a live
/// process, unlike this set, which only changes when a process spawns/
/// exits or a group is expanded/collapsed.
#[derive(Clone, PartialEq)]
struct NameCellProps {
    exe_path: String,
    package_full_name: String,
    name: String,
    has_children: bool,
    is_expanded: bool,
    group_size: usize,
    /// `Some` on a heading row: no icon, and the chevron collapses a whole
    /// category instead of one process group.
    section: Option<SectionRow>,
}

/// The `Name` column's cell: process icon, name, and - for a group leader -
/// an expand chevron and a "(N)" size badge. Expanded members render at
/// without a chevron of their own, at the same left edge as the leader.
///
/// Was wrapped in [`memo`] keyed on [`NameCellProps`], and is not any more:
/// a memoised component whose render returns a real widget (this one returns
/// an `hstack`) stops being re-rendered by the reconciler - upstream
/// microsoft/windows-rs#4802, still open. In the table that showed up as
/// cells drawn over their previous contents.
///
/// The memo is worth restoring once that is fixed, because the reasoning
/// below still holds:
///
/// Keyed on [`NameCellProps`] - a whole-row memo key
/// would be invalidated every render by `cpu_percent`/`memory_bytes`/...
/// and never actually skip the icon lookup (the expensive part: an
/// `IconCache` lookup from `exe_path`/`package_full_name`, plus building
/// the name text/chevron). Keying on identity+topology alone means all of
/// it is resolved once per process and reused for that process's entire
/// lifetime in the table (or until it changes group/expand state),
/// independent of how often its metrics change. This is also what was
/// causing the icon to visibly flicker every tick before this change - a
/// fresh `Image` element was being constructed on every render regardless
/// of whether the icon itself could possibly have changed.
struct NameCell {
    icons: HookRef<context2::IconCache>,
    toggle_expanded: SetState<String>,
    toggle_section: SetState<ProcessCategory>,
}

impl Component<NameCellProps> for NameCell {
    fn render(&self, props: &NameCellProps, _cx: &mut RenderCx) -> Element {
        if let Some(section) = &props.section {
            return self.render_section(props, section);
        }

        let package = Some(props.package_full_name.as_str()).filter(|s| !s.is_empty());
        let icon_path = self.icons.borrow().icon_path(context2::IconRequest {
            path: &props.exe_path,
            package_full_name: package,
        });
        let icon: Element = match icon_path {
            Some(path) => {
                let uri = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
                Image::new_with_uri(uri).width(16.0).height(16.0).into()
            }
            None => fallback_process_icon(),
        };

        let name = if props.has_children {
            format!("{} ({})", props.name, props.group_size)
        } else {
            props.name.clone()
        };
        let text = table_styles::TABLE_STYLES.text_cell(name);

        // Reserved at a fixed width on every row, whether or not this row
        // has a chevron - otherwise rows with and without one would start
        // their icon/name at different x offsets, which is what actually
        // read as "overlapping" text: the eye expects one consistent left
        // edge for the whole Name column, not one that shifts per row.
        let chevron_content: Element = if props.has_children {
            let name_key = props.name.clone();
            let toggle = self.toggle_expanded.clone();
            border(expand_chevron(props.is_expanded))
                .on_tapped(move || toggle.call(name_key.clone()))
                .into()
        } else {
            Element::Empty
        };
        let chevron = border(chevron_content).width(CHEVRON_SLOT_WIDTH);

        // No per-depth indent. Expanded group members sit at the same left
        // edge as their leader: the leader is one of them (the lowest pid),
        // not a parent they belong under, so stepping them in would claim a
        // hierarchy that isn't there.
        hstack((Element::from(chevron), icon, text))
            .spacing(NAME_CELL_SPACING)
            .into()
    }
}

impl NameCell {
    /// A heading: label, count and a chevron that hides everything under it.
    /// No process icon: there is no process here, and borrowing one from the
    /// first member would suggest the heading *is* that process.
    fn render_section(&self, props: &NameCellProps, section: &SectionRow) -> Element {
        let text = table_styles::TABLE_STYLES.section_cell(format!(
            "{} ({})",
            section.category.label(),
            props.group_size
        ));

        let category = section.category;
        let toggle = self.toggle_section.clone();
        let chevron = border(expand_chevron(props.is_expanded))
            .on_tapped(move || toggle.call(category));

        hstack((
            Element::from(border(chevron).width(CHEVRON_SLOT_WIDTH)),
            text,
        ))
        .spacing(NAME_CELL_SPACING)
        .into()
    }
}

fn name_column(
    width: Signal<u64>,
    min_width: f64,
    icons: HookRef<context2::IconCache>,
    toggle_expanded: SetState<String>,
    toggle_section: SetState<ProcessCategory>,
) -> ColumnSpec<DisplayRow> {
    // Inset by exactly the chevron slot the cells reserve, so the header
    // label starts on the same x as a section heading's text rather than
    // over the chevrons.
    let header = || {
        text_block("Name")
            .padding(windows_reactor::Thickness {
                left: CHEVRON_SLOT_WIDTH + NAME_CELL_SPACING,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            })
            .into()
    };
    ColumnSpec::new_with_header("name", header, width, move |d: &DisplayRow| {
        // Not `memo`, deliberately - see the note on NameCell.
        let content = component(
            NameCell {
                icons: icons.clone(),
                toggle_expanded: toggle_expanded.clone(),
                toggle_section: toggle_section.clone(),
            },
            NameCellProps {
                exe_path: d.row.exe_path.clone(),
                package_full_name: d.row.package_full_name.clone(),
                // The display name is what a person reads; `name` stays the
                // grouping key and the memo key, so a process whose
                // resolved name is empty still behaves identically.
                name: d.row.display_name.clone(),
                has_children: d.has_children,
                is_expanded: d.is_expanded,
                group_size: d.group_size,
                section: d.section.clone(),
            },
        );
        // `row_view` (guinea's table widget) applies `.width()`/`.padding()`
        // to whatever this closure returns - but `Element::Component` (what
        // `memo` produces) has no modifiers slot, so those calls would
        // silently no-op directly on it, collapsing the column's width and
        // blowing up the whole row's layout. Wrapping in a plain `border`
        // gives the table a real widget element to size, while the memoised
        // content stays nested (and still skippable) inside it.
        border(content).into()
    })
    .min_width(min_width)
    // The cell reserves its own chevron slot; the table's shared inset on
    // top of that is what pushed every name off the left edge.
    .flush()
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
        table_styles::TABLE_STYLES.heat_cell(
            fmt(&d.row),
            intensity(&d.row),
            heat_color(&d.row, accent),
        )
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
        table_styles::TABLE_STYLES.heat_cell(
            format!("{cpu_percent:.1}%"),
            intensity,
            heat_color(&d.row, accent),
        )
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
            heat_color(&d.row, accent),
        )
    })
    .min_width(min_width)
    .sortable()
}

pub(super) fn build_columns(
    layout: &super::column_layout::ColumnLayout,
    machine: Option<MachineSummary>,
    rows: &[ProcessRow],
    icons: HookRef<context2::IconCache>,
    toggle_expanded: SetState<String>,
    toggle_section: SetState<ProcessCategory>,
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
                toggle_section.clone(),
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
    toggle_section: SetState<ProcessCategory>,
) -> ColumnSpec<DisplayRow> {
    let width = entry.width.clone();
    let min_width = entry.min_width.get() as f64;
    match entry.id {
        "name" => name_column(
            width,
            min_width,
            icons,
            toggle_expanded,
            toggle_section,
        ),
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
