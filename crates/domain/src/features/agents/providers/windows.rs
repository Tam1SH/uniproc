use crate::features::agents::actor::{GenericAgentActor, Init, Ping};
use crate::features::agents::backend::AgentBackend;
use crate::features::agents::decode;
use crate::features::agents::rpc::{RpcHandle, RpcService};
use crate::features::agents::settings::AgentSettings;
use anyhow::{anyhow, bail};
use app_contracts::features::agents::{
    AgentConnectionState, ScanTick, WindowsAction, WindowsActionRequest, WindowsAgentRuntimeEvent, WindowsReport,
    WindowsReportMessage,
};
use guinea::app::FeatureBuilder;
use guinea::feature::{ContextActorExt, ContextReactorExt};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::ratelimit;
use ogurpchik::auth::handshake::{HandshakeMode, authenticate_client};
use ogurpchik::endpoint::Endpoint;
use ogurpchik::rpc::{RpcSession, Side, spawn_session};
use std::rc::Rc;
use std::time::{Duration, Instant};
use tracing::instrument;
use uniproc_protocol::windows_capnp::windows_agent;

const APP_NAME: &str = "uniproc";
const AGENT_SERVICE_NAME: &str = "windows-agent";

struct HostStub;
impl windows_agent::Server for HostStub {}

pub enum WindowsRequest {
    Ping,
    GetReport,
    Action(WindowsAction),
}

pub enum WindowsReply {
    Pong,
    Report(WindowsReport),
    Code(u32),
}

pub struct WindowsRpc;

impl RpcService for WindowsRpc {
    type Session = Rc<RpcSession<windows_agent::Client>>;
    type Request = WindowsRequest;
    type Reply = WindowsReply;

    const NAME: &'static str = "Windows";

    async fn connect(timeout_secs: u64) -> anyhow::Result<Self::Session> {
        let endpoint =
            Endpoint::for_service(APP_NAME, AGENT_SERVICE_NAME).map_err(|e| anyhow!("{e:?}"))?;

        let mut conn = endpoint
            .connect_ready(Duration::from_secs(timeout_secs))
            .await
            .map_err(|e| anyhow!("{e:?}"))?;

        authenticate_client(&mut conn, &HandshakeMode::version_only())
            .await
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(Rc::new(spawn_session::<windows_agent::Client, _>(
            conn,
            Side::Client,
            HostStub,
        )))
    }

    async fn dispatch(session: Self::Session, request: Self::Request) -> anyhow::Result<Self::Reply> {
        let client = session.remote();

        match request {
            WindowsRequest::Ping => {
                client.ping_request().send().promise.await?;
                Ok(WindowsReply::Pong)
            }
            WindowsRequest::GetReport => {
                let reply = client.get_report_request().send().promise.await?;
                let report = decode::windows_report(reply.get()?.get_report()?)?;
                Ok(WindowsReply::Report(report))
            }
            WindowsRequest::Action(action) => {
                let code = perform_action(client, action).await?;
                Ok(WindowsReply::Code(code))
            }
        }
    }
}

async fn perform_action(client: &windows_agent::Client, action: WindowsAction) -> anyhow::Result<u32> {
    macro_rules! by_pid {
        ($request:ident, $pid:expr) => {{
            let mut req = client.$request();
            req.get().set_pid($pid);
            req.send().promise.await?.get()?.get_code()
        }};
    }
    macro_rules! by_name {
        ($request:ident, $name:expr) => {{
            let mut req = client.$request();
            req.get().set_name(&$name);
            req.send().promise.await?.get()?.get_code()
        }};
    }

    let code = match action {
        WindowsAction::Kill { pid } => by_pid!(kill_request, pid),
        WindowsAction::Suspend { pid } => by_pid!(suspend_request, pid),
        WindowsAction::Resume { pid } => by_pid!(resume_request, pid),
        WindowsAction::SetPriority { pid, priority } => {
            let mut req = client.set_priority_request();
            req.get().set_pid(pid);
            req.get().set_priority(decode::priority(priority));
            req.send().promise.await?.get()?.get_code()
        }
        WindowsAction::SetAffinity { pid, mask } => {
            let mut req = client.set_affinity_request();
            req.get().set_pid(pid);
            req.get().set_mask(mask);
            req.send().promise.await?.get()?.get_code()
        }
        WindowsAction::ServiceStart { name } => by_name!(service_start_request, name),
        WindowsAction::ServiceStop { name } => by_name!(service_stop_request, name),
        WindowsAction::ServicePause { name } => by_name!(service_pause_request, name),
        WindowsAction::ServiceResume { name } => by_name!(service_resume_request, name),
        WindowsAction::ServiceRestart { name } => by_name!(service_restart_request, name),
    };

    Ok(code)
}

pub struct WindowsBackend;

impl AgentBackend for WindowsBackend {
    type Client = RpcHandle<WindowsRpc>;
    type RuntimeEvent = WindowsAgentRuntimeEvent;
    type ScanMessage = WindowsReportMessage;
    const NAME: &'static str = "Windows";

    async fn connect(timeout: u64) -> anyhow::Result<Self::Client> {
        RpcHandle::connect(timeout).await
    }

    async fn ping(client: &Self::Client) -> anyhow::Result<i32> {
        let start = Instant::now();
        match client.call(WindowsRequest::Ping).await? {
            WindowsReply::Pong => Ok(start.elapsed().as_millis() as i32),
            _ => bail!("agent answered a ping with something else"),
        }
    }

    #[instrument(skip(client), level = "debug", err)]
    async fn perform_scan(client: &Self::Client) -> anyhow::Result<()> {
        match client.call(WindowsRequest::GetReport).await? {
            WindowsReply::Report(report) => {
                GlobalEventBus::publish(WindowsReportMessage::Report(report));
                ratelimit!(3600, info!("Report published to event bus"));
                Ok(())
            }
            _ => bail!("agent answered getReport with something else"),
        }
    }

    fn create_runtime_event(state: AgentConnectionState, latency: Option<i32>) -> Self::RuntimeEvent {
        WindowsAgentRuntimeEvent { state, latency_ms: latency }
    }

    fn scan_unavailable(state: AgentConnectionState) -> Self::ScanMessage {
        WindowsReportMessage::Unavailable(state)
    }
}

pub fn windows_agent_feature(app: &mut FeatureBuilder) -> anyhow::Result<()> {
    let settings = AgentSettings::new()?;
    let ping_interval = settings.ping_interval_ms();

    let addr = app.spawn(GenericAgentActor::<WindowsBackend>::new(
        settings.connect_timeout_secs(),
    ));

    app.spawn_heartbeat(&addr, move || ping_interval.get(), || Ping);

    app.subscribe_actor::<_, ScanTick>(&addr);
    app.subscribe_actor::<_, WindowsActionRequest>(&addr);
    addr.send(Init);

    Ok(())
}
