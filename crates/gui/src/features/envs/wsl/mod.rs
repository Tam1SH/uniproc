use crate::core::actor::addr::Addr;
use crate::core::reactor::Reactor;
use crate::features::envs::wsl::actors::{Init, InstallAgent, WslActor};
use crate::features::Feature;
use crate::AppWindow;
use slint::ComponentHandle;

mod actors;
mod domain;

pub struct WslFeature;

#[derive(Clone, Debug)]
struct RawDistroData {
    name: String,
    is_installed: bool,
    is_running: bool,
}

impl Feature for WslFeature {
    fn install(self, _: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        let addr = Addr::new(WslActor, ui.as_weak());

        addr.send(Init);

        ui.on_install_agent(addr.handler_with(InstallAgent));

        Ok(())
    }
}
