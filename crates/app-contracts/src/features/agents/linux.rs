use guinea_core::actor::Message;

use super::connection::AgentConnectionState;

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
