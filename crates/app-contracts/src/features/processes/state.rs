use crate::features::agents::AgentConnectionState;
use guinea_core::Load;
use guinea_macros::reducer;
use std::rc::Rc;

use super::messages::{ProcessesDispatch, ProcessesMsg};
use super::model::{MachineSummary, ProcessRow};

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
