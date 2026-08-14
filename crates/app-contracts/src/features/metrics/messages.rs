use crate::features::processes::MachineSummary;
use guinea_macros::port;

#[derive(Clone)]
pub enum MetricsMsg {
    SetHistory {
        cpu: Vec<(u64, f32)>,
        memory: Vec<(u64, f32)>,
        machine: MachineSummary,
    },
}

#[port]
pub trait MetricsPort: 'static {
    fn send(&self, msg: MetricsMsg);
}
