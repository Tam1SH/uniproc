use crate::features::environments::wsl::agent::{connect_wsl_agent, ping_wsl_agent};
use crate::features::environments::wsl::domain::{
    check_wsl_availability_async, fetch_distros_data, inject_agent_async,
};
use crate::features::environments::wsl::state::{
    ConnectionEvent, ConnectionMachine, ConnectionState, Transition, TransitionEffect,
};
use crate::messages;
use app_contracts::features::environments::{
    EnvironmentsUiPort, WslAgentRuntimeEvent, WslClient, WslConnectionState, WslDistroDto,
};
use app_core::actor::event_bus::EVENT_BUS;
use app_core::actor::traits::{Context, Handler, Message};
use slint::ComponentHandle;
use std::fmt::Debug;
use tracing::{debug, error, info, instrument, trace, warn};

pub struct WslActor<P: EnvironmentsUiPort> {
    client: Option<WslClient>,
    connection: ConnectionMachine,
    ping_in_flight: bool,
    connect_timeout_secs: u64,
    distros: Vec<WslDistroDto>,
    ui_port: P,
}

impl<P: EnvironmentsUiPort> Debug for WslActor<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WslActor")
            .field("connection", &self.connection.state())
            .field("ping_in_flight", &self.ping_in_flight)
            .finish()
    }
}

impl<P: EnvironmentsUiPort> WslActor<P> {
    pub fn new(connect_timeout_secs: u64, ui_port: P) -> Self {
        Self {
            client: None,
            connection: ConnectionMachine::new(),
            ping_in_flight: false,
            connect_timeout_secs,
            distros: Vec::new(),
            ui_port,
        }
    }

    fn apply_transition(&mut self, event: ConnectionEvent) -> Option<Transition> {
        match self.connection.apply(event) {
            Ok(transition) => {
                debug!(
                    "WSL FSM transition: {:?} --{:?}--> {:?} (effect: {:?})",
                    transition.from, transition.event, transition.to, transition.effect
                );
                Some(transition)
            }
            Err(err) => {
                warn!(
                    "WSL FSM invalid transition: state={:?}, event={:?}",
                    err.state, err.event
                );
                None
            }
        }
    }

    fn spawn_connect_now<TWindow: ComponentHandle + 'static>(&self, ctx: &Context<Self, TWindow>) {
        let timeout_secs = self.connect_timeout_secs;
        ctx.spawn_bg(async move {
            match connect_wsl_agent(timeout_secs).await {
                Ok(client) => ConnectResult(Some(client)),
                Err(err) => {
                    warn!("Failed to connect to WSL guest: {err}");
                    ConnectResult(None)
                }
            }
        });
    }

    fn sync_distros_to_ui(&self) {
        self.ui_port.set_wsl_distros(self.distros.clone());
    }

    fn set_distros(&mut self, updated: Vec<WslDistroDto>) {
        self.distros = updated;
        self.sync_distros_to_ui();
    }

    fn update_all_distro_install_state(&mut self, is_installed: bool) {
        self.distros.iter_mut().for_each(|distro| {
            distro.is_installed = is_installed;
        });
        self.sync_distros_to_ui();
    }

    fn apply_ping_latency(&mut self, latency_ms: i32) {
        self.distros.iter_mut().for_each(|distro| {
            distro.latency_ms = latency_ms;
            distro.is_installed = true;
        });
        self.sync_distros_to_ui();
    }

    fn apply_latency_updates(&mut self, updates: &[(String, i32)]) {
        self.distros.iter_mut().for_each(|distro| {
            if let Some((_, latency)) = updates
                .iter()
                .find(|(name, _)| name.as_str() == distro.name.as_str())
            {
                distro.latency_ms = *latency;
            }
        });
        self.sync_distros_to_ui();
    }

    fn publish_runtime(&self) {
        EVENT_BUS.with(|bus| {
            bus.publish(WslAgentRuntimeEvent {
                state: self.connection.state().into(),
                client: self.client.clone(),
            })
        });
    }
}

impl From<ConnectionState> for WslConnectionState {
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
    StartConnect,
    CheckStatus,
    SetStatus(bool),
    RefreshDistros,
    UpdateDistros(Vec<WslDistroDto>),
    ConnectionLost,
    UpdateLatency(Vec<(String, i32)>),
    InstallAgent(String),
    Ping,
    PingResult(Option<i32>),
    TryConnectWithDelay(u64),
    RetryTimerElapsed,
}

pub struct ConnectResult(pub Option<WslClient>);
impl Message for ConnectResult {}

impl<P, TWindow> Handler<Init, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: Init, ctx: &Context<Self, TWindow>) {
        info!("Initializing WSL actor");
        self.publish_runtime();
        ctx.addr().send(CheckStatus);
        ctx.addr().send(StartConnect);
    }
}

impl<P, TWindow> Handler<StartConnect, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: StartConnect, ctx: &Context<Self, TWindow>) {
        if let Some(transition) = self.apply_transition(ConnectionEvent::BeginConnect) {
            if transition.to == ConnectionState::Connecting {
                self.publish_runtime();
                self.spawn_connect_now(ctx);
            }
        }
    }
}

