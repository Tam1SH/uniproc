use app_contracts2::features::wsl::{AgentPresence, DistroRow, LinuxMachineSummary, WslReducer};
use guicons::icon;
use guinea::router::PageCx;
use guinea::widgets::table::{ColumnSpec, table};
use guinea_core::Load;
use windows_reactor::{
    body_large, border, grid, hstack, text_block, Color, Element, ElementExt, GridLength,
    HorizontalAlignment, ProgressRing, Shape, VerticalAlignment,
};

use crate::table_styles;

const RUNNING_GREEN: Color = Color {
    a: 255,
    r: 32,
    g: 166,
    b: 79,
};

const SILENT_AMBER: Color = Color {
    a: 255,
    r: 214,
    g: 154,
    b: 46,
};

fn maybe_dim(el: Element, running: bool) -> Element {
    if running { el } else { el.opacity(0.55) }
}

fn agent_label(presence: AgentPresence) -> (&'static str, Option<Color>) {
    match presence {
        AgentPresence::Answering => ("Answering", Some(RUNNING_GREEN)),
        AgentPresence::Silent => ("No agent", Some(SILENT_AMBER)),
        AgentPresence::NotChecked => ("Not checked", None),
    }
}

fn machine_summary(machine: Option<&LinuxMachineSummary>) -> Element {
    let Some(machine) = machine else {
        return text_block("No agent is reporting").padding(8.0).into();
    };

    let memory = format!(
        "{} of {}",
        table_styles::format_bytes(machine.memory_used_bytes),
        table_styles::format_bytes(machine.memory_total_bytes),
    );

    let cpu = match machine.cpu_percent {
        Some(percent) => format!("CPU: {percent:.1}%"),
        None => "CPU: -".to_string(),
    };

    hstack((
        text_block(cpu),
        text_block(format!("Memory: {memory}")),
        text_block(format!("Processes: {}", machine.process_count)),
        text_block(format!("Containers: {}", machine.container_count)),
    ))
    .spacing(16.0)
    .padding(8.0)
    .into()
}

pub fn wsl_view(cx: &mut PageCx) -> Element {
    let (state, _dispatch) = cx.use_reducer::<WslReducer>();

    let header = hstack((body_large("WSL").padding(12.0),)).spacing(12.0);

    let body: Element = match &state.distros {
        Load::Ready(rows) => {
            let columns = vec![
                ColumnSpec::new("name", "Distribution", 220u64, |row: &DistroRow| {
                    let content = hstack((
                        icon!(linux).size(16.0).build_element(),
                        table_styles::cell_text(row.name.clone()),
                    ))
                    .spacing(6.0);
                    maybe_dim(content.into(), row.running)
                }),
                ColumnSpec::new("state", "State", 110u64, |row: &DistroRow| {
                    let label = if row.running { "Running" } else { "Stopped" };
                    let el: Element = if row.running {
                        text_block(label).foreground(RUNNING_GREEN).into()
                    } else {
                        text_block(label).into()
                    };
                    maybe_dim(el, row.running)
                }),
                ColumnSpec::new("agent", "Agent", 140u64, |row: &DistroRow| {
                    let (label, color) = agent_label(row.agent);
                    let el: Element = match color {
                        Some(color) => text_block(label).foreground(color).into(),
                        None => text_block(label).into(),
                    };
                    maybe_dim(el, row.running)
                }),
            ];

            table(
                cx,
                rows.to_vec(),
                columns,
                |row: &DistroRow| row.name.clone(),
                None,
                None,
            )
        }
        Load::Failed(err) => text_block(format!("Failed to list distributions: {err}")).into(),
        _ => ProgressRing::indeterminate()
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };

    let separator: Element = Shape::rectangle()
        .fill(Color {
            a: 48,
            r: 128,
            g: 128,
            b: 128,
        })
        .height(1.0)
        .into();

    let status_bar = text_block(format!(
        "Distributions: {} ({} running)",
        state.total(),
        state.running()
    ))
    .padding(8.0);

    grid((
        header.grid_row(0),
        machine_summary(state.machine.as_ref()).grid_row(1),
        border(body).grid_row(2),
        separator.grid_row(3),
        status_bar.grid_row(4),
    ))
    .rows([
        GridLength::Auto,
        GridLength::Auto,
        GridLength::Star(1.0),
        GridLength::Auto,
        GridLength::Auto,
    ])
    .into()
}
