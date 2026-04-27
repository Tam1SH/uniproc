use super::backend::AgentBackend;
use crate::features::agents::connection::*;
use app_contracts::features::agents::ScanTick;
use app_contracts::features::environments::AgentConnectionState;
use app_core::actor::event_bus::EventBus;
use app_core::actor::{Context, Message, NoOp};
use app_core::messages;
use framework::settings::ReactiveSetting;
use macros::handler;
use std::fmt::Debug;
use tracing::{info, warn};

messages! {
    Init,
    Ping,
    StartConnect,
    TryConnectWithDelay(u64),
    RetryTimerElapsed,
    ConnectionLost,
    PingResult(Option<i32>)
}

struct ConnectResult<C>(Option<C>);
impl<C: Send + 'static> Message for ConnectResult<C> {}

pub struct GenericAgentActor<B: AgentBackend> {
    client: Option<B::Client>,
    connection: ConnectionMachine,
    ping_in_flight: bool,
    connect_timeout_secs: ReactiveSetting<u64>,
}

impl<B: AgentBackend> GenericAgentActor<B> {
    pub fn new(connect_timeout_secs: ReactiveSetting<u64>) -> Self {
        Self {
            client: None,
            connection: ConnectionMachine::new(),
            ping_in_flight: false,
            connect_timeout_secs,
        }
    }

    fn apply(&mut self, event: ConnectionEvent) -> Option<Transition> {
        match self.connection.apply(event) {
            Ok(t) => Some(t),
            Err(err) => {
                warn!(
                    "[{}] FSM invalid: {:?} on {:?}",
                    B::NAME,
                    err.event,
                    err.state
                );
                None
            }
        }
    }

    fn publish_state(&self, latency_ms: Option<i32>) {
        let event = B::create_runtime_event(self.connection.state(), latency_ms);
        EventBus::publish(event);
    }

    fn spawn_connect(&self, ctx: &Context<Self>) {
        let timeout = self.connect_timeout_secs.get().max(1);
        ctx.spawn_bg(async move {
            match B::connect(timeout).await {
                Ok(client) => ConnectResult(Some(client)),
                Err(err) => {
                    warn!("[{}] Connect failed: {err}", B::NAME);
                    ConnectResult(None)
                }
            }
        });
    }
}

#[handler]
fn init<B: AgentBackend>(
    this: &mut GenericAgentActor<B>,
    _: Init,
    ctx: &Context<GenericAgentActor<B>>,
) {
    info!("[{}] Actor init", B::NAME);
    this.publish_state(None);
    ctx.addr().send(StartConnect);
}

#[handler]
fn start_connect<B: AgentBackend>(
    this: &mut GenericAgentActor<B>,
    _: StartConnect,
    ctx: &Context<GenericAgentActor<B>>,
) {
    if let Some(t) = this.apply(ConnectionEvent::BeginConnect)
        && t.to == AgentConnectionState::Connecting
    {
        this.publish_state(None);
        this.spawn_connect(ctx);
    }
}

#[handler]
fn on_connect_result<B: AgentBackend>(
    this: &mut GenericAgentActor<B>,
    msg: ConnectResult<B::Client>,
    ctx: &Context<GenericAgentActor<B>>,
) {
    match msg.0 {
        Some(client) => {
            if this.apply(ConnectionEvent::ConnectSucceeded).is_some() {
                info!("[{}] Connected", B::NAME);
                this.client = Some(client);
                this.ping_in_flight = false;
                this.publish_state(None);
                ctx.addr().send(Ping);
            }
        }
        None => {
            if let Some(t) = this.apply(ConnectionEvent::ConnectFailed) {
                this.client = None;
                this.publish_state(None);
                if let TransitionEffect::ScheduleRetry { delay_secs } = t.effect {
                    ctx.addr().send(TryConnectWithDelay(delay_secs));
                }
            }
        }
    }
}

