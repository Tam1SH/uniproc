use app_contracts::features::agents::WindowsServiceState;
use app_contracts::features::services::ServiceRow;
use guicons::icon;
use guinea::widgets::table::ColumnSpec;
use windows_reactor::{hstack, tokens, Element, ElementExt};

use crate::l10n::L10n;
use crate::theme::{size, space};
use crate::widgets::table_cell;

fn service_icon() -> Element {
    icon!(gears).size(size::Icon).build_element()
}

fn maybe_dim(el: Element, running: bool) -> Element {
    if running { el } else { el.opacity(0.55) }
}

fn state_label(l10n: &L10n, state: WindowsServiceState) -> String {
    match state {
        WindowsServiceState::Unknown => l10n.services_state_unknown(),
        WindowsServiceState::Stopped => l10n.services_state_stopped(),
        WindowsServiceState::StartPending => l10n.services_state_start_pending(),
        WindowsServiceState::StopPending => l10n.services_state_stop_pending(),
        WindowsServiceState::Running => l10n.services_state_running(),
        WindowsServiceState::ContinuePending => l10n.services_state_continue_pending(),
        WindowsServiceState::PausePending => l10n.services_state_pause_pending(),
        WindowsServiceState::Paused => l10n.services_state_paused(),
    }
}

pub(crate) fn build_columns(l10n: &L10n) -> Vec<ColumnSpec<ServiceRow>> {
    vec![
        ColumnSpec::new(
            "name",
            l10n.services_col_name(),
            260u64,
            |row: &ServiceRow| {
                let content = hstack((
                    service_icon(),
                    table_cell::cell_text(row.display_name.clone()),
                ))
                .spacing(space::Control);
                maybe_dim(content.into(), row.is_running())
            },
        )
        .sortable(),
        ColumnSpec::new(
            "status",
            l10n.services_col_status(),
            90u64,
            {
                let l10n = l10n.clone();
                move |row: &ServiceRow| {
                    let running = row.is_running();
                    let label = state_label(&l10n, row.state);
                    let el: Element = if running {
                        table_cell::cell_text(label)
                            .foreground(tokens::SystemSuccess)
                            .into()
                    } else {
                        table_cell::cell_text(label).into()
                    };
                    maybe_dim(el, running)
                }
            },
        )
        .sortable(),
        ColumnSpec::new("pid", l10n.services_col_pid(), 70u64, |row: &ServiceRow| {
            let text = if row.pid == 0 {
                String::new()
            } else {
                row.pid.to_string()
            };
            maybe_dim(table_cell::cell_text(text).into(), row.is_running())
        })
        .sortable(),
        ColumnSpec::new(
            "group",
            l10n.services_col_group(),
            120u64,
            |row: &ServiceRow| {
                maybe_dim(
                    table_cell::cell_text(row.group.clone()).into(),
                    row.is_running(),
                )
            },
        )
        .sortable(),
        ColumnSpec::new(
            "description",
            l10n.services_col_description(),
            320u64,
            |row: &ServiceRow| {
                maybe_dim(
                    table_cell::cell_text(row.description.clone()).into(),
                    row.is_running(),
                )
            },
        ),
    ]
}
