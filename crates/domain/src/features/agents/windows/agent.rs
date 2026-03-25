use app_contracts::features::environments::AgentClient;
use ogurpchik::discovery::Scope;
use ogurpchik::high::node::Node;
use ogurpchik::transport::stream::adapters::uds::UdsTransport;
use std::time::Instant;
use uniproc_protocol::{services, WindowsCodec, WindowsRequest};

pub async fn connect_windows_agent(timeout_secs: u64) -> anyhow::Result<AgentClient> {
    let client = Node::new()?
        .scope(Scope::Internal)?
        .connect::<WindowsCodec, _>(UdsTransport::temp("uniproc-windows"))
        .wait_for(services::WINDOWS_AGENT)
        .timeout(timeout_secs)
        .start()
        .await?;

    Ok(client)
}

pub async fn ping_windows_agent(client: AgentClient) -> anyhow::Result<i32> {
    let started = Instant::now();
    client.call(WindowsRequest::Ping).await?;
    Ok(started.elapsed().as_millis() as i32)
}
