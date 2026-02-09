use crate::core::actor::traits::{Context, Handler, Message};
use crate::features::envs::get_icon_for_env;
use crate::features::envs::wsl::domain::*;
use crate::features::envs::wsl::RawDistroData;
use crate::{messages, WslDistro};
use crate::{AppWindow, EnvsLoading};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

pub struct WslActor;

messages! {
    Init,
    CheckStatus,
    SetStatus(bool),

    RefreshDistros,
    UpdateDistros(Vec<RawDistroData>),

    InstallAgent(SharedString),
}

impl Handler<Init, AppWindow> for WslActor {
    fn handle(&mut self, _msg: Init, ctx: &Context<Self, AppWindow>) {
        ctx.addr().send(CheckStatus);
    }
}

impl Handler<CheckStatus, AppWindow> for WslActor {
    fn handle(&mut self, _msg: CheckStatus, ctx: &Context<Self, AppWindow>) {
        ctx.spawn_task(
            async move {
                let has_wsl = check_wsl_availability_async().await.unwrap_or(false);
                SetStatus(has_wsl)
            },
            |ui, loading| ui.global::<EnvsLoading>().set_wsl_is_loading(loading),
        );
    }
}

impl Handler<SetStatus, AppWindow> for WslActor {
    fn handle(&mut self, msg: SetStatus, ctx: &Context<Self, AppWindow>) {
        let has_wsl = msg.0;
        ctx.with_ui(move |ui| ui.set_has_wsl(has_wsl));

        if has_wsl {
            ctx.addr().send(RefreshDistros);
        }
    }
}

impl Handler<RefreshDistros, AppWindow> for WslActor {
    fn handle(&mut self, _msg: RefreshDistros, ctx: &Context<Self, AppWindow>) {
        ctx.spawn_task(
            async move {
                let data = fetch_distros_logic().await;
                UpdateDistros(data)
            },
            |ui, loading| {
                ui.global::<EnvsLoading>()
                    .set_wsl_distros_is_loading(loading)
            },
        );
    }
}

impl Handler<UpdateDistros, AppWindow> for WslActor {
    fn handle(&mut self, msg: UpdateDistros, ctx: &Context<Self, AppWindow>) {
        ctx.with_ui(move |ui| {
            let distro_models: Vec<WslDistro> = msg
                .0
                .into_iter()
                .map(|rd| WslDistro {
                    name: rd.name.clone().into(),
                    is_installed: rd.is_installed,
                    is_running: rd.is_running,
                    icon: get_icon_for_env(&rd.name),
                })
                .collect();

            ui.set_wsl_distros(ModelRc::new(VecModel::from(distro_models)));
        });
    }
}

impl Handler<InstallAgent, AppWindow> for WslActor {
    fn handle(&mut self, msg: InstallAgent, ctx: &Context<Self, AppWindow>) {
        let distro_name = msg.0;
        ctx.spawn_bg(async move {
            let _ = inject_agent_async(&distro_name).await;
            RefreshDistros
        });
    }
}
