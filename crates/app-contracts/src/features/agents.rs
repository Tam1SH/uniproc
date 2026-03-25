use app_core::actor::traits::Message;
use uniproc_protocol::{MachineStats, ProcessStats};

#[derive(Debug, Clone)]
pub struct ScanTick;
impl Message for ScanTick {}

#[derive(Clone)]
pub struct RemoteScanResult {
    pub schema_id: &'static str,
    pub processes: Vec<ProcessStats>,
    pub machine: MachineStats,
}
impl Message for RemoteScanResult {}
