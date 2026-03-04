#[derive(Debug, Clone)]
pub struct WindowsProcessStat {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub parent_pid: u32,
    pub exe_path: Option<String>,
    pub command_line: Option<String>,
    pub user_name: Option<String>,
    pub disk_read: Option<u64>,
    pub disk_write: Option<u64>,
    pub net_usage: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WindowsStats {
    pub cpu_percent: f32,
    pub ram_percent: f32,
    pub disk_percent: f32,
    pub net_percent: f32,
    pub net_total_bandwidth: u64,
    pub total_memory: u64,
}

pub struct WindowsScanResult {
    pub processes: Vec<WindowsProcessStat>,
    pub stats: WindowsStats,
}
