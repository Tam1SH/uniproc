use app_core::app::Window;
mod actors;
pub mod domain;

pub use actors::{Init, InstallAgent, WslEnvActor};

use app_contracts::features::environments::{
    EnvironmentsUiBindings, EnvironmentsUiPort, WslAgentRuntimeEvent,
};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use app_core::SharedState;

pub struct WslFeature<F> {
    make_ui_port: F,
}

impl<F> WslFeature<F> {
    pub fn new(make_ui_port: F) -> Self {
        Self { make_ui_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for WslFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: EnvironmentsUiPort + EnvironmentsUiBindings + Clone + 'static,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        _shared: &SharedState,
    ) -> anyhow::Result<()> {
        let ui_port = (self.make_ui_port)(ui);
        let addr = Addr::new(WslEnvActor::new(ui_port.clone()), ui.as_weak());

        let a = addr.clone();
        ui_port.on_install_agent(move |distro| a.send(InstallAgent(distro)));

        EventBus::subscribe::<WslEnvActor<P>, WslAgentRuntimeEvent, TWindow>(
            &ui.new_token(),
            addr.clone(),
        );
        addr.send(Init);
        Ok(())
    }
}
