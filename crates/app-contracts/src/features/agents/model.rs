use app_core::actor::traits::Message;
use std::sync::Arc;

use uniproc_protocol::{
    LinuxDockerContainerInfo, LinuxEnvironmentInfo, LinuxMachineStats, LinuxProcessStats,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ScanTick;
impl Message for ScanTick {}

#[derive(Clone)]
pub struct RemoteScanResult {
    pub schema_id: &'static str,
    pub processes: Vec<LinuxProcessStats>,
    pub machine: LinuxMachineStats,
    pub environments: Vec<LinuxEnvironmentInfo>,
    pub docker_containers: Vec<LinuxDockerContainerInfo>,
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
            request_bytes: Arc<[u8]>,
        }

        #[derive(Clone, Debug)]
        pub struct WindowsActionResponse {
            pub correlation_id: Uuid,
            response_bytes: Arc<[u8]>,
        }

        impl Message for WindowsActionRequest {}
        impl Message for WindowsActionResponse {}

        impl WindowsActionRequest {
            pub fn new(correlation_id: Uuid, request: WindowsRequest) -> Self {
                let request_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&request)
                    .expect("WindowsActionRequest should serialize");

                Self {
                    correlation_id,
                    request_bytes: Arc::<[u8]>::from(request_bytes.into_boxed_slice()),
                }
            }

            pub fn decode_request(&self) -> Result<WindowsRequest, rkyv::rancor::Error> {
                rkyv::from_bytes::<WindowsRequest, rkyv::rancor::Error>(&self.request_bytes)
            }
        }

        impl WindowsActionResponse {
            pub fn new(correlation_id: Uuid, response: &WindowsResponse) -> Self {
                let response_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(response)
                    .expect("WindowsActionResponse should serialize");

                Self {
                    correlation_id,
                    response_bytes: Arc::<[u8]>::from(response_bytes.into_boxed_slice()),
                }
            }

            pub fn decode_response(&self) -> Result<WindowsResponse, rkyv::rancor::Error> {
                rkyv::from_bytes::<WindowsResponse, rkyv::rancor::Error>(&self.response_bytes)
            }
        }

    }
}
