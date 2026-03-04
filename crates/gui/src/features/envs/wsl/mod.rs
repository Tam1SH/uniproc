use crate::core::actor::addr::Addr;
use crate::core::reactor::Reactor;
use crate::features::envs::wsl::actors::{Init, InstallAgent, Ping, WslActor};
use crate::features::Feature;
use crate::{AppWindow, EnvironmentsFeatureGlobal};
use slint::ComponentHandle;
use std::time::Duration;

mod actors;
mod agent;
mod domain;
mod state;

pub struct WslFeature;

impl Feature for WslFeature {
    fn install(self, reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        let addr = Addr::new(WslActor::new(), ui.as_weak());

        let a = addr.clone();
        reactor.add_loop(Duration::from_millis(500), move || a.send(Ping));

        addr.send(Init);

        let global = ui.global::<EnvironmentsFeatureGlobal>();
        global.on_install_agent(addr.handler_with(InstallAgent));

        Ok(())
    }
}

