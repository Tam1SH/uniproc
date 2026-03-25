use crate::features::environments::wsl::domain::{
    check_wsl_availability_async, fetch_distros_data, inject_agent_async,
};
use crate::messages;
use app_contracts::features::environments::{
    EnvironmentsUiPort, WslAgentRuntimeEvent, WslConnectionState, WslDistroDto,
};
use app_core::actor::traits::Message;
use app_core::actor::traits::{Context, Handler};
use slint::ComponentHandle;
use std::fmt::Debug;
use tracing::{error, info, instrument};

pub struct WslEnvActor<P: EnvironmentsUiPort> {
    distros: Vec<WslDistroDto>,
    ui_port: P,
}

impl<P: EnvironmentsUiPort> Debug for WslEnvActor<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WslEnvActor")
            .field("distros_count", &self.distros.len())
            .finish()
    }
}

impl<P: EnvironmentsUiPort> WslEnvActor<P> {
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

messages! {
    Init,
    InstallAgent(String),
    CheckStatus,
    SetStatus(bool),
    RefreshDistros,
    UpdateDistros(Vec<WslDistroDto>),
}

impl<P, TWindow> Handler<WslAgentRuntimeEvent, TWindow> for WslEnvActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: WslAgentRuntimeEvent, _ctx: &Context<Self, TWindow>) {
        match msg.state {
            WslConnectionState::Connected => {
                if let Some(latency_ms) = msg.latency_ms {
                    self.apply_latency(latency_ms);
                }
            }
            WslConnectionState::Disconnected | WslConnectionState::WaitingRetry { .. } => {
                self.apply_disconnected();
            }
            WslConnectionState::Connecting => {}
        }
    }
}

impl<P, TWindow> Handler<Init, TWindow> for WslEnvActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, _: Init, ctx: &Context<Self, TWindow>) {
        ctx.addr().send(CheckStatus);
    }
}

impl<P, TWindow> Handler<CheckStatus, TWindow> for WslEnvActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, _: CheckStatus, ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_is_loading(true);
        ctx.spawn_bg(
            async move { SetStatus(check_wsl_availability_async().await.unwrap_or(false)) },
        );
    }
}

impl<P, TWindow> Handler<SetStatus, TWindow> for WslEnvActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: SetStatus, ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_is_loading(false);
        self.ui_port.set_has_wsl(msg.0);
        if msg.0 {
            ctx.addr().send(RefreshDistros);
        }
    }
}

impl<P, TWindow> Handler<RefreshDistros, TWindow> for WslEnvActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, _: RefreshDistros, ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_distros_is_loading(true);
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
}

impl<P, TWindow> Handler<UpdateDistros, TWindow> for WslEnvActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: UpdateDistros, _ctx: &Context<Self, TWindow>) {
        self.ui_port.set_wsl_distros_is_loading(false);
        self.set_distros(msg.0);
    }
}

impl<P, TWindow> Handler<InstallAgent, TWindow> for WslEnvActor<P>
where
    P: EnvironmentsUiPort,
    TWindow: ComponentHandle + 'static,
{
    #[instrument(skip(self, ctx))]
    fn handle(&mut self, msg: InstallAgent, ctx: &Context<Self, TWindow>) {
        let distro_name = msg.0;
        ctx.spawn_bg(async move {
            match inject_agent_async(&distro_name).await {
                Ok(_) => info!("Agent installed in {distro_name}"),
                Err(err) => error!("Failed to install agent in {distro_name}: {err}"),
            }
            RefreshDistros
        });
    }
}
