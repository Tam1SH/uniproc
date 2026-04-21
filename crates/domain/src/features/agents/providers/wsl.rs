use crate::agents_impl::actor::{GenericAgentActor, Init, Ping};
use crate::agents_impl::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
use app_contracts::features::environments::{
    AgentConnectionState, WslAgentRuntimeEvent, WslClient,
};
use app_core::actor::event_bus::EventBus;
use app_core::feature::{AppFeature, AppFeatureInitContext};
use app_core::lifecycle_tracker::FeatureLifecycle;
use app_core::{actor::addr::Addr, ratelimit};
use ogurpchik::discovery::register_vm_default;
use ogurpchik::high::node::Node;
use ogurpchik::transport::stream::adapters::vsock::{VsockAddr, VsockTransport};
use std::ops::Deref;
use std::time::Instant;
use tracing::{error, instrument, warn};
use uniproc_protocol::{LinuxCodec, LinuxRequest, LinuxResponse, services};

pub struct WslBackend;

impl AgentBackend for WslBackend {
    type Client = WslClient;
    type RuntimeEvent = WslAgentRuntimeEvent;
    const NAME: &'static str = "WSL";

    async fn connect(timeout: u64) -> anyhow::Result<Self::Client> {
        register_vm_default("WSL").ok();
        Node::new()?
            .connect::<LinuxCodec, _>(VsockTransport::client(VsockAddr::SelfManaged))
            .wait_for(services::LINUX_AGENT)
            .timeout(timeout)
            .start()
            .await
    }

    async fn ping(client: &Self::Client) -> anyhow::Result<i32> {
        let start = Instant::now();
        client.call(LinuxRequest::Ping).await?;
        Ok(start.elapsed().as_millis() as i32)
    }

    #[instrument(skip(client), level = "debug", fields(target = "wsl"), err)]
    async fn perform_scan(client: &Self::Client) -> anyhow::Result<()> {
        let resp = client.call(LinuxRequest::GetReport).await?;

        let report = rkyv::deserialize::<LinuxResponse, rkyv::rancor::Error>(*resp.deref())
            .map_err(|e| {
                error!(error = %e, "Failed to deserialize WSL response");
                anyhow::anyhow!("WSL scan deserialization error: {}", e)
            })?;

        if let LinuxResponse::Report(r) = report {
            EventBus::publish(RemoteScanResult {
                schema_id: "wsl",
                processes: r.processes,
                machine: r.machine,
                environments: r.environments,
                docker_containers: r.docker_containers,
            });

            ratelimit!(3600, info!("Report published to event bus"));
        } else {
            warn!(response = ?report, "Unexpected WSL response type â€” strange");
        }

        Ok(())
    }

    fn create_runtime_event(
        state: AgentConnectionState,
        latency: Option<i32>,
    ) -> Self::RuntimeEvent {
        WslAgentRuntimeEvent {
            state: state,
            latency_ms: latency,
        }
    }
}

pub struct WslAgentFeature;
impl AppFeature for WslAgentFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let settings = AgentSettings::new(ctx.shared)?;

        let addr = Addr::new(
            GenericAgentActor::<WslBackend>::new(settings.connect_timeout_secs()),
            ctx.token.clone(),
            &FeatureLifecycle::new(),
        );

        let a = addr.clone();
        ctx.reactor
            .add_dynamic_loop(settings.ping_interval_ms().as_signal(), move || {
                a.send(Ping)
            });

        EventBus::subscribe::<GenericAgentActor<WslBackend>, ScanTick>(
            addr.clone(),
            &FeatureLifecycle::new(),
        );
        addr.send(Init);
        Ok(())
    }
}
