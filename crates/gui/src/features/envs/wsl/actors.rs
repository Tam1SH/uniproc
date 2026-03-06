use crate::core::actor::event_bus::EVENT_BUS;
use crate::core::actor::traits::{Context, Handler, Message};
use crate::features::envs::get_icon_for_env;
use crate::features::envs::wsl::agent::{WslClient, connect_wsl_agent, ping_wsl_agent};
use crate::features::envs::wsl::domain::{
    RawDistroData, check_wsl_availability_async, fetch_distros_data, inject_agent_async,
};
use crate::features::envs::wsl::state::{
    ConnectionEvent, ConnectionMachine, ConnectionState, Transition, TransitionEffect,
};
use crate::{AppWindow, EnvironmentsFeatureGlobal, EnvsLoading, WslDistro, messages};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::fmt::Debug;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Clone)]
pub struct WslAgentRuntimeEvent {
    pub state: ConnectionState,
    pub client: Option<WslClient>,
}
impl Message for WslAgentRuntimeEvent {}

pub struct WslActor {
    client: Option<WslClient>,
    connection: ConnectionMachine,
    ping_in_flight: bool,
    connect_timeout_secs: u64,
}

impl Debug for WslActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WslActor")
            .field("connection", &self.connection.state())
            .field("ping_in_flight", &self.ping_in_flight)
            .finish()
    }
}

impl WslActor {
    pub fn new(connect_timeout_secs: u64) -> Self {
        Self {
            client: None,
            connection: ConnectionMachine::new(),
            ping_in_flight: false,
            connect_timeout_secs,
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

    fn spawn_connect_now(&self, ctx: &Context<Self, AppWindow>) {
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

    fn replace_distro_models(ctx: &Context<Self, AppWindow>, updated: Vec<WslDistro>) {
        ctx.with_ui(move |ui| {
            ui.global::<EnvironmentsFeatureGlobal>()
                .set_wsl_distros(ModelRc::new(VecModel::from(updated)));
        });
    }

    fn update_all_distro_install_state(ctx: &Context<Self, AppWindow>, is_installed: bool) {
        ctx.with_ui(move |ui| {
            let global = ui.global::<EnvironmentsFeatureGlobal>();
            let updated: Vec<WslDistro> = global
                .get_wsl_distros()
                .iter()
                .map(|mut distro| {
                    distro.is_installed = is_installed;
                    distro
                })
                .collect();

            global.set_wsl_distros(ModelRc::new(VecModel::from(updated)));
        });
    }

    fn publish_runtime(&self) {
        EVENT_BUS.with(|bus| {
            bus.publish(WslAgentRuntimeEvent {
                state: self.connection.state(),
                client: self.client.clone(),
            })
        });
    }
}

messages! {
    Init,
    StartConnect,
    CheckStatus,
    SetStatus(bool),
    RefreshDistros,
    UpdateDistros(Vec<RawDistroData>),
    ConnectionLost,
    UpdateLatency(Vec<(String, i32)>),
    InstallAgent(SharedString),
    Ping,
    PingResult(Option<i32>),
    TryConnectWithDelay(u64),
    RetryTimerElapsed,
}

pub struct ConnectResult(pub Option<WslClient>);
impl Message for ConnectResult {}

impl Handler<Init, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: Init, ctx: &Context<Self, AppWindow>) {
        info!("Initializing WSL actor");
        self.publish_runtime();
        ctx.addr().send(CheckStatus);
        ctx.addr().send(StartConnect);
    }
}

impl Handler<StartConnect, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: StartConnect, ctx: &Context<Self, AppWindow>) {
        if let Some(transition) = self.apply_transition(ConnectionEvent::BeginConnect) {
            if transition.to == ConnectionState::Connecting {
                self.publish_runtime();
                self.spawn_connect_now(ctx);
            }
        }
    }
}

impl Handler<ConnectResult, AppWindow> for WslActor {
    #[instrument(skip(self, ctx, msg))]
    fn handle(&mut self, msg: ConnectResult, ctx: &Context<Self, AppWindow>) {
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

impl Handler<TryConnectWithDelay, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: TryConnectWithDelay, ctx: &Context<Self, AppWindow>) {
        let delay_secs = msg.0;
        debug!("Scheduling reconnect timer for {delay_secs}s");

        ctx.spawn_bg(async move {
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            RetryTimerElapsed
        });
    }
}

