//! Agent-facing contracts: owned data the rest of the app consumes, plus the
//! command vocabulary it sends back.
//!
//! Deliberately free of any transport/wire types. The agents now speak
//! capnp-rpc (see `domain2::features::agents`), whose readers borrow from the
//! message buffer they were decoded out of and are `!Send` - neither of which
//! survives a trip through the event bus. Everything here is plain owned Rust,
//! decoded once at the transport boundary.

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

// ─────────────────────────────── Windows ───────────────────────────────

/// Mirrors `windows.capnp`'s `SignatureStatus`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SignatureStatus {
    #[default]
    Unknown,
    Unsigned,
    Microsoft,
    ThirdParty,
}

/// Mirrors `windows.capnp`'s `ProcessPriority`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessPriority {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

/// Mirrors `windows.capnp`'s `MachineStats`.
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

/// Mirrors `windows.capnp`'s `ProcessStats`.
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
}

/// Mirrors `windows.capnp`'s `Report`.
#[derive(Clone, Debug, Default)]
pub struct WindowsReport {
    pub machine: WindowsMachineStats,
    pub processes: Vec<WindowsProcessStats>,
}

#[derive(Clone, Debug)]
pub struct WindowsReportMessage(pub WindowsReport);
impl Message for WindowsReportMessage {}

#[derive(Clone, Debug)]
pub struct WindowsAgentRuntimeEvent {
    pub state: AgentConnectionState,
    pub latency_ms: Option<i32>,
}
impl Message for WindowsAgentRuntimeEvent {}

/// A single mutating call on the Windows agent. One variant per non-`ping`,
/// non-`getReport` method in `windows.capnp`'s `WindowsAgent` interface; every
/// one of them answers with a Win32 error code, hence the shared
/// [`WindowsActionResponse`].
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
    /// Win32 error code as reported by the agent; `0` is success.
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

// ──────────────────────────────── Linux ────────────────────────────────

/// Mirrors `linux.capnp`'s `EnvironmentKind`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EnvironmentKind {
    #[default]
    Unknown,
    CurrentDistro,
    DockerContainer,
    UnknownExternalNamespace,
}

/// Mirrors `linux.capnp`'s `MachineStats`.
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
}

/// Mirrors `linux.capnp`'s `ProcessStats`.
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

/// Mirrors `linux.capnp`'s `EnvironmentInfo`.
#[derive(Clone, Debug, Default)]
pub struct LinuxEnvironmentInfo {
    pub mnt_ns: u64,
    pub pid_ns: u64,
    pub kind: EnvironmentKind,
    /// Distro name for `CurrentDistro`, container id for `DockerContainer`,
    /// empty otherwise.
    pub name: String,
}

/// Mirrors `linux.capnp`'s `DockerContainerInfo`.
#[derive(Clone, Debug, Default)]
pub struct LinuxDockerContainerInfo {
    pub id: String,
    pub mnt_ns: u64,
    pub pid_ns: u64,
    pub api_version: String,
    pub raw_json: String,
}

/// Mirrors `linux.capnp`'s `Report`.
#[derive(Clone, Debug, Default)]
pub struct LinuxReport {
    pub machine: LinuxMachineStats,
    pub processes: Vec<LinuxProcessStats>,
    pub environments: Vec<LinuxEnvironmentInfo>,
    pub docker_containers: Vec<LinuxDockerContainerInfo>,
}

/// A scan result from a non-host agent, tagged with which one produced it.
#[derive(Clone, Debug)]
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