#[handler]
fn ping<B: AgentBackend>(
    this: &mut GenericAgentActor<B>,
    _: Ping,
    ctx: &Context<GenericAgentActor<B>>,
) {
    let has_subs = EventBus::has_subscribers::<B::RuntimeEvent>();
    if !has_subs {
        return;
    }

    if !matches!(this.connection.state(), AgentConnectionState::Connected) || this.ping_in_flight {
        return;
    }
    let Some(client) = this.client.clone() else {
        return;
    };
    this.ping_in_flight = true;
    ctx.spawn_bg(async move {
        match B::ping(&client).await {
            Ok(ms) => PingResult(Some(ms)),
            Err(err) => {
                warn!("[{}] Ping failed: {err}", B::NAME);
                PingResult(None)
            }
        }
    });
}

#[handler]
fn on_ping_result<B: AgentBackend>(
    this: &mut GenericAgentActor<B>,
    msg: PingResult,
    ctx: &Context<GenericAgentActor<B>>,
) {
    if !this.ping_in_flight {
        return;
    }
    this.ping_in_flight = false;
    match msg.0 {
        Some(ms) => this.publish_state(Some(ms)),
        None => ctx.addr().send(ConnectionLost),
    }
}

#[handler]
fn perform_scan_tick<B: AgentBackend>(
    this: &mut GenericAgentActor<B>,
    _: ScanTick,
    ctx: &Context<GenericAgentActor<B>>,
) {
    if !matches!(this.connection.state(), AgentConnectionState::Connected) {
        return;
    }

    let Some(client) = this.client.clone() else {
        warn!("[{}] client is None (unexpected state)", B::NAME);
        return;
    };

    ctx.spawn_bg(async move {
        if let Err(err) = B::perform_scan(&client).await {
            warn!("[{}] Scan failed: {err}", B::NAME);
        }
        NoOp
    });
}

#[handler]
fn schedule_retry<B: AgentBackend>(
    _: &mut GenericAgentActor<B>,
    msg: TryConnectWithDelay,
    ctx: &Context<GenericAgentActor<B>>,
) {
    let secs = msg.0;
    ctx.spawn_bg(async move {
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        RetryTimerElapsed
    });
}

#[handler]
fn on_retry_elapsed<B: AgentBackend>(
    _: &mut GenericAgentActor<B>,
    _: RetryTimerElapsed,
    ctx: &Context<GenericAgentActor<B>>,
) {
    ctx.addr().send(StartConnect);
}

#[handler]
fn on_connection_lost<B: AgentBackend>(
    this: &mut GenericAgentActor<B>,
    _: ConnectionLost,
    ctx: &Context<GenericAgentActor<B>>,
) {
    if this.apply(ConnectionEvent::ConnectionLost).is_none() {
        return;
    }
    warn!("[{}] Connection lost", B::NAME);
    this.client = None;
    this.ping_in_flight = false;
    this.publish_state(None);
    ctx.addr().send(StartConnect);
}

#[cfg(windows)]
mod windows {
    use super::*;
    use crate::agents_impl::providers::windows::WindowsBackend;
    use app_contracts::features::agents::{WindowsActionRequest, WindowsActionResponse};
    use std::ops::Deref;
    use tracing::error;
    use uniproc_protocol::WindowsResponse;

    #[handler]
    fn handle_windows_action(
        this: &mut GenericAgentActor<WindowsBackend>,
        msg: WindowsActionRequest,
    ) {
        let Some(client) = this.client.clone() else {
            error!("Client not initialized");
            return;
        };

        let correlation_id = msg.correlation_id;
        let request = match msg.decode_request() {
            Ok(request) => request,
            Err(err) => {
                error!("Failed to decode backend request: {:?}", err);
                return;
            }
        };

        tokio::spawn(async move {
            match client.call(request).await {
                Ok(resp_data) => {
                    if let Ok(response) = rkyv::deserialize::<WindowsResponse, rkyv::rancor::Error>(
                        *resp_data.deref(),
                    ) {
                        EventBus::publish(WindowsActionResponse::new(correlation_id, &response));
                    }
                }
                Err(e) => {
                    error!("Backend call failed: {:?}", e);
                }
            }
        });
    }
}
