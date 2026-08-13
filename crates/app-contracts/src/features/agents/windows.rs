use guinea_core::actor::Message;
use uuid::Uuid;

use super::connection::AgentConnectionState;

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowsServiceState {
    #[default]
    Unknown,
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
}

impl WindowsServiceState {
    pub fn id(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Stopped => "stopped",
            Self::StartPending => "start-pending",
            Self::StopPending => "stop-pending",
            Self::Running => "running",
            Self::ContinuePending => "continue-pending",
            Self::PausePending => "pause-pending",
            Self::Paused => "paused",
        }
    }

    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

#[derive(Clone, Debug, Default)]
pub struct WindowsServiceStats {
    pub name: String,
    pub display_name: String,
    pub pid: u32,
    pub state: WindowsServiceState,
    pub load_group: String,
    pub description: String,
    pub image_path: String,
}

#[derive(Clone, Debug, Default)]
pub struct WindowsReport {
    pub machine: WindowsMachineStats,
    pub processes: Vec<WindowsProcessStats>,
    pub services: Vec<WindowsServiceStats>,
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

