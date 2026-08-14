#[derive(Clone, PartialEq, Debug, Default)]
pub struct MetricPoint {
    pub timestamp: u64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
}
