#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AgentPresence {
    Answering,
    Silent,
    NotChecked,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DistroRow {
    pub name: String,
    pub running: bool,
    pub agent: AgentPresence,
    pub metrics: Option<LinuxMachineSummary>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct LinuxMachineSummary {
    pub cpu_percent: Option<f32>,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_bytes: u64,
    pub net_bytes: u64,
    pub process_count: usize,
    pub container_count: usize,
}
