#[derive(Clone, Debug)]
pub struct RawDistroData {
    pub name: String,
    pub is_installed: bool,
    pub is_running: bool,
    pub latency_ms: i32,
}
