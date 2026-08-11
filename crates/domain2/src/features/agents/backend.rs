use app_contracts2::features::agents::AgentConnectionState;
use guinea_core::actor::Message;

pub trait AgentBackend: Send + Sync + 'static {
    type Client: Clone + Send + Sync + 'static;
    type RuntimeEvent: Message + Send + Clone + 'static;
    type ScanMessage: Message + Send + Clone + 'static;

    const NAME: &'static str;

    fn connect(timeout_secs: u64) -> impl Future<Output=anyhow::Result<Self::Client>> + Send;
    fn ping(client: &Self::Client) -> impl Future<Output=anyhow::Result<i32>> + Send;
    fn perform_scan(client: &Self::Client) -> impl Future<Output=anyhow::Result<()>> + Send;

    fn create_runtime_event(state: AgentConnectionState, latency_ms: Option<i32>) -> Self::RuntimeEvent;

    fn scan_unavailable(state: AgentConnectionState) -> Self::ScanMessage;
}
