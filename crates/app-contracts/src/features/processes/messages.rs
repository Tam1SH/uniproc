use crate::features::agents::AgentConnectionState;
use guinea_core::messages;
use guinea_macros::port;
use std::rc::Rc;

use super::model::{MachineSummary, ProcessRow};

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

messages! {
    pub Processes {
        Sort(String),
        Select(u32),
        Deselect,
        Terminate,
    }
}
