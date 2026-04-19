use crate::agents_impl::actor::{GenericAgentActor, Init, Ping};
use crate::agents_impl::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts::features::agents::{ScanTick, WindowsReportMessage};
use app_contracts::features::environments::{
    AgentClient, AgentConnectionState, WindowsAgentRuntimeEvent,
};
use app_core::feature::{AppFeature, AppFeatureInitContext};
use app_core::lifecycle_tracker::FeatureLifecycle;
use app_core::{
    actor::{addr::Addr, event_bus::EventBus},
    ratelimit,
};
use ogurpchik::discovery::Scope;
use ogurpchik::transport::stream::adapters::uds::UdsTransport;
use std::ops::Deref;
use std::time::Instant;
use tracing::{error, instrument, warn};
use uniproc_protocol::{services, WindowsCodec, WindowsRequest, WindowsResponse};

pub struct WindowsBackend;

impl AgentBackend for WindowsBackend {
    type Client = AgentClient;
    type RuntimeEvent = WindowsAgentRuntimeEvent;
    const NAME: &'static str = "Windows";

    async fn connect(timeout: u64) -> anyhow::Result<Self::Client> {
        ogurpchik::high::node::Node::new()?
            .scope(Scope::Internal)?
            .connect::<WindowsCodec, _>(UdsTransport::temp("uniproc-windows"))
            .wait_for(services::WINDOWS_AGENT)
            .timeout(timeout)
            .start()
            .await
    }

    async fn ping(client: &Self::Client) -> anyhow::Result<i32> {
        let start = Instant::now();
        client.call(WindowsRequest::Ping).await?;
        Ok(start.elapsed().as_millis() as i32)
    }

    #[instrument(skip(client), level = "debug", err)]
    async fn perform_scan(client: &Self::Client) -> anyhow::Result<()> {
        let resp = client.call(WindowsRequest::GetReport).await?;

        let response = rkyv::deserialize::<WindowsResponse, rkyv::rancor::Error>(*resp.deref())
            .map_err(|e| {
                error!(error = %e, "Deserialization failed");
                anyhow::anyhow!("Failed to deserialize WindowsResponse: {}", e)
            })?;

        if let WindowsResponse::Report(r) = response {
            EventBus::publish(WindowsReportMessage(r));
            ratelimit!(3600, info!("Report published to event bus"));
        } else {
            warn!("Unexpected response type: {:?}", response);
        }

        Ok(())
    }

    fn create_runtime_event(
        state: AgentConnectionState,
        latency: Option<i32>,
    ) -> Self::RuntimeEvent {
        WindowsAgentRuntimeEvent {
            state,
            latency_ms: latency,
        }
    }
}

pub struct WindowsAgentFeature;
impl AppFeature for WindowsAgentFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let settings = AgentSettings::new(ctx.shared)?;

        let addr = Addr::new(
            GenericAgentActor::<WindowsBackend>::new(settings.connect_timeout_secs()),
            ctx.token.clone(),
            &FeatureLifecycle::new(),
        );

        let a = addr.clone();
        ctx.reactor
            .add_dynamic_loop(settings.ping_interval_ms().as_signal(), move || {
                a.send(Ping)
            });

        let _ = EventBus::subscribe::<GenericAgentActor<WindowsBackend>, ScanTick>(
            addr.clone(),
            &FeatureLifecycle::new(),
        );

        addr.send(Init);
        Ok(())
    }
}
