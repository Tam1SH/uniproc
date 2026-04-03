use super::backend::AgentBackend;
use crate::features::agents::connection::*;
use app_contracts::features::agents::ScanTick;
use app_contracts::features::environments::AgentConnectionState;
use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::{Context, Handler, Message, NoOp};
use app_core::app::Window;
use app_core::messages;
use context::settings::ReactiveSetting;
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

    fn spawn_connect<T: Window>(&self, ctx: &Context<Self, T>) {
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

impl<B, T> Handler<Init, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, _: Init, ctx: &Context<Self, T>) {
        info!("[{}] Actor init", B::NAME);
        self.publish_state(None);
        ctx.addr().send(StartConnect);
    }
}

impl<B, T> Handler<StartConnect, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, _: StartConnect, ctx: &Context<Self, T>) {
        if let Some(t) = self.apply(ConnectionEvent::BeginConnect)
            && t.to == AgentConnectionState::Connecting
        {
            self.publish_state(None);
            self.spawn_connect(ctx);
        }
    }
}

impl<B, T> Handler<ConnectResult<B::Client>, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, msg: ConnectResult<B::Client>, ctx: &Context<Self, T>) {
        match msg.0 {
            Some(client) => {
                if self.apply(ConnectionEvent::ConnectSucceeded).is_some() {
                    info!("[{}] Connected", B::NAME);
                    self.client = Some(client);
                    self.ping_in_flight = false;
                    self.publish_state(None);
                    ctx.addr().send(Ping);
                }
            }
            None => {
                if let Some(t) = self.apply(ConnectionEvent::ConnectFailed) {
                    self.client = None;
                    self.publish_state(None);
                    if let TransitionEffect::ScheduleRetry { delay_secs } = t.effect {
                        ctx.addr().send(TryConnectWithDelay(delay_secs));
                    }
                }
            }
        }
    }
}

impl<B, T> Handler<Ping, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, _: Ping, ctx: &Context<Self, T>) {
        let has_subs = EventBus::has_subscribers::<B::RuntimeEvent>();
        if !has_subs {
            return;
        }

        if !matches!(self.connection.state(), AgentConnectionState::Connected)
            || self.ping_in_flight
        {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        self.ping_in_flight = true;
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
}

impl<B, T> Handler<PingResult, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, msg: PingResult, ctx: &Context<Self, T>) {
        if !self.ping_in_flight {
            return;
        }
        self.ping_in_flight = false;
        match msg.0 {
            Some(ms) => self.publish_state(Some(ms)),
            None => ctx.addr().send(ConnectionLost),
        }
    }
}

impl<B, T> Handler<ScanTick, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, _: ScanTick, ctx: &Context<Self, T>) {
        if !matches!(self.connection.state(), AgentConnectionState::Connected) {
            return;
        }

        let Some(client) = self.client.clone() else {
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
}

impl<B, T> Handler<TryConnectWithDelay, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, msg: TryConnectWithDelay, ctx: &Context<Self, T>) {
        let secs = msg.0;
        ctx.spawn_bg(async move {
            tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
            RetryTimerElapsed
        });
    }
}

impl<B, T> Handler<RetryTimerElapsed, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, _: RetryTimerElapsed, ctx: &Context<Self, T>) {
        ctx.addr().send(StartConnect);
    }
}

impl<B, T> Handler<ConnectionLost, T> for GenericAgentActor<B>
where
    B: AgentBackend,
    T: Window,
{
    fn handle(&mut self, _: ConnectionLost, ctx: &Context<Self, T>) {
        if self.apply(ConnectionEvent::ConnectionLost).is_none() {
            return;
        }
        warn!("[{}] Connection lost", B::NAME);
        self.client = None;
        self.ping_in_flight = false;
        self.publish_state(None);
        ctx.addr().send(StartConnect);
    }
}

#[cfg(windows)]
mod windows {
    use crate::agents_impl::actor::GenericAgentActor;
    use crate::agents_impl::providers::windows::WindowsBackend;
    use app_contracts::features::agents::{WindowsActionRequest, WindowsActionResponse};
    use app_core::actor::event_bus::EventBus;
    use app_core::actor::traits::{Context, Handler};
    use app_core::app::Window;
    use std::ops::Deref;
    use tracing::error;
    use uniproc_protocol::WindowsResponse;

    impl<T: Window> Handler<WindowsActionRequest, T> for GenericAgentActor<WindowsBackend> {
        fn handle(&mut self, msg: WindowsActionRequest, _: &Context<Self, T>) {
            let Some(client) = self.client.clone() else {
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
                        if let Ok(response) = rkyv::deserialize::<
                            WindowsResponse,
                            rkyv::rancor::Error,
                        >(*resp_data.deref())
                        {
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
}
