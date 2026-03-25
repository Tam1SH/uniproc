use app_contracts::features::environments::AgentClient;
use ogurpchik::discovery::Scope;
use ogurpchik::high::node::Node;
use ogurpchik::transport::stream::adapters::uds::UdsTransport;
use std::time::Instant;
use uniproc_protocol::{LinuxCodec, LinuxRequest};

pub async fn connect_linux_agent(timeout_secs: u64) -> anyhow::Result<AgentClient> {
    let client = Node::new()?
        .scope(Scope::Internal)?
        .connect::<LinuxCodec, _>(UdsTransport::temp("uniproc"))
        .wait_for("uniproc")
        .timeout(timeout_secs)
        .start()
        .await?;

    Ok(client)
}

pub async fn ping_linux_agent(client: AgentClient) -> anyhow::Result<i32> {
    let started = Instant::now();
    client.call(LinuxRequest::Ping).await?;
    Ok(started.elapsed().as_millis() as i32)
}
