use crate::features::processes::MachineSummary;
use guinea_core::Load;
use guinea_macros::reducer;

use super::messages::MetricsMsg;

#[derive(Clone, PartialEq, Debug)]
pub struct MetricsState {
    pub cpu_history: Load<Vec<(u64, f32)>>,
    pub memory_history: Load<Vec<(u64, f32)>>,
    pub machine: Load<MachineSummary>,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self {
            cpu_history: Load::Loading,
            memory_history: Load::Loading,
            machine: Load::Loading,
        }
    }
}

#[reducer]
pub fn metrics_reducer(state: &mut MetricsState, msg: MetricsMsg) {
    match msg {
        MetricsMsg::SetHistory {
            cpu,
            memory,
            machine,
        } => {
            state.cpu_history = Load::Ready(cpu);
            state.memory_history = Load::Ready(memory);
            state.machine = Load::Ready(machine);
        }
    }
}
