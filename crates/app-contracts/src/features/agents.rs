use app_core::actor::traits::Message;

use uniproc_protocol::{LinuxMachineStats, LinuxProcessStats};
use uuid::Uuid;

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

        use uniproc_protocol::{WindowsReport, WindowsRequest, WindowsResponse};

        #[derive(Clone)]
        pub struct WindowsReportMessage(pub WindowsReport);

        impl Message for WindowsReportMessage {}

        #[derive(Clone, Debug)]
        pub struct WindowsActionRequest {
            pub correlation_id: Uuid,
            pub request: WindowsRequest,
        }

        #[derive(Clone, Debug)]
        pub struct WindowsActionResponse {
            pub correlation_id: Uuid,
            pub response: WindowsResponse,
        }

        impl Message for WindowsActionRequest {}
        impl Message for WindowsActionResponse {}

    }
}
