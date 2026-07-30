use crate::environments_impl::UiEnvironmentsBindings;
use crate::features::environments::wsl::domain::{
    check_wsl_availability_async, fetch_distros_data, inject_agent_async,
};
use app_contracts::features::environments::EnvironmentsPartialBinder;
use app_contracts::features::environments::{
    EnvironmentsBinder, UiEnvironmentsPort, UiEnvironmentsPortMsg, WslDistroDto,
};
use forsl_core::actor::{Context, ManagedActor};

use app_contracts::features::agents::{WslAgentRuntimeEvent, WslConnectionState};
use forsl_macros::handler;
use macros::actor_manifest;
use std::fmt::Debug;
use tracing::{error, info};

#[actor_manifest(binder = EnvironmentsBinder)]
impl<P: UiEnvironmentsPort> ManagedActor for WslEnvActor<P> {
    type Bus = bus!(WslAgentRuntimeEvent);
    type Handlers = handlers!(
        Init,
        #[bind]
        InstallAgent(String),
        CheckStatus,
        SetStatus(bool),
        RefreshDistros,
        UpdateDistros(Vec<WslDistroDto>)
    );
}

pub struct WslEnvActor<P: UiEnvironmentsPort> {
    distros: Vec<WslDistroDto>,
    ui_port: P,
}

impl<P: UiEnvironmentsPort> WslEnvActor<P> {
    pub fn new(ui_port: P) -> Self {
        Self {
            distros: Vec::new(),
            ui_port,
        }
    }

    fn sync_to_ui(&self) {
        self.ui_port
            .send(UiEnvironmentsPortMsg::SetWslDistros(self.distros.clone()));
    }

    fn set_distros(&mut self, updated: Vec<WslDistroDto>) {
        self.distros = updated;
        self.sync_to_ui();
    }

    fn apply_latency(&mut self, latency_ms: i32) {
        self.distros.iter_mut().for_each(|d| {
            d.latency_ms = latency_ms;
            d.is_installed = true;
        });
        self.sync_to_ui();
    }

    fn apply_disconnected(&mut self) {
        self.distros.iter_mut().for_each(|d| d.is_installed = false);
        self.sync_to_ui();
    }
}

#[handler]
fn handle_agent_runtime_event<P: UiEnvironmentsPort>(
    this: &mut WslEnvActor<P>,
    msg: WslAgentRuntimeEvent,
) {
    match msg.state {
        WslConnectionState::Connected => {
            if let Some(latency_ms) = msg.latency_ms {
                this.apply_latency(latency_ms);
            }
        }
        WslConnectionState::Disconnected | WslConnectionState::WaitingRetry { .. } => {
            this.apply_disconnected();
        }
        WslConnectionState::Connecting => {}
    }
}

#[handler]
fn init<P: UiEnvironmentsPort>(_: &WslEnvActor<P>, _: Init, ctx: &Context<WslEnvActor<P>>) {
    ctx.addr().send(CheckStatus);
}

#[handler]
fn check_status<P: UiEnvironmentsPort>(
    this: &WslEnvActor<P>,
    _: CheckStatus,
    ctx: &Context<WslEnvActor<P>>,
) {
    this.ui_port.send(UiEnvironmentsPortMsg::SetWslIsLoading(true));
    ctx.spawn_bg(async move { SetStatus(check_wsl_availability_async().await.unwrap_or(false)) });
}

#[handler]
fn set_status<P: UiEnvironmentsPort>(
    this: &WslEnvActor<P>,
    msg: SetStatus,
    ctx: &Context<WslEnvActor<P>>,
) {
    this.ui_port.send(UiEnvironmentsPortMsg::SetWslIsLoading(false));
    this.ui_port.send(UiEnvironmentsPortMsg::SetHasWsl(msg.0));
    if msg.0 {
        ctx.addr().send(RefreshDistros);
    }
}

#[handler]
fn refresh_distros<P: UiEnvironmentsPort>(
    this: &WslEnvActor<P>,
    _: RefreshDistros,
    ctx: &Context<WslEnvActor<P>>,
) {
    this.ui_port
        .send(UiEnvironmentsPortMsg::SetWslDistrosIsLoading(true));
    ctx.spawn_bg(async move {
        let distros = fetch_distros_data()
            .await
            .into_iter()
            .map(|d| WslDistroDto {
                name: d.name,
                is_installed: d.is_installed,
                is_running: d.is_running,
                latency_ms: d.latency_ms,
            })
            .collect();
        UpdateDistros(distros)
    });
}

#[handler]
fn update_distros<P: UiEnvironmentsPort>(this: &mut WslEnvActor<P>, msg: UpdateDistros) {
    this.ui_port
        .send(UiEnvironmentsPortMsg::SetWslDistrosIsLoading(false));
    this.set_distros(msg.0);
}

#[handler]
fn install_agent<P: UiEnvironmentsPort>(
    _: &WslEnvActor<P>,
    msg: InstallAgent,
    ctx: &Context<WslEnvActor<P>>,
) {
    let distro_name = msg.0;
    ctx.spawn_bg(async move {
        match inject_agent_async(&distro_name).await {
            Ok(_) => info!("Agent installed in {distro_name}"),
            Err(err) => error!("Failed to install agent in {distro_name}: {err}"),
        }
        RefreshDistros
    });
}
