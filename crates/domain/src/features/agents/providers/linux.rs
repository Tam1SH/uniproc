use crate::agents_impl::actor::{GenericAgentActor, Init, Ping};
use crate::agents_impl::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
use app_contracts::features::environments::{
    AgentClient, AgentConnectionState, WslAgentRuntimeEvent,
};
use app_core::app::Window;
use app_core::{
    SharedState,
    actor::{addr::Addr, event_bus::EVENT_BUS},
    app::Feature,
    reactor::Reactor,
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

    #[instrument(skip(client), level = "debug", fields(target = "linux"), err)]
    async fn perform_scan(client: &Self::Client) -> anyhow::Result<()> {
        let resp = client.call(LinuxRequest::GetReport).await?;

        let report = rkyv::deserialize::<LinuxResponse, rkyv::rancor::Error>(*resp.deref())
            .map_err(|e| {
                error!(error = %e, "Failed to deserialize Linux response");
                anyhow::anyhow!("Linux scan deserialization error: {}", e)
            })?;

        if let LinuxResponse::Report(r) = report {
            EVENT_BUS.with(|bus| {
                bus.publish(RemoteScanResult {
                    schema_id: "linux",
                    processes: r.processes,
                    machine: r.machine,
                })
            });
            ratelimit!(3600, info!("Report published to event bus"));
        } else {
            warn!(response = ?report, "Unexpected Linux response type â€” strange");
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
impl<T: Window> Feature<T> for LinuxAgentFeature {
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
