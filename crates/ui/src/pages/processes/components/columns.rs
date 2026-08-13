use app_contracts::features::processes::{MachineSummary, ProcessCategory, ProcessRow};
use guicons::icon;
use guinea::widgets::table::ColumnSpec;
use guinea_core::signal::Signal;
use windows_reactor::{
    border, component, hstack, text_block, tokens, Color, ColorScheme, Component, Element,
    ElementExt, HookRef, Image, RenderCx, SetState,
};

use crate::format;
use crate::l10n::L10n;
use crate::theme::{accent_color, size, space, Palette};
use crate::widgets::table_cell;

use super::column_layout::ColumnLayoutEntry;
use super::grouping::{DisplayRow, SectionRow};

pub(crate) fn sort_indicator_icon(descending: bool) -> Element {
    if descending {
        icon!(chevron_down_regular).size(size::Chevron).build_element()
    } else {
        icon!(chevron_up_regular).size(size::Chevron).build_element()
    }
}

const CHEVRON_SLOT_WIDTH: f64 = 14.0;
const NAME_CELL_SPACING: f64 = space::Control;
use super::grouping::MEMORY_COMPRESSION;

fn memory_heat_color(row: &ProcessRow, accent: Color, palette: Palette) -> Color {
    if row.name == MEMORY_COMPRESSION {
        palette.heat_muted
    } else {
        accent
    }
}


fn expand_chevron(expanded: bool) -> Element {
    if expanded {
        icon!(chevron_down_regular).size(size::Chevron).build_element()
    } else {
        icon!(chevron_right_regular).size(size::Chevron).build_element()
    }
}

fn fallback_process_icon() -> Element {
    icon!(app).size(size::Icon).build_element()
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
    section_label: String,
}

struct NameCell {
    icons: HookRef<context::IconCache>,
    toggle_expanded: SetState<String>,
    toggle_section: SetState<ProcessCategory>,
}

