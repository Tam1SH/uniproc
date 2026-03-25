use app_contracts::features::environments::WslClient;
use ogurpchik::discovery::register_vm_default;
use ogurpchik::high::node::Node;
use ogurpchik::transport::stream::adapters::vsock::{VsockAddr, VsockTransport};
use std::time::Instant;
use uniproc_protocol::{HostCodec, HostRequest, services};

pub async fn connect_wsl_agent(timeout_secs: u64) -> anyhow::Result<WslClient> {
    register_vm_default("WSL").ok();

    let client = Node::new()?
        .connect::<HostCodec, _>(VsockTransport::client(VsockAddr::SelfManaged))
        .wait_for(services::GUEST)
        .timeout(timeout_secs)
        .start()
        .await?;

    Ok(client)
}

pub async fn ping_wsl_agent(client: WslClient) -> anyhow::Result<i32> {
    let started = Instant::now();
    client.call(HostRequest::Ping).await?;
    Ok(started.elapsed().as_millis() as i32)
}
