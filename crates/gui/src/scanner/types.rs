#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub exe_path: String,
    pub command_line: String,
    pub disk_read: u64,
    pub disk_write: u64,
    pub parent_pid: u32,
    pub net_usage: u64,
    pub user_name: String,
}
#[derive(Debug, Clone)]

pub struct ModelMachineStats {
    pub cpu_percent: f32,
    pub ram_percent: f32,
    pub disk_percent: f32,
    pub net_percent: f32,
    pub net_total_bandwidth: u64,
    pub total_memory: u64,
}
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub processes: Vec<ProcessInfo>,
    pub stats: ModelMachineStats,
}

pub trait ProcessScanner {
    fn scan(&mut self) -> ScanResult;
}
