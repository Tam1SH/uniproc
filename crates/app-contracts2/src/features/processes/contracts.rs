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

#[derive(Clone, PartialEq, Debug)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub net_bytes: u64,
    pub exe_path: String,
    pub package_full_name: String,
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
    /// Agent reports as an async resource: `Load::Loading` until the first
    /// snapshot arrives. An empty vec inside `Ready` is a valid answer (and a
    /// search filter may legitimately produce one) - it must never be read as
    /// "still loading".
    pub rows: Load<Rc<[ProcessRow]>>,
    /// Machine-level summary (CPU and memory totals) delivered together with
    /// each process snapshot.
    pub machine_summary: Load<MachineSummary>,
    pub selected: Option<u32>,
    pub sort_column: String,
    pub descending: bool,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            rows: Load::Loading,
            machine_summary: Load::Loading,
            selected: None,
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
        ProcessesMsg::SetRows { rows, machine } => {
            state.rows = Load::Ready(rows);
            state.machine_summary = Load::Ready(machine);
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
