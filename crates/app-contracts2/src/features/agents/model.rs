use guinea_core::actor::Message;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ScanTick;
impl Message for ScanTick {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentConnectionState {
    Disconnected,
    Connecting,
    Connected,
    WaitingRetry { delay_secs: u64 },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SignatureStatus {
    #[default]
    Unknown,
    Unsigned,
    Microsoft,
    ThirdParty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessPriority {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

#[derive(Clone, Debug, Default)]
pub struct WindowsMachineStats {
    pub total_physical_kb: u64,
    pub available_physical_kb: u64,
    pub used_physical_kb: u64,
    pub cpu_percent: f32,
    pub cpu_max_mhz: u64,
    pub cpu_current_mhz: u64,

    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub disk_read_iops: u64,
    pub disk_write_iops: u64,

    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct WindowsProcessStats {
    pub pid: u32,
    pub parent_pid: u32,
    pub session_id: u32,
    pub name: String,
    pub cmdline: Vec<String>,
    pub package_full_name: String,
    pub package_relative_app_id: String,
    pub cpu_percent: f32,
    pub working_set_kb: u64,
    pub private_bytes_kb: u64,
    pub peak_working_set_kb: u64,
    pub private_working_set_kb: u64,

    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub disk_read_iops: u64,
    pub disk_write_iops: u64,

    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,

    pub is_service: bool,
    pub has_visible_window: bool,
    pub is_kernel_process: bool,
    pub is_windows_process: bool,
    pub signature: SignatureStatus,
    pub image_path: String,
    pub display_name: String,
}

impl WindowsProcessStats {
    pub fn memory_kb(&self) -> u64 {
        if self.private_working_set_kb > 0 {
            self.private_working_set_kb
        } else {
            self.working_set_kb
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowsReport {
    pub machine: WindowsMachineStats,
    pub processes: Vec<WindowsProcessStats>,
}

#[derive(Clone, Debug)]
pub enum WindowsReportMessage {
    Report(WindowsReport),
    Unavailable(AgentConnectionState),
}
impl Message for WindowsReportMessage {}

#[derive(Clone, Debug)]
pub struct WindowsAgentRuntimeEvent {
    pub state: AgentConnectionState,
    pub latency_ms: Option<i32>,
}
impl Message for WindowsAgentRuntimeEvent {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowsAction {
    Kill { pid: u32 },
    Suspend { pid: u32 },
    Resume { pid: u32 },
    SetPriority { pid: u32, priority: ProcessPriority },
    SetAffinity { pid: u32, mask: u64 },
    ServiceStart { name: String },
    ServiceStop { name: String },
    ServicePause { name: String },
    ServiceResume { name: String },
    ServiceRestart { name: String },
}

#[derive(Clone, Debug)]
pub struct WindowsActionRequest {
    pub correlation_id: Uuid,
    pub action: WindowsAction,
}
impl Message for WindowsActionRequest {}

impl WindowsActionRequest {
    pub fn new(correlation_id: Uuid, action: WindowsAction) -> Self {
        Self { correlation_id, action }
    }
}

#[derive(Clone, Debug)]
pub struct WindowsActionResponse {
    pub correlation_id: Uuid,
    pub code: u32,
}
impl Message for WindowsActionResponse {}

impl WindowsActionResponse {
    pub fn new(correlation_id: Uuid, code: u32) -> Self {
        Self { correlation_id, code }
    }

    pub fn succeeded(&self) -> bool {
        self.code == 0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EnvironmentKind {
    #[default]
    Unknown,
    CurrentDistro,
    DockerContainer,
    UnknownExternalNamespace,
}

#[derive(Clone, Debug, Default)]
pub struct LinuxMachineStats {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub used_kb: u64,
    pub cached_kb: u64,

    pub busy_ns: u64,
    pub last_tsc: u64,

    pub vsock_rx_bytes: u64,
    pub vsock_tx_bytes: u64,
    pub p9_rx_bytes: u64,
    pub p9_tx_bytes: u64,

    pub tcp_tx_lo_bytes: u64,
    pub tcp_rx_lo_bytes: u64,
    pub tcp_tx_remote_bytes: u64,
    pub tcp_rx_remote_bytes: u64,
    pub udp_tx_lo_bytes: u64,
    pub udp_rx_lo_bytes: u64,
    pub udp_tx_remote_bytes: u64,
    pub udp_rx_remote_bytes: u64,
    pub uds_tx_bytes: u64,
    pub uds_rx_bytes: u64,

    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub disk_read_iops: u64,
    pub disk_write_iops: u64,

    pub pipe_read_bytes: u64,
    pub pipe_write_bytes: u64,
    pub sendfile_bytes: u64,

    pub cpu_count: u32,
}

#[derive(Clone, Debug, Default)]
pub struct LinuxProcessStats {
    pub global_pid: u32,
    pub local_pid: u32,
    pub mnt_ns: u64,
    pub pid_ns: u64,
    pub name: String,

    pub cpu_percent: f32,
    pub rss_kb: u64,
    pub last_active_ns: u64,

    pub vsock_rx_bytes: u64,
    pub vsock_tx_bytes: u64,
    pub p9_rx_bytes: u64,
    pub p9_tx_bytes: u64,

    pub tcp_tx_lo_bytes: u64,
    pub tcp_rx_lo_bytes: u64,
    pub tcp_tx_remote_bytes: u64,
    pub tcp_rx_remote_bytes: u64,
    pub udp_tx_lo_bytes: u64,
    pub udp_rx_lo_bytes: u64,
    pub udp_tx_remote_bytes: u64,
    pub udp_rx_remote_bytes: u64,
    pub uds_tx_bytes: u64,
    pub uds_rx_bytes: u64,

    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub disk_read_iops: u64,
    pub disk_write_iops: u64,

    pub pipe_read_bytes: u64,
    pub pipe_write_bytes: u64,
    pub sendfile_bytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct LinuxEnvironmentInfo {
    pub mnt_ns: u64,
    pub pid_ns: u64,
    pub kind: EnvironmentKind,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct LinuxDockerContainerInfo {
    pub id: String,
    pub mnt_ns: u64,
    pub pid_ns: u64,
    pub api_version: String,
    pub raw_json: String,
}

#[derive(Clone, Debug, Default)]
pub struct LinuxReport {
    pub machine: LinuxMachineStats,
    pub processes: Vec<LinuxProcessStats>,
    pub environments: Vec<LinuxEnvironmentInfo>,
    pub docker_containers: Vec<LinuxDockerContainerInfo>,
}

#[derive(Clone, Debug)]
pub struct RemoteScan {
    pub schema_id: &'static str,
    pub processes: Vec<LinuxProcessStats>,
    pub machine: LinuxMachineStats,
    pub environments: Vec<LinuxEnvironmentInfo>,
    pub docker_containers: Vec<LinuxDockerContainerInfo>,
}

#[derive(Clone, Debug)]
pub enum RemoteScanResult {
    Scan(RemoteScan),
    Unavailable(AgentConnectionState),
}
impl Message for RemoteScanResult {}

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        #[derive(Clone, Debug)]
        pub struct WslAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub latency_ms: Option<i32>,
        }
        impl Message for WslAgentRuntimeEvent {}
    } else {
        #[derive(Clone, Debug)]
        pub struct LinuxAgentRuntimeEvent {
            pub state: AgentConnectionState,
            pub latency_ms: Option<i32>,
        }
        impl Message for LinuxAgentRuntimeEvent {}
    }
}
