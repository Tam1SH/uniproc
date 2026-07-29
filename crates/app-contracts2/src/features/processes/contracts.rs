use guinea_core::Load;
use guinea_macros::{actions, port, reducer};

#[derive(Clone, PartialEq, Debug)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub net_bytes: u64,
}

#[derive(Clone)]
pub enum ProcessesMsg {
    SetRows(Vec<ProcessRow>),
    SetSelected(Option<u32>),
    SetSort { column: String, descending: bool },
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
    pub rows: Load<Vec<ProcessRow>>,
    pub selected: Option<u32>,
    pub sort_column: String,
    pub descending: bool,
}

impl Default for ProcessesState {
    fn default() -> Self {
        Self {
            rows: Load::Loading,
            selected: None,
            sort_column: "cpu".to_string(),
            descending: true,
        }
    }
}

impl ProcessesState {
    pub fn rows(&self) -> &[ProcessRow] {
        self.rows.ready().map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn total(&self) -> usize {
        self.rows.ready().map(Vec::len).unwrap_or(0)
    }
}

#[reducer]
#[dispatch(ProcessesActions)]
pub fn processes_reducer(state: &mut ProcessesState, msg: ProcessesMsg) {
    match msg {
        ProcessesMsg::SetRows(rows) => {
            state.rows = Load::Ready(rows);
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
