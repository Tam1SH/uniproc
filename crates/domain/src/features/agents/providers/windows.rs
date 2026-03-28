use crate::agents_impl::actor::{GenericAgentActor, Init, Ping};
use crate::agents_impl::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts::features::agents::{ScanTick, WindowsReportMessage};
use app_contracts::features::environments::{
    AgentClient, AgentConnectionState, WindowsAgentRuntimeEvent,
};
use app_core::{
    actor::{addr::Addr, event_bus::EVENT_BUS},
    app::Feature,
    reactor::Reactor,
    SharedState,
};
use ogurpchik::discovery::Scope;
use ogurpchik::transport::stream::adapters::uds::UdsTransport;
use slint::ComponentHandle;
use std::ops::Deref;
use std::time::Instant;
use uniproc_protocol::{services, WindowsCodec, WindowsRequest, WindowsResponse};

pub struct WindowsBackend;

impl AgentBackend for WindowsBackend {
    type Client = AgentClient;
    type RuntimeEvent = WindowsAgentRuntimeEvent;
    const NAME: &'static str = "Windows";

    async fn connect(timeout: u64) -> anyhow::Result<Self::Client> {
        Ok(ogurpchik::high::node::Node::new()?
            .scope(Scope::Internal)?
            .connect::<WindowsCodec, _>(UdsTransport::temp("uniproc-windows"))
            .wait_for(services::WINDOWS_AGENT)
            .timeout(timeout)
            .start()
            .await?)
    }

    async fn ping(client: &Self::Client) -> anyhow::Result<i32> {
        let start = Instant::now();
        client.call(WindowsRequest::Ping).await?;
        Ok(start.elapsed().as_millis() as i32)
    }

    async fn perform_scan(client: &Self::Client) -> anyhow::Result<()> {
        let resp = client.call(WindowsRequest::GetReport).await?;
        if let Ok(WindowsResponse::Report(r)) =
            rkyv::deserialize::<WindowsResponse, rkyv::rancor::Error>(*resp.deref())
        {
            EVENT_BUS.with(|bus| bus.publish(WindowsReportMessage(r)));
        }
        Ok(())
    }

    fn create_runtime_event(
        state: AgentConnectionState,
        latency: Option<i32>,
    ) -> Self::RuntimeEvent {
        WindowsAgentRuntimeEvent {
            state: state.into(),
            client: None,
            latency_ms: latency,
        }
    }
}

pub struct WindowsAgentFeature;
impl<T: ComponentHandle + 'static> Feature<T> for WindowsAgentFeature {
    fn install(self, reactor: &mut Reactor, ui: &T, shared: &SharedState) -> anyhow::Result<()> {
        let settings = AgentSettings::new(shared)?;
        let addr = Addr::new(
            GenericAgentActor::<WindowsBackend>::new(settings.connect_timeout_secs()?),
            ui.as_weak(),
        );
        let a = addr.clone();
        reactor.add_dynamic_loop(&settings.ping_interval_ms()?, move || a.send(Ping));
        EVENT_BUS.with(|bus| {
            bus.subscribe::<GenericAgentActor<WindowsBackend>, ScanTick, T>(addr.clone())
        });
        addr.send(Init);
        Ok(())
    }
}
