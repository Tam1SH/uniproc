use crate::agents_impl::actor::{GenericAgentActor, Init, Ping};
use crate::agents_impl::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
use app_contracts::features::environments::{
    AgentClient, AgentConnectionState, WslAgentRuntimeEvent,
};
use app_core::{
    actor::{addr::Addr, event_bus::EVENT_BUS},
    app::Feature,
    reactor::Reactor,
    SharedState,
};
use ogurpchik::discovery::Scope;
use ogurpchik::high::node::Node;
use ogurpchik::transport::stream::adapters::uds::UdsTransport;
use slint::ComponentHandle;
use uniproc_protocol::{LinuxCodec, LinuxRequest, LinuxResponse};

pub struct LinuxBackend;

impl AgentBackend for LinuxBackend {
    type Client = AgentClient;
    type RuntimeEvent = LinuxAgentRuntimeEvent;
    const NAME: &'static str = "Linux";

    async fn connect(timeout: u64) -> anyhow::Result<Self::Client> {
        Ok(Node::new()?
            .scope(Scope::Internal)?
            .connect::<LinuxCodec, _>(UdsTransport::temp("uniproc"))
            .wait_for("uniproc")
            .timeout(timeout)
            .start()
            .await?)
    }

    async fn ping(client: &Self::Client) -> anyhow::Result<i32> {
        let start = std::time::Instant::now();
        client.call(LinuxRequest::Ping).await?;
        Ok(start.elapsed().as_millis() as i32)
    }

    async fn perform_scan(client: &Self::Client) -> anyhow::Result<()> {
        let resp = client.call(LinuxRequest::GetReport).await?;
        if let Ok(LinuxResponse::Report(r)) =
            rkyv::deserialize::<LinuxResponse, rkyv::rancor::Error>(*resp.deref())
        {
            EVENT_BUS.with(|bus| {
                bus.publish(RemoteScanResult {
                    schema_id: "linux",
                    processes: r.processes,
                    machine: r.machine,
                })
            });
        }
        Ok(())
    }

    fn create_runtime_event(
        state: AgentConnectionState,
        latency: Option<i32>,
    ) -> Self::RuntimeEvent {
        LinuxAgentRuntimeEvent {
            state: state.into(),
            latency_ms: latency,
        }
    }
}

pub struct LinuxAgentFeature;
impl<T: ComponentHandle + 'static> Feature<T> for LinuxAgentFeature {
    fn install(self, reactor: &mut Reactor, ui: &T, shared: &SharedState) -> anyhow::Result<()> {
        let settings = AgentSettings::new(shared)?;
        let addr = Addr::new(
            GenericAgentActor::<LinuxBackend>::new(settings.connect_timeout_secs()?),
            ui.as_weak(),
        );
        let a = addr.clone();
        reactor.add_dynamic_loop(&settings.ping_interval_ms()?, move || a.send(Ping));
        EVENT_BUS.with(|bus| {
            bus.subscribe::<GenericAgentActor<LinuxBackend>, ScanTick, T>(addr.clone())
        });
        addr.send(Init);
        Ok(())
    }
}
