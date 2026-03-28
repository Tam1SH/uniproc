use crate::agents_impl::actor::{GenericAgentActor, Init, Ping};
use crate::agents_impl::backend::AgentBackend;
use crate::features::agents::settings::AgentSettings;
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
use app_contracts::features::environments::{
    AgentConnectionState, WslAgentRuntimeEvent, WslClient,
};
use app_core::{
    actor::{addr::Addr, event_bus::EVENT_BUS},
    app::Feature,
    reactor::Reactor,
    SharedState,
};
use ogurpchik::discovery::register_vm_default;
use ogurpchik::high::node::Node;
use ogurpchik::transport::stream::adapters::vsock::{VsockAddr, VsockTransport};
use slint::ComponentHandle;
use std::ops::Deref;
use std::time::Instant;
use uniproc_protocol::{services, LinuxCodec, LinuxRequest, LinuxResponse};

pub struct WslBackend;

impl AgentBackend for WslBackend {
    type Client = WslClient;
    type RuntimeEvent = WslAgentRuntimeEvent;
    const NAME: &'static str = "WSL";

    async fn connect(timeout: u64) -> anyhow::Result<Self::Client> {
        register_vm_default("WSL").ok();
        Ok(Node::new()?
            .connect::<LinuxCodec, _>(VsockTransport::client(VsockAddr::SelfManaged))
            .wait_for(services::LINUX_AGENT)
            .timeout(timeout)
            .start()
            .await?)
    }

    async fn ping(client: &Self::Client) -> anyhow::Result<i32> {
        let start = Instant::now();
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
                    schema_id: "wsl",
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
        WslAgentRuntimeEvent {
            state: state.into(),
            latency_ms: latency,
        }
    }
}

pub struct WslAgentFeature;
impl<T: ComponentHandle + 'static> Feature<T> for WslAgentFeature {
    fn install(self, reactor: &mut Reactor, ui: &T, shared: &SharedState) -> anyhow::Result<()> {
        let settings = AgentSettings::new(shared)?;
        let addr = Addr::new(
            GenericAgentActor::<WslBackend>::new(settings.connect_timeout_secs()?),
            ui.as_weak(),
        );
        let a = addr.clone();
        reactor.add_dynamic_loop(&settings.ping_interval_ms()?, move || a.send(Ping));
        EVENT_BUS
            .with(|bus| bus.subscribe::<GenericAgentActor<WslBackend>, ScanTick, T>(addr.clone()));
        addr.send(Init);
        Ok(())
    }
}
