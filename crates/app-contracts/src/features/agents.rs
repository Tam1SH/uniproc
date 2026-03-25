use app_core::actor::traits::Message;
use uniproc_protocol::{LinuxMachineStats, LinuxProcessStats};
#[cfg(target_os = "windows")]
use uniproc_protocol::WindowsReport;

#[derive(Debug, Clone)]
pub struct ScanTick;
impl Message for ScanTick {}

#[derive(Clone)]
pub struct RemoteScanResult {
    pub schema_id: &'static str,
    pub processes: Vec<LinuxProcessStats>,
    pub machine: LinuxMachineStats,
}
impl Message for RemoteScanResult {}

#[cfg(target_os = "windows")]
#[derive(Clone)]
pub struct WindowsReportMessage(pub WindowsReport);

#[cfg(target_os = "windows")]
impl Message for WindowsReportMessage {}