impl Component<NameCellProps> for NameCell {
    fn render(&self, props: &NameCellProps, _cx: &mut RenderCx) -> Element {
        if let Some(section) = &props.section {
            return self.render_section(props, section);
        }

        let package = Some(props.package_full_name.as_str()).filter(|s| !s.is_empty());
        let icon_path = self.icons.borrow().icon_path(context::IconRequest {
            path: &props.exe_path,
            package_full_name: package,
        });
        let icon: Element = match icon_path {
            Some(path) => {
                let uri = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
                Image::new_with_uri(uri).width(size::Icon).height(size::Icon).into()
            }
            None => fallback_process_icon(),
        };

        let name = if props.has_children {
            format!("{} ({})", props.name, props.group_size)
        } else {
            props.name.clone()
        };
        let text = table_cell::text_cell(name);

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
        let text = table_cell::section_cell(format!(
            "{} ({})",
            props.section_label, props.group_size
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
    icons: HookRef<context::IconCache>,
    toggle_expanded: SetState<String>,
    toggle_section: SetState<ProcessCategory>,
    l10n: L10n,
) -> ColumnSpec<DisplayRow> {
    let header_l10n = l10n.clone();
    let header = move || {
        text_block(header_l10n.processes_col_name())
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
                section_label: d
                    .section
                    .as_ref()
                    .map(|s| category_label(&l10n, s.category))
                    .unwrap_or_default(),
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
    header: String,
    width: Signal<u64>,
    min_width: f64,
    accent: Color,
    fmt: impl Fn(&ProcessRow) -> String + 'static,
    intensity: impl Fn(&ProcessRow) -> f32 + 'static,
) -> ColumnSpec<DisplayRow> {
    ColumnSpec::new(id, header, width, move |d: &DisplayRow| {
        table_cell::heat_cell(fmt(&d.row), intensity(&d.row), accent)
    })
    .min_width(min_width)
    .sortable()
}

fn metric_header(label: String, value: String) -> Element {
    hstack((
        text_block(label),
        text_block(value).foreground(tokens::SecondaryText),
    ))
    .spacing(space::Control)
    .into()
}

fn cpu_column(
    width: Signal<u64>,
    min_width: f64,
    machine: Option<MachineSummary>,
    accent: Color,
    l10n: L10n,
) -> ColumnSpec<DisplayRow> {
    let header = move || {
        machine
            .as_ref()
            .map(|m| {
                metric_header(
                    l10n.processes_col_cpu(),
                    format!("{:.1}%", m.cpu_percent),
                )
            })
            .unwrap_or_else(|| text_block(l10n.processes_col_cpu()).into())
    };
    ColumnSpec::new_with_header("cpu", header, width, move |d: &DisplayRow| {
        let cpu_percent = d.row.cpu_percent;
        let intensity = cpu_percent / 100.0;
        table_cell::heat_cell(format!("{cpu_percent:.1}%"), intensity, accent)
    })
    .min_width(min_width)
    .sortable()
}

fn memory_column(
    width: Signal<u64>,
    min_width: f64,
    machine: Option<MachineSummary>,
    accent: Color,
    palette: Palette,
    l10n: L10n,
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
                metric_header(l10n.processes_col_memory(), format!("{:.1}%", percent))
            })
            .unwrap_or_else(|| text_block(l10n.processes_col_memory()).into())
    };
    ColumnSpec::new_with_header("memory", header, width, move |d: &DisplayRow| {
        let intensity = if memory_total_bytes > 0 {
            d.row.memory_bytes as f32 / memory_total_bytes as f32
        } else {
            0.0
        };
        let compressed = d.section.as_ref().map_or(0, |s| s.compressed_bytes);
        if compressed > 0 {
            return table_cell::split_heat_cell(
                format::bytes(d.row.memory_bytes),
                intensity,
                accent,
                palette.heat_muted,
                d.row.memory_bytes,
                compressed,
            );
        }
        table_cell::heat_cell(
            format::bytes(d.row.memory_bytes),
            intensity,
            memory_heat_color(&d.row, accent, palette),
        )
    })
    .min_width(min_width)
    .sortable()
}

pub(crate) fn build_columns(
    layout: &super::column_layout::ColumnLayout,
    machine: Option<MachineSummary>,
    rows: &[ProcessRow],
    icons: HookRef<context::IconCache>,
    toggle_expanded: SetState<String>,
    toggle_section: SetState<ProcessCategory>,
    scheme: ColorScheme,
    l10n: &L10n,
) -> Vec<ColumnSpec<DisplayRow>> {
    let palette = Palette::of(scheme);
    let net_max = rows.iter().map(|r| r.net_bytes).max().unwrap_or(0).max(1) as f32;
    let disk_max = rows.iter().map(|r| r.disk_bytes).max().unwrap_or(0).max(1) as f32;
    let accent = accent_color();
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
                palette,
                l10n.clone(),
            )
        })
        .collect()
}

pub(crate) fn category_label(l10n: &L10n, category: ProcessCategory) -> String {
    match category {
        ProcessCategory::App => l10n.processes_category_app(),
        ProcessCategory::BackgroundThirdParty => l10n.processes_category_background_third_party(),
        ProcessCategory::BackgroundMicrosoft => l10n.processes_category_background_microsoft(),
        ProcessCategory::WindowsService => l10n.processes_category_windows_service(),
        ProcessCategory::WindowsKernel => l10n.processes_category_windows_kernel(),
    }
}

fn build_column(
    entry: &ColumnLayoutEntry,
    machine: Option<MachineSummary>,
    accent: Color,
    net_max: f32,
    disk_max: f32,
    icons: HookRef<context::IconCache>,
    toggle_expanded: SetState<String>,
    toggle_section: SetState<ProcessCategory>,
    palette: Palette,
    l10n: L10n,
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
            l10n,
        ),
        "cpu" => cpu_column(width, min_width, machine, accent, l10n),
        "memory" => memory_column(width, min_width, machine, accent, palette, l10n),
        "net" => heat_column(
            "net",
            l10n.processes_col_net(),
            width,
            min_width,
            accent,
            |r| format::bytes(r.net_bytes),
            move |r| r.net_bytes as f32 / net_max,
        ),
        "disk" => heat_column(
            "disk",
            l10n.processes_col_disk(),
            width,
            min_width,
            accent,
            |r| format::bytes(r.disk_bytes),
            move |r| r.disk_bytes as f32 / disk_max,
        ),
        _ => unreachable!("unknown column id"),
    }
}
