use crate::agents_impl::actor::{GenericAgentActor, Init, Ping};
use crate::agents_impl::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts::features::agents::{ScanTick, WindowsReportMessage};
use app_contracts::features::environments::{
    AgentClient, AgentConnectionState, WindowsAgentRuntimeEvent,
};
use app_core::app::Window;
use app_core::{
    SharedState,
    actor::{addr::Addr, event_bus::EventBus},
    app::Feature,
    ratelimit,
    reactor::Reactor,
};
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
            state: state,
            latency_ms: latency,
        }
    }
}

pub struct WindowsAgentFeature;
impl<T: Window> Feature<T> for WindowsAgentFeature {
    fn install(self, reactor: &mut Reactor, ui: &T, shared: &SharedState) -> anyhow::Result<()> {
        let settings = AgentSettings::new(shared)?;
        let addr = Addr::new(
            GenericAgentActor::<WindowsBackend>::new(settings.connect_timeout_secs()),
            ui.as_weak(),
        );
        let a = addr.clone();
        reactor.add_dynamic_loop(&settings.ping_interval_ms(), move || a.send(Ping));
        EventBus::subscribe::<GenericAgentActor<WindowsBackend>, ScanTick, T>(
            &ui.new_token(),
            addr.clone(),
        );

        addr.send(Init);
        Ok(())
    }
}