impl<P, TWindow> Handler<ConnectResult, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx, msg))]
    fn handle(&mut self, msg: ConnectResult, ctx: &Context<Self, TWindow>) {
        match msg.0 {
            Some(client) => {
                if self
                    .apply_transition(ConnectionEvent::ConnectSucceeded)
                    .is_some()
                {
                    info!("WSL client connected successfully");
                    self.client = Some(client);
                    self.ping_in_flight = false;
                    self.publish_runtime();
                    ctx.addr().send(CheckStatus);
                    ctx.addr().send(Ping);
                }
            }
            None => {
                if let Some(transition) = self.apply_transition(ConnectionEvent::ConnectFailed) {
                    self.client = None;
                    self.publish_runtime();
                    if let TransitionEffect::ScheduleRetry { delay_secs } = transition.effect {
                        ctx.addr().send(TryConnectWithDelay(delay_secs));
                    }
                }
            }
        }
    }
}

impl<P, TWindow> Handler<TryConnectWithDelay, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: TryConnectWithDelay, ctx: &Context<Self, TWindow>) {
        let delay_secs = msg.0;
        debug!("Scheduling reconnect timer for {delay_secs}s");

        ctx.spawn_bg(async move {
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            RetryTimerElapsed
        });
    }
}

impl<P, TWindow> Handler<RetryTimerElapsed, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: RetryTimerElapsed, ctx: &Context<Self, TWindow>) {
        if let Some(transition) = self.apply_transition(ConnectionEvent::RetryDelayElapsed) {
            if transition.to == ConnectionState::Connecting {
                self.publish_runtime();
                self.spawn_connect_now(ctx);
            }
        }
    }
}

impl<P, TWindow> Handler<Ping, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: Ping, ctx: &Context<Self, TWindow>) {
        if !matches!(self.connection.state(), ConnectionState::Connected) {
            trace!("Skipping ping while state is not connected");
            return;
        }

        if self.ping_in_flight {
            trace!("Skipping ping because previous ping is still in flight");
            return;
        }

        let Some(client) = self.client.clone() else {
            debug!("Cannot ping WSL guest without active client");
            return;
        };

        self.ping_in_flight = true;

        ctx.spawn_bg(async move {
            match ping_wsl_agent(client).await {
                Ok(latency) => PingResult(Some(latency)),
                Err(err) => {
                    warn!("Ping failed: {err}");
                    PingResult(None)
                }
            }
        });
    }
}

impl<P, TWindow> Handler<PingResult, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: PingResult, ctx: &Context<Self, TWindow>) {
        if !self.ping_in_flight {
            trace!("Ignoring ping result because no ping is currently in flight");
            return;
        }

        self.ping_in_flight = false;

        match msg.0 {
            Some(latency_ms) => {
                trace!("Received ping latency {latency_ms}ms");
                self.apply_ping_latency(latency_ms);
            }
            None => {
                warn!("Ping failed, marking connection as lost");
                ctx.addr().send(ConnectionLost);
            }
        }
    }
}

impl<P, TWindow> Handler<ConnectionLost, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: ConnectionLost, ctx: &Context<Self, TWindow>) {
        if self
            .apply_transition(ConnectionEvent::ConnectionLost)
            .is_none()
        {
            return;
        }

        warn!("WSL connection lost");
        self.client = None;
        self.ping_in_flight = false;
        self.publish_runtime();

        self.update_all_distro_install_state(false);
        ctx.addr().send(StartConnect);
    }
}

impl<P, TWindow> Handler<UpdateLatency, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: UpdateLatency, _ctx: &Context<Self, TWindow>) {
        self.apply_latency_updates(&msg.0);
    }
}

impl<P, TWindow> Handler<CheckStatus, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: CheckStatus, ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_is_loading(true);
        ctx.spawn_bg(
            async move { SetStatus(check_wsl_availability_async().await.unwrap_or(false)) },
        );
    }
}

impl<P, TWindow> Handler<SetStatus, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: SetStatus, ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_is_loading(false);
        let has_wsl = msg.0;
        self.ui_port.set_has_wsl(has_wsl);

        if has_wsl {
            ctx.addr().send(RefreshDistros);
        }
    }
}

impl<P, TWindow> Handler<RefreshDistros, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: RefreshDistros, ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_distros_is_loading(true);
        ctx.spawn_bg(async move {
            let distros = fetch_distros_data()
                .await
                .into_iter()
                .map(|distro| WslDistroDto {
                    name: distro.name,
                    is_installed: distro.is_installed,
                    is_running: distro.is_running,
                    latency_ms: distro.latency_ms,
                })
                .collect();
            UpdateDistros(distros)
        });
    }
}

impl<P, TWindow> Handler<UpdateDistros, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: UpdateDistros, _ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_distros_is_loading(false);
        self.set_distros(msg.0);
    }
}

impl<P, TWindow> Handler<InstallAgent, TWindow> for WslActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: InstallAgent, ctx: &Context<Self, TWindow>) {
        let distro_name = msg.0;

        ctx.spawn_bg(async move {
            match inject_agent_async(&distro_name).await {
                Ok(_) => info!("Successfully installed agent in {distro_name}"),
                Err(err) => error!("Failed to install agent in {distro_name}: {err}"),
            }

            RefreshDistros
        });
    }
}
