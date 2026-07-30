use crate::features::agents::actor::{GenericAgentActor, Init, Ping};
use crate::features::agents::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts2::features::agents::{AgentClient, AgentConnectionState, ScanTick, WindowsAgentRuntimeEvent, WindowsReportMessage};
use guinea::feature::{AppFeature, AppFeatureInitContext, ContextActorExt, ContextReactorExt, ContextStoreExt};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::ratelimit;
use guinea_macros::app_feature;
use ogurpchik::discovery::Scope;
use ogurpchik::transport::stream::adapters::uds::UdsTransport;
use std::ops::Deref;
use std::time::Instant;
use tracing::{error, instrument, warn};
use uniproc_protocol::{WindowsCodec, WindowsRequest, WindowsResponse, services};

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

        let response = rkyv::deserialize::<WindowsResponse, rkyv::rancor::Error>(*resp.deref()).map_err(|e| {
            error!(error = %e, "Deserialization failed");
            anyhow::anyhow!("Failed to deserialize WindowsResponse: {}", e)
        })?;

        if let WindowsResponse::Report(r) = response {
            GlobalEventBus::publish(WindowsReportMessage(r));
            ratelimit!(3600, info!("Report published to event bus"));
        } else {
            warn!("Unexpected response type: {:?}", response);
        }

        Ok(())
    }

    fn create_runtime_event(state: AgentConnectionState, latency: Option<i32>) -> Self::RuntimeEvent {
        WindowsAgentRuntimeEvent { state, latency_ms: latency }
    }
}

#[app_feature]
pub fn windows_agent_feature(ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
    let store = ctx.store();
    let settings = AgentSettings::new_with(&store)?;

    let addr = ctx.spawn(GenericAgentActor::<WindowsBackend>::new(settings.connect_timeout_secs()));

    ctx.spawn_heartbeat(&addr, settings.ping_interval_ms(), || Ping);

    GlobalEventBus::subscribe::<GenericAgentActor<WindowsBackend>, ScanTick>(addr.clone());
    addr.send(Init);

    Ok(())
}
