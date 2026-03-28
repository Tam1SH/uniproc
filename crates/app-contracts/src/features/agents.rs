use app_core::actor::traits::Message;
#[cfg(target_os = "windows")]
use uniproc_protocol::WindowsReport;
use uniproc_protocol::{LinuxMachineStats, LinuxProcessStats};

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

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        #[derive(Clone)]
        pub struct WindowsReportMessage(pub WindowsReport);
        impl Message for WindowsReportMessage {}
    }
}
