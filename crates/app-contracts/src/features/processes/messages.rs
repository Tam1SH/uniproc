use crate::features::agents::AgentConnectionState;
use guinea_macros::{actions, port};
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
