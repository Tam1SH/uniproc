use crate::features::environments::wsl::domain::{
    check_wsl_availability_async, fetch_distros_data, inject_agent_async,
};
use crate::messages;
use app_contracts::features::environments::{
    UiEnvironmentsPort, WslAgentRuntimeEvent, WslConnectionState, WslDistroDto,
};
use app_core::actor::traits::{Context, Handler};
use app_core::app::Window;

use macros::handler;
use std::fmt::Debug;
use tracing::{error, info, instrument};

messages! {
    Init,
    InstallAgent(String),
    CheckStatus,
    SetStatus(bool),
    RefreshDistros,
    UpdateDistros(Vec<WslDistroDto>),
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
        self.ui_port.set_wsl_distros(self.distros.clone());
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
fn init<P: UiEnvironmentsPort>(_: &mut WslEnvActor<P>, _: Init, ctx: &Context<WslEnvActor<P>>) {
    ctx.addr().send(CheckStatus);
}

#[handler]
fn check_status<P: UiEnvironmentsPort>(
    this: &mut WslEnvActor<P>,
    _: CheckStatus,
    ctx: &Context<WslEnvActor<P>>,
) {
    this.ui_port.set_wsl_is_loading(true);
    ctx.spawn_bg(async move { SetStatus(check_wsl_availability_async().await.unwrap_or(false)) });
}

#[handler]
fn set_status<P: UiEnvironmentsPort>(
    this: &mut WslEnvActor<P>,
    msg: SetStatus,
    ctx: &Context<WslEnvActor<P>>,
) {
    this.ui_port.set_wsl_is_loading(false);
    this.ui_port.set_has_wsl(msg.0);
    if msg.0 {
        ctx.addr().send(RefreshDistros);
    }
}

#[handler]
fn refresh_distros<P: UiEnvironmentsPort>(
    this: &mut WslEnvActor<P>,
    _: RefreshDistros,
    ctx: &Context<WslEnvActor<P>>,
) {
    this.ui_port.set_wsl_distros_is_loading(true);
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
    this.ui_port.set_wsl_distros_is_loading(false);
    this.set_distros(msg.0);
}

#[handler]
fn install_agent<P: UiEnvironmentsPort>(
    _: &mut WslEnvActor<P>,
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
