use app_contracts::features::wsl::{AgentPresence, DistroRow, LinuxMachineSummary};
use guicons::icon;
use guinea::widgets::table::ColumnSpec;
use windows_reactor::{
    border, hstack, text_block, tokens, BrushBinding, Color, Element, ElementExt, Thickness,
    VerticalAlignment,
};

use crate::format;
use crate::l10n::L10n;
use crate::theme::{accent_color, size, space};
use crate::widgets::table_cell;

const NAME_TEXT_INSET: f64 = size::Dot + size::Icon + space::Control * 2.0 + 6.0;

fn maybe_dim(el: Element, running: bool) -> Element {
    if running { el } else { el.opacity(0.55) }
}

fn distro_icon(name: &str) -> Element {
    let name = name.to_ascii_lowercase();
    let icon = if name.contains("ubuntu") {
        icon!(ubuntu)
    } else if name.contains("centos") {
        icon!(centos)
    } else if name.contains("debian") {
        icon!(debian)
    } else if name.contains("fedora") {
        icon!(fedora)
    } else if name.contains("docker") {
        icon!(docker)
    } else {
        icon!(linux)
    };
    icon.size(size::Icon).build_element()
}

fn agent_dot(presence: AgentPresence) -> Element {
    let color: BrushBinding = match presence {
        AgentPresence::Answering => tokens::SystemSuccess.into(),
        AgentPresence::Silent => tokens::SystemCaution.into(),
        AgentPresence::NotChecked => Color {
            a: 90,
            r: 128,
            g: 128,
            b: 128,
        }
        .into(),
    };

    border(Element::Empty)
        .width(size::Dot)
        .height(size::Dot)
        .corner_radius(size::Dot / 2.0)
        .background(color)
        .vertical_alignment(VerticalAlignment::Center)
        .into()
}

fn name_header(l10n: &L10n) -> Element {
    border(text_block(l10n.wsl_col_distribution()).margin(Thickness {
        left: NAME_TEXT_INSET,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    }))
    .into()
}

fn metric_column(
    id: &'static str,
    header: String,
    width: u64,
    read: impl Fn(&LinuxMachineSummary) -> (String, f32) + 'static,
) -> ColumnSpec<DistroRow> {
    ColumnSpec::new(id, header, width, move |row: &DistroRow| {
        let Some(metrics) = row.metrics.as_ref() else {
            return maybe_dim(
                table_cell::cell_text("-")
                    .foreground(tokens::SecondaryText)
                    .into(),
                row.running,
            );
        };
        let (text, intensity) = read(metrics);
        table_cell::heat_cell(text, intensity, accent_color())
    })
}

fn name_column(l10n: &L10n) -> ColumnSpec<DistroRow> {
    let header_l10n = l10n.clone();
    ColumnSpec::new_with_header(
        "name",
        move || name_header(&header_l10n),
        260u64,
        |row: &DistroRow| {
            let content = hstack((
                agent_dot(row.agent),
                distro_icon(&row.name),
                table_cell::cell_text(row.name.clone()),
            ))
            .spacing(space::Control);
            maybe_dim(content.into(), row.running)
        },
    )
    .flush()
}

fn status_column(l10n: &L10n) -> ColumnSpec<DistroRow> {
    let cell_l10n = l10n.clone();
    ColumnSpec::new(
        "status",
        l10n.wsl_col_status(),
        110u64,
        move |row: &DistroRow| {
            let label = if row.running {
                cell_l10n.wsl_running()
            } else {
                cell_l10n.wsl_stopped()
            };
            let el: Element = if row.running {
                table_cell::cell_text(label)
                    .foreground(tokens::SystemSuccess)
                    .into()
            } else {
                table_cell::cell_text(label)
                    .foreground(tokens::SecondaryText)
                    .into()
            };
            maybe_dim(el, row.running)
        },
    )
}

pub(crate) fn build_columns(l10n: &L10n) -> Vec<ColumnSpec<DistroRow>> {
    vec![
        name_column(l10n),
        status_column(l10n),
        metric_column("cpu", l10n.wsl_col_cpu(), 110u64, |m| {
            (
                m.cpu_percent
                    .map(|p| format!("{p:.1}%"))
                    .unwrap_or_else(|| "-".into()),
                m.cpu_percent.unwrap_or(0.0) / 100.0,
            )
        }),
        metric_column("memory", l10n.wsl_col_memory(), 130u64, |m| {
            let share = if m.memory_total_bytes > 0 {
                m.memory_used_bytes as f32 / m.memory_total_bytes as f32
            } else {
                0.0
            };
            (format::bytes(m.memory_used_bytes), share)
        }),
        metric_column("net", l10n.wsl_col_net(), 110u64, |m| {
            (format::bytes(m.net_bytes), 0.0)
        }),
        metric_column("disk", l10n.wsl_col_disk(), 110u64, |m| {
            (format::bytes(m.disk_bytes), 0.0)
        }),
    ]
}
