use crate::features::agents::actor::{GenericAgentActor, Init, Ping};
use crate::features::agents::backend::AgentBackend;
use crate::features::agents::decode;
use crate::features::agents::rpc::{RpcHandle, RpcService};
use crate::features::agents::settings::AgentSettings;
use anyhow::{anyhow, bail};
use app_contracts2::features::agents::{
    AgentConnectionState, LinuxReport, RemoteScan, RemoteScanResult, ScanTick, WslAgentRuntimeEvent,
};
use guinea::feature::{AppFeature, AppFeatureInitContext, ContextActorExt, ContextReactorExt, ContextStoreExt};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::ratelimit;
use guinea_macros::app_feature;
use ogurpchik::auth::handshake::{HandshakeMode, authenticate_client};
use ogurpchik::endpoint::Endpoint;
use ogurpchik::rpc::{RpcSession, Side, spawn_session};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tracing::instrument;
use uniproc_protocol::linux_capnp::linux_agent;
use uuid::Uuid;

const AGENT_VSOCK_PORT: u32 = 5000;

const SCHEMA_ID: &str = "wsl";

struct HostStub;
impl linux_agent::Server for HostStub {}

pub struct WslSession {
    rpc: RpcSession<linux_agent::Client>,
    child: Child,
}

impl WslSession {
    fn remote(&self) -> &linux_agent::Client {
        self.rpc.remote()
    }
}

impl Drop for WslSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn launch_agent(distro: &str, agent_path: &str, secret: &str) -> anyhow::Result<Child> {
    let process_name = agent_path.rsplit(['/', '\\']).next().unwrap_or(agent_path);
    let _ = Command::new("wsl.exe")
        .args(["-d", distro, "-u", "root", "--", "pkill", "-x", process_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let mut child = Command::new("wsl.exe")
        .args(["-d", distro, "-u", "root", "--", agent_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("failed to launch the WSL agent via wsl.exe: {e}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("wsl.exe gave us no stdin to write the secret to"))?;
    stdin.write_all(secret.as_bytes())?;
    drop(stdin);

    Ok(child)
}

fn generate_secret() -> String {
    let mut secret = String::with_capacity(64);
    for byte in Uuid::new_v4().as_bytes().iter().chain(Uuid::new_v4().as_bytes()) {
        secret.push_str(&format!("{byte:02x}"));
    }
    secret
}

pub enum WslRequest {
    Ping,
    GetReport,
}

pub enum WslReply {
    Pong,
    Report(LinuxReport),
}

pub struct WslRpc;

static LAUNCH: OnceLock<(String, String)> = OnceLock::new();

pub fn set_launch_config(distro: impl Into<String>, agent_path: impl Into<String>) {
    let _ = LAUNCH.set((distro.into(), agent_path.into()));
}

impl RpcService for WslRpc {
    type Session = Rc<WslSession>;
    type Request = WslRequest;
    type Reply = WslReply;

    const NAME: &'static str = "WSL";

    async fn connect(timeout_secs: u64) -> anyhow::Result<Self::Session> {
        let (distro, agent_path) = LAUNCH
            .get()
            .ok_or_else(|| anyhow!("WSL launch settings were never published"))?;

        let secret = generate_secret();
        let child = launch_agent(distro, agent_path, &secret)?;

        let endpoint = Endpoint::vsock_to_best_vm(AGENT_VSOCK_PORT).map_err(|e| anyhow!("{e:?}"))?;

        let mut conn = endpoint
            .connect_ready(Duration::from_secs(timeout_secs))
            .await
            .map_err(|e| anyhow!("{e:?}"))?;

        authenticate_client(&mut conn, &HandshakeMode::hmac(secret.into_bytes()))
            .await
            .map_err(|e| anyhow!("{e:?}"))?;

        Ok(Rc::new(WslSession {
            rpc: spawn_session::<linux_agent::Client, _>(conn, Side::Client, HostStub),
            child,
        }))
    }

    async fn dispatch(session: Self::Session, request: Self::Request) -> anyhow::Result<Self::Reply> {
        let client = session.remote();

        match request {
            WslRequest::Ping => {
                client.ping_request().send().promise.await?;
                Ok(WslReply::Pong)
            }
            WslRequest::GetReport => {
                let reply = client.get_report_request().send().promise.await?;
                let report = decode::linux_report(reply.get()?.get_report()?)?;
                Ok(WslReply::Report(report))
            }
        }
    }
}

pub struct WslBackend;

impl AgentBackend for WslBackend {
    type Client = RpcHandle<WslRpc>;
    type RuntimeEvent = WslAgentRuntimeEvent;
    type ScanMessage = RemoteScanResult;
    const NAME: &'static str = "WSL";

    async fn connect(timeout: u64) -> anyhow::Result<Self::Client> {
        RpcHandle::connect(timeout).await
    }

    async fn ping(client: &Self::Client) -> anyhow::Result<i32> {
        let start = Instant::now();
        match client.call(WslRequest::Ping).await? {
            WslReply::Pong => Ok(start.elapsed().as_millis() as i32),
            _ => bail!("WSL agent answered a ping with something else"),
        }
    }

    #[instrument(skip(client), level = "debug", fields(target = "wsl"), err)]
    async fn perform_scan(client: &Self::Client) -> anyhow::Result<()> {
        match client.call(WslRequest::GetReport).await? {
            WslReply::Report(report) => {
                GlobalEventBus::publish(RemoteScanResult::Scan(RemoteScan {
                    schema_id: SCHEMA_ID,
                    processes: report.processes,
                    machine: report.machine,
                    environments: report.environments,
                    docker_containers: report.docker_containers,
                }));
                ratelimit!(3600, info!("Report published to event bus"));
                Ok(())
            }
            _ => bail!("WSL agent answered getReport with something else"),
        }
    }

    fn create_runtime_event(state: AgentConnectionState, latency: Option<i32>) -> Self::RuntimeEvent {
        WslAgentRuntimeEvent { state, latency_ms: latency }
    }

    fn scan_unavailable(state: AgentConnectionState) -> Self::ScanMessage {
        RemoteScanResult::Unavailable(state)
    }
}

#[app_feature]
pub fn wsl_agent_feature(ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
    let store = ctx.store();
    let settings = AgentSettings::new_with(&store)?;

    set_launch_config(settings.wsl_distro().get(), settings.wsl_agent_path().get());

    let addr = ctx.spawn(GenericAgentActor::<WslBackend>::new(
        settings.wsl_connect_timeout_secs(),
    ));

    ctx.spawn_heartbeat(&addr, settings.ping_interval_ms(), || Ping);

    GlobalEventBus::subscribe::<GenericAgentActor<WslBackend>, ScanTick>(addr.clone());
    addr.send(Init);

    Ok(())
}
