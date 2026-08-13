use guinea_core::actor::Message;

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
