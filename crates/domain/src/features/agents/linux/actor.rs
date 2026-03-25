use crate::features::agents::connection::{
    ConnectionEvent, ConnectionMachine, ConnectionState, Transition, TransitionEffect,
};
use crate::features::agents::linux::agent::{connect_linux_agent, ping_linux_agent};
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
use app_contracts::features::environments::{
    AgentClient, AgentConnectionState, LinuxAgentRuntimeEvent,
};
use app_core::actor::event_bus::EVENT_BUS;
use app_core::actor::traits::{Context, Handler, Message};
use app_core::messages;
use app_core::settings::ReactiveSetting;
use slint::ComponentHandle;
use std::ops::Deref;
use tracing::{info, warn};
use uniproc_protocol::{LinuxMachineStats, LinuxProcessStats, LinuxRequest, LinuxResponse};

pub struct LinuxAgentActor {
    client: Option<AgentClient>,
    connection: ConnectionMachine,
    ping_in_flight: bool,
    connect_timeout_secs: ReactiveSetting<u64>,
}

impl LinuxAgentActor {
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
                warn!("LinuxAgent FSM invalid: {:?} on {:?}", err.event, err.state);
                None
            }
        }
    }

    fn spawn_connect<TWindow: ComponentHandle + 'static>(&self, ctx: &Context<Self, TWindow>) {
        let timeout_secs = self.connect_timeout_secs.get().max(1);
        ctx.spawn_bg(async move {
            match connect_linux_agent(timeout_secs).await {
                Ok(c) => ConnectResult(Some(c)),
                Err(err) => {
                    warn!("Linux agent connect failed: {err}");
                    ConnectResult(None)
                }
            }
        });
    }

    fn publish_state(&self, latency_ms: Option<i32>) {
        let state: AgentConnectionState = self.connection.state().into();
        EVENT_BUS.with(|bus| bus.publish(LinuxAgentRuntimeEvent { state, latency_ms }));
    }
}

impl From<ConnectionState> for AgentConnectionState {
    fn from(value: ConnectionState) -> Self {
        match value {
            ConnectionState::Disconnected => Self::Disconnected,
            ConnectionState::Connecting => Self::Connecting,
            ConnectionState::Connected => Self::Connected,
            ConnectionState::WaitingRetry { delay_secs } => Self::WaitingRetry { delay_secs },
        }
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

struct ConnectResult(Option<AgentClient>);
impl Message for ConnectResult {}

struct ReportResult(
    Option<(
        Vec<LinuxProcessStats>,
        LinuxMachineStats,
    )>,
);
impl Message for ReportResult {}

impl<TWindow: ComponentHandle + 'static> Handler<Init, TWindow> for LinuxAgentActor {
    fn handle(&mut self, _: Init, ctx: &Context<Self, TWindow>) {
        info!("LinuxAgentActor init");
        self.publish_state(None);
        ctx.addr().send(StartConnect);
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<ScanTick, TWindow> for LinuxAgentActor {
    fn handle(&mut self, _: ScanTick, ctx: &Context<Self, TWindow>) {
        if !matches!(self.connection.state(), ConnectionState::Connected) {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        ctx.spawn_bg(async move {
            let response = match client.call(LinuxRequest::GetReport).await {
                Ok(r) => r,
                Err(err) => {
                    warn!("Linux GetReport failed: {err}");
                    return ReportResult(None);
                }
            };
            match rkyv::deserialize::<LinuxResponse, rkyv::rancor::Error>(*response.deref()) {
                Ok(LinuxResponse::Report(report)) => {
                    ReportResult(Some((report.processes, report.machine)))
                }
                _ => ReportResult(None),
            }
        });
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<ReportResult, TWindow> for LinuxAgentActor {
    fn handle(&mut self, msg: ReportResult, ctx: &Context<Self, TWindow>) {
        match msg.0 {
            Some((processes, machine)) => {
                EVENT_BUS.with(|bus| {
                    bus.publish(RemoteScanResult {
                        schema_id: "linux",
                        processes,
                        machine,
                    })
                });
            }
            None => ctx.addr().send(ConnectionLost),
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<StartConnect, TWindow> for LinuxAgentActor {
    fn handle(&mut self, _: StartConnect, ctx: &Context<Self, TWindow>) {
        if let Some(t) = self.apply(ConnectionEvent::BeginConnect) {
            if t.to == ConnectionState::Connecting {
                self.publish_state(None);
                self.spawn_connect(ctx);
            }
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<ConnectResult, TWindow> for LinuxAgentActor {
    fn handle(&mut self, msg: ConnectResult, ctx: &Context<Self, TWindow>) {
        match msg.0 {
            Some(client) => {
                if self.apply(ConnectionEvent::ConnectSucceeded).is_some() {
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

impl<TWindow: ComponentHandle + 'static> Handler<TryConnectWithDelay, TWindow> for LinuxAgentActor {
    fn handle(&mut self, msg: TryConnectWithDelay, ctx: &Context<Self, TWindow>) {
        let secs = msg.0;
        ctx.spawn_bg(async move {
            std::thread::sleep(std::time::Duration::from_secs(secs));
            RetryTimerElapsed
        });
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<RetryTimerElapsed, TWindow> for LinuxAgentActor {
    fn handle(&mut self, _: RetryTimerElapsed, ctx: &Context<Self, TWindow>) {
        if let Some(t) = self.apply(ConnectionEvent::RetryDelayElapsed) {
            if t.to == ConnectionState::Connecting {
                self.publish_state(None);
                self.spawn_connect(ctx);
            }
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<Ping, TWindow> for LinuxAgentActor {
    fn handle(&mut self, _: Ping, ctx: &Context<Self, TWindow>) {
        if !matches!(self.connection.state(), ConnectionState::Connected) || self.ping_in_flight {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        self.ping_in_flight = true;
        ctx.spawn_bg(async move {
            match ping_linux_agent(client).await {
                Ok(ms) => PingResult(Some(ms)),
                Err(err) => {
                    warn!("Linux ping failed: {err}");
                    PingResult(None)
                }
            }
        });
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<PingResult, TWindow> for LinuxAgentActor {
    fn handle(&mut self, msg: PingResult, ctx: &Context<Self, TWindow>) {
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

impl<TWindow: ComponentHandle + 'static> Handler<ConnectionLost, TWindow> for LinuxAgentActor {
    fn handle(&mut self, _: ConnectionLost, ctx: &Context<Self, TWindow>) {
        if self.apply(ConnectionEvent::ConnectionLost).is_none() {
            return;
        }
        self.client = None;
        self.ping_in_flight = false;
        self.publish_state(None);
        ctx.addr().send(StartConnect);
    }
}
