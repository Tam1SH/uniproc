use crate::features::agents::AgentConnectionState;
use guinea_core::Load;
use guinea_macros::{actions, port, reducer};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, amethystate::AmeType)]
pub struct ColumnConfig {
    pub width: u64,
    pub min_width: u64,
    pub visible: bool,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            width: 110,
            min_width: 80,
            visible: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ProcessCategory {
    App,
    BackgroundMicrosoft,
    BackgroundThirdParty,
    WindowsService,
    WindowsKernel,
}

impl ProcessCategory {
    pub fn classify(
        has_visible_window: bool,
        is_kernel_process: bool,
        is_service: bool,
        is_microsoft_signed: bool,
    ) -> Self {
        if has_visible_window {
            Self::App
        } else if is_kernel_process {
            Self::WindowsKernel
        } else if is_service {
            Self::WindowsService
        } else if is_microsoft_signed {
            Self::BackgroundMicrosoft
        } else {
            Self::BackgroundThirdParty
        }
    }

    pub const ORDER: [Self; 5] = [
        Self::App,
        Self::BackgroundThirdParty,
        Self::BackgroundMicrosoft,
        Self::WindowsService,
        Self::WindowsKernel,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::App => "Apps",
            Self::BackgroundThirdParty => "Background processes",
            Self::BackgroundMicrosoft => "Background processes (Microsoft)",
            Self::WindowsService => "Services",
            Self::WindowsKernel => "Windows kernel",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub display_name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub net_bytes: u64,
    pub exe_path: String,
    pub package_full_name: String,
    pub category: ProcessCategory,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct MachineSummary {
    pub cpu_percent: f32,
    pub cpu_current_mhz: u64,
    pub cpu_max_mhz: u64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
}

#[derive(Clone)]
pub enum ProcessesMsg {
    SetRows {
        rows: Rc<[ProcessRow]>,
        machine: MachineSummary,
        agent_state: AgentConnectionState,
    },
    SetSelected(Option<u32>),
    SetSort {
        column: String,
        descending: bool,
    },
}

#[port]
pub trait ProcessesPort: 'static {
    fn send(&self, msg: ProcessesMsg);
}

#[actions]
pub trait ProcessesActions {
    fn on_sort<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    fn on_select<F>(&self, handler: F)
    where
        F: Fn(u32) + 'static;

    fn on_deselect<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_terminate<F>(&self, handler: F)
    where
        F: Fn() + 'static;
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProcessesState {
    pub rows: Load<Rc<[ProcessRow]>>,
    pub machine_summary: Load<MachineSummary>,
    pub selected: Option<u32>,
    pub sort_column: String,
    pub descending: bool,
    pub agent_state: AgentConnectionState,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            rows: Load::Loading,
            machine_summary: Load::Loading,
            selected: None,
            agent_state: AgentConnectionState::Disconnected,
            sort_column: "cpu".to_string(),
            descending: true,
        }
    }
}

impl ProcessesState {
    pub fn rows(&self) -> &[ProcessRow] {
        self.rows.ready().map(|r| r.as_ref()).unwrap_or(&[])
    }

    pub fn total(&self) -> usize {
        self.rows().len()
    }

    pub fn machine_summary(&self) -> Option<&MachineSummary> {
        self.machine_summary.ready()
    }
}

#[reducer]
#[dispatch(ProcessesActions)]
pub fn processes_reducer(state: &mut ProcessesState, msg: ProcessesMsg) {
    match msg {
        ProcessesMsg::SetRows { rows, machine, agent_state } => {
            state.rows = Load::Ready(rows);
            state.machine_summary = Load::Ready(machine);
            state.agent_state = agent_state;
        }
        ProcessesMsg::SetSelected(pid) => {
            state.selected = pid;
        }
        ProcessesMsg::SetSort { column, descending } => {
            state.sort_column = column;
            state.descending = descending;
        }
    }
}
