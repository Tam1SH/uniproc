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

/// Fixed vsock port the Linux agent listens on inside the VM.
const AGENT_VSOCK_PORT: u32 = 5000;

/// Tags this agent's reports on the bus, distinguishing them from the host's.
const SCHEMA_ID: &str = "wsl";

struct HostStub;
impl linux_agent::Server for HostStub {}

/// The agent, plus the child that owns it. Dropping the session kills the
/// launch, so the VM is not left with a stray agent once we disconnect.
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
        // `wsl.exe` is only a launcher - killing it does not necessarily reap
        // the process it started inside the VM, so ask the guest directly too.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts the agent inside the VM and hands it a one-shot HMAC secret.
///
/// vsock carries no peer identity across the VM boundary, so
/// `HandshakeMode::signed_process` cannot work here - ogurpchik refuses that
/// transport outright. Launching the agent ourselves and giving it a secret
/// nobody else saw is what takes its place: only the process we started can
/// answer, and if something already squats the port our own launch fails
/// loudly instead of us silently talking to an impostor.
///
/// The secret goes over stdin rather than argv or the environment, both of
/// which are readable from `/proc` by other processes in the VM.
///
/// The binary is expected to carry `cap_bpf,cap_net_admin,cap_perfmon,
/// cap_syslog+ep` (see `AgentSettings::wsl_agent_path`); without them it needs
/// root, and there is no password we could supply that is not the user's own.
fn launch_agent(distro: &str, agent_path: &str, secret: &str) -> anyhow::Result<Child> {
    // A leftover agent from a previous run still owns the port, and the new one
    // would fail to bind. Best-effort: a missing pkill or no match is fine.
    //
    // `-x` against the executable name, not `-f` against the full command line:
    // `-f` matches anything whose arguments merely mention the path, which
    // includes the very shell or wrapper that invoked the kill - that reliably
    // kills the wrong thing and leaves the actual agent running.
    let process_name = agent_path.rsplit(['/', '\\']).next().unwrap_or(agent_path);
    let _ = Command::new("wsl.exe")
        .args(["-d", distro, "--", "pkill", "-x", process_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let mut child = Command::new("wsl.exe")
        .args(["-d", distro, "--", agent_path])
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
    // Closing it is the signal that the secret is complete.
    drop(stdin);

    Ok(child)
}

/// 32 bytes of randomness, rendered as ASCII hex, from two v4 UUIDs so this
/// needs no extra dependency.
///
/// Hex rather than the raw bytes because the secret travels through `wsl.exe`'s
/// stdin, which is not a clean binary pipe - it is free to translate encodings
/// and line endings, and mangled key material shows up only as a handshake that
/// fails with no explanation. The hex *text* is what both sides feed to HMAC as
/// the key, so there is no decode step to disagree about either.
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

/// Where the agent lives, published by [`wsl_agent_feature`] before the actor
/// starts. `RpcHandle::connect` runs on its own thread, so this cannot ride
/// along in a thread-local.
static LAUNCH: OnceLock<(String, String)> = OnceLock::new();

/// Publishes where the agent lives. Called by [`wsl_agent_feature`] in the app;
/// exposed so an out-of-app probe (see `examples/agent_e2e.rs`) can stand the
/// connection up without booting the whole feature graph. First call wins.
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

        // The guest can only listen on vsock, so the host is always the side
        // that dials - and it has to resolve the VM's id first.
        let endpoint = Endpoint::vsock_to_best_vm(AGENT_VSOCK_PORT).map_err(|e| anyhow!("{e:?}"))?;

        // The agent has to bring eBPF up before it starts listening, so the
        // port is not there the instant the process exists - retry until the
        // timeout rather than failing on the first refusal.
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

    // Published before the actor starts, so the first connect already has it.
    set_launch_config(settings.wsl_distro().get(), settings.wsl_agent_path().get());

    let addr = ctx.spawn(GenericAgentActor::<WslBackend>::new(
        settings.wsl_connect_timeout_secs(),
    ));

    ctx.spawn_heartbeat(&addr, settings.ping_interval_ms(), || Ping);

    GlobalEventBus::subscribe::<GenericAgentActor<WslBackend>, ScanTick>(addr.clone());
    addr.send(Init);

    Ok(())
}
