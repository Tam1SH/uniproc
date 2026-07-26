use guinea_core::actor::Message;
use std::sync::Arc;

use ogurpchik::codecs::base::{HasAllocator, MessageCodec};
use ogurpchik::high::client::Client;
use ogurpchik::pool::buf_guard::BufGuard;
use uniproc_protocol::{
    LinuxCodec, LinuxDockerContainerInfo, LinuxEnvironmentInfo, LinuxMachineStats, LinuxProcessStats,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentConnectionState {
    Disconnected,
    Connecting,
    Connected,
    WaitingRetry { delay_secs: u64 },
}

type RpcClient<C> = Client<
    C,
    <C as MessageCodec>::Dest,
    BufGuard<<C as MessageCodec>::Dest, <<C as MessageCodec>::Dest as HasAllocator>::SharedAlloc>,
>;

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        use uniproc_protocol::{WindowsCodec, WindowsReport, WindowsRequest, WindowsResponse};

        #[derive(Clone, Debug)]
        pub struct WslAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub latency_ms: Option<i32>,
        }
        impl Message for WslAgentRuntimeEvent {}

        #[derive(Clone, Debug)]
        pub struct WindowsAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub latency_ms: Option<i32>,
        }
        impl Message for WindowsAgentRuntimeEvent {}

        pub type AgentClient = RpcClient<WindowsCodec>;
        pub type WslClient = RpcClient<LinuxCodec>;

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
    } else {
        pub type AgentClient = RpcClient<LinuxCodec>;

        #[derive(Clone)]
        pub struct LinuxAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub latency_ms: Option<i32>,
        }
        impl Message for LinuxAgentRuntimeEvent {}
    }
}
