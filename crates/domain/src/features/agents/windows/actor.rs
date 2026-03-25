use crate::features::agents::connection::{
    ConnectionEvent, ConnectionMachine, ConnectionState, Transition, TransitionEffect,
};
use crate::features::agents::windows::agent::{connect_windows_agent, ping_windows_agent};
use crate::messages;
use app_contracts::features::environments::{
    AgentClient, AgentConnectionState, WindowsAgentRuntimeEvent,
};
use app_core::actor::event_bus::EVENT_BUS;
use app_core::actor::traits::{Context, Handler, Message};
use slint::ComponentHandle;
use std::fmt::Debug;
use tracing::{info, warn};

pub struct WindowsAgentActor {
    client: Option<AgentClient>,
    connection: ConnectionMachine,
    ping_in_flight: bool,
    connect_timeout_secs: u64,
}

impl Debug for WindowsAgentActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsAgentActor")
            .field("connection", &self.connection.state())
            .finish()
    }
}

impl WindowsAgentActor {
    pub fn new(connect_timeout_secs: u64) -> Self {
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
                    "WindowsAgent FSM invalid: {:?} on {:?}",
                    err.event, err.state
                );
                None
            }
        }
    }

    fn spawn_connect<TWindow: ComponentHandle + 'static>(&self, ctx: &Context<Self, TWindow>) {
        let timeout_secs = self.connect_timeout_secs;
        ctx.spawn_bg(async move {
            match connect_windows_agent(timeout_secs).await {
                Ok(c) => ConnectResult(Some(c)),
                Err(err) => {
                    warn!("Windows agent connect failed: {err}");
                    ConnectResult(None)
                }
            }
        });
    }

    fn publish(&self, latency_ms: Option<i32>) {
        let state: AgentConnectionState = self.connection.state().into();
        let client = self.client.clone();
        EVENT_BUS.with(|bus| {
            bus.publish(WindowsAgentRuntimeEvent {
                state,
                client,
                latency_ms,
            })
        });
    }
}

messages! {
    Init,
    Ping,
    StartConnect,
    TryConnectWithDelay(u64),
    RetryTimerElapsed,
    ConnectionLost,
    PingResult(Option<i32>),
}

pub struct ConnectResult(pub Option<AgentClient>);
impl Message for ConnectResult {}

impl<TWindow: ComponentHandle + 'static> Handler<Init, TWindow> for WindowsAgentActor {
    fn handle(&mut self, _: Init, ctx: &Context<Self, TWindow>) {
        info!("WindowsAgentActor init");
        self.publish(None);
        ctx.addr().send(StartConnect);
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<StartConnect, TWindow> for WindowsAgentActor {
    fn handle(&mut self, _: StartConnect, ctx: &Context<Self, TWindow>) {
        if let Some(t) = self.apply(ConnectionEvent::BeginConnect) {
            if t.to == ConnectionState::Connecting {
                self.publish(None);
                self.spawn_connect(ctx);
            }
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<ConnectResult, TWindow> for WindowsAgentActor {
    fn handle(&mut self, msg: ConnectResult, ctx: &Context<Self, TWindow>) {
        match msg.0 {
            Some(client) => {
                if self.apply(ConnectionEvent::ConnectSucceeded).is_some() {
                    info!("Windows agent connected");
                    self.client = Some(client);
                    self.ping_in_flight = false;
                    self.publish(None);
                    ctx.addr().send(Ping);
                }
            }
            None => {
                if let Some(t) = self.apply(ConnectionEvent::ConnectFailed) {
                    self.client = None;
                    self.publish(None);
                    if let TransitionEffect::ScheduleRetry { delay_secs } = t.effect {
                        ctx.addr().send(TryConnectWithDelay(delay_secs));
                    }
                }
            }
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<TryConnectWithDelay, TWindow>
    for WindowsAgentActor
{
    fn handle(&mut self, msg: TryConnectWithDelay, ctx: &Context<Self, TWindow>) {
        let secs = msg.0;
        ctx.spawn_bg(async move {
            tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
            RetryTimerElapsed
        });
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<RetryTimerElapsed, TWindow> for WindowsAgentActor {
    fn handle(&mut self, _: RetryTimerElapsed, ctx: &Context<Self, TWindow>) {
        if let Some(t) = self.apply(ConnectionEvent::RetryDelayElapsed) {
            if t.to == ConnectionState::Connecting {
                self.publish(None);
                self.spawn_connect(ctx);
            }
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<Ping, TWindow> for WindowsAgentActor {
    fn handle(&mut self, _: Ping, ctx: &Context<Self, TWindow>) {
        if !matches!(self.connection.state(), ConnectionState::Connected) || self.ping_in_flight {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        self.ping_in_flight = true;
        ctx.spawn_bg(async move {
            match ping_windows_agent(client).await {
                Ok(ms) => PingResult(Some(ms)),
                Err(err) => {
                    warn!("Windows agent ping failed: {err}");
                    PingResult(None)
                }
            }
        });
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<PingResult, TWindow> for WindowsAgentActor {
    fn handle(&mut self, msg: PingResult, ctx: &Context<Self, TWindow>) {
        if !self.ping_in_flight {
            return;
        }
        self.ping_in_flight = false;
        match msg.0 {
            Some(ms) => self.publish(Some(ms)),
            None => ctx.addr().send(ConnectionLost),
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<ConnectionLost, TWindow> for WindowsAgentActor {
    fn handle(&mut self, _: ConnectionLost, ctx: &Context<Self, TWindow>) {
        if self.apply(ConnectionEvent::ConnectionLost).is_none() {
            return;
        }
        warn!("Windows agent connection lost");
        self.client = None;
        self.ping_in_flight = false;
        self.publish(None);
        ctx.addr().send(StartConnect);
    }
}