impl Handler<RetryTimerElapsed, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: RetryTimerElapsed, ctx: &Context<Self, AppWindow>) {
        if let Some(transition) = self.apply_transition(ConnectionEvent::RetryDelayElapsed) {
            if transition.to == ConnectionState::Connecting {
                self.publish_runtime();
                self.spawn_connect_now(ctx);
            }
        }
    }
}

impl Handler<Ping, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: Ping, ctx: &Context<Self, AppWindow>) {
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

impl Handler<PingResult, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: PingResult, ctx: &Context<Self, AppWindow>) {
        if !self.ping_in_flight {
            trace!("Ignoring ping result because no ping is currently in flight");
            return;
        }

        self.ping_in_flight = false;

        match msg.0 {
            Some(latency_ms) => {
                trace!("Received ping latency {latency_ms}ms");
                ctx.with_ui(move |ui| {
                    let global = ui.global::<EnvironmentsFeatureGlobal>();
                    let updated: Vec<WslDistro> = global
                        .get_wsl_distros()
                        .iter()
                        .map(|mut distro| {
                            distro.latency_ms = latency_ms;
                            distro.is_installed = true;
                            distro
                        })
                        .collect();

                    global.set_wsl_distros(ModelRc::new(VecModel::from(updated)));
                });
            }
            None => {
                warn!("Ping failed, marking connection as lost");
                ctx.addr().send(ConnectionLost);
            }
        }
    }
}

impl Handler<ConnectionLost, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: ConnectionLost, ctx: &Context<Self, AppWindow>) {
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

        Self::update_all_distro_install_state(ctx, false);
        ctx.addr().send(StartConnect);
    }
}

impl Handler<UpdateLatency, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: UpdateLatency, ctx: &Context<Self, AppWindow>) {
        ctx.with_ui(move |ui| {
            let global = ui.global::<EnvironmentsFeatureGlobal>();
            let updated: Vec<WslDistro> = global
                .get_wsl_distros()
                .iter()
                .map(|mut distro| {
                    if let Some((_, latency)) = msg
                        .0
                        .iter()
                        .find(|(name, _)| name.as_str() == distro.name.as_str())
                    {
                        distro.latency_ms = *latency;
                    }
                    distro
                })
                .collect();

            global.set_wsl_distros(ModelRc::new(VecModel::from(updated)));
        });
    }
}

impl Handler<CheckStatus, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: CheckStatus, ctx: &Context<Self, AppWindow>) {
        ctx.spawn_task(
            async move { SetStatus(check_wsl_availability_async().await.unwrap_or(false)) },
            |ui, loading| {
                ui.global::<EnvsLoading>().set_wsl_is_loading(loading);
            },
        );
    }
}

impl Handler<SetStatus, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: SetStatus, ctx: &Context<Self, AppWindow>) {
        let has_wsl = msg.0;
        ctx.with_ui(move |ui| {
            ui.global::<EnvironmentsFeatureGlobal>()
                .set_has_wsl(has_wsl);
        });

        if has_wsl {
            ctx.addr().send(RefreshDistros);
        }
    }
}

impl Handler<RefreshDistros, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _msg: RefreshDistros, ctx: &Context<Self, AppWindow>) {
        ctx.spawn_task(
            async move { UpdateDistros(fetch_distros_data().await) },
            |ui, loading| {
                ui.global::<EnvsLoading>()
                    .set_wsl_distros_is_loading(loading);
            },
        );
    }
}

impl Handler<UpdateDistros, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: UpdateDistros, ctx: &Context<Self, AppWindow>) {
        let updated: Vec<WslDistro> = msg
            .0
            .into_iter()
            .map(|distro| WslDistro {
                name: distro.name.clone().into(),
                is_installed: distro.is_installed,
                is_running: distro.is_running,
                icon: get_icon_for_env(&distro.name),
                latency_ms: distro.latency_ms,
            })
            .collect();

        Self::replace_distro_models(ctx, updated);
    }
}

impl Handler<InstallAgent, AppWindow> for WslActor {
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: InstallAgent, ctx: &Context<Self, AppWindow>) {
        let distro_name = msg.0.to_string();

        ctx.spawn_bg(async move {
            match inject_agent_async(&distro_name).await {
                Ok(_) => info!("Successfully installed agent in {distro_name}"),
                Err(err) => error!("Failed to install agent in {distro_name}: {err}"),
            }

            RefreshDistros
        });
    }
}
