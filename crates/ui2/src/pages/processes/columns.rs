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

pub(super) fn sort_indicator_icon(descending: bool) -> Element {
    if descending {
        icon!(chevron_down_regular).size(10.0).build_element()
    } else {
        icon!(chevron_up_regular).size(10.0).build_element()
    }
}

const CHEVRON_SLOT_WIDTH: f64 = 14.0;
const NAME_CELL_SPACING: f64 = 6.0;
const MEMORY_COMPRESSION: &str = "Memory Compression";

fn heat_color(row: &ProcessRow, accent: Color) -> Color {
    if row.name == MEMORY_COMPRESSION {
        MUTED_HEAT_COLOR
    } else {
        accent
    }
}

const MUTED_HEAT_COLOR: Color = Color {
    a: 255,
    r: 150,
    g: 150,
    b: 150,
};

fn expand_chevron(expanded: bool) -> Element {
    if expanded {
        icon!(chevron_down_regular).size(10.0).build_element()
    } else {
        icon!(chevron_right_regular).size(10.0).build_element()
    }
}

fn fallback_process_icon() -> Element {
    icon!(app).size(16.0).build_element()
}

#[derive(Clone, PartialEq)]
struct NameCellProps {
    exe_path: String,
    package_full_name: String,
    name: String,
    has_children: bool,
    is_expanded: bool,
    group_size: usize,
    section: Option<SectionRow>,
}

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

        hstack((Element::from(chevron), icon, text))
            .spacing(NAME_CELL_SPACING)
            .into()
    }
}

impl NameCell {
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
        let content = component(
            NameCell {
                icons: icons.clone(),
                toggle_expanded: toggle_expanded.clone(),
                toggle_section: toggle_section.clone(),
            },
            NameCellProps {
                exe_path: d.row.exe_path.clone(),
                package_full_name: d.row.package_full_name.clone(),
                name: d.row.display_name.clone(),
                has_children: d.has_children,
                is_expanded: d.is_expanded,
                group_size: d.group_size,
                section: d.section.clone(),
            },
        );
        border(content).into()
    })
    .min_width(min_width)
    .flush()
    .sortable()
}

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
