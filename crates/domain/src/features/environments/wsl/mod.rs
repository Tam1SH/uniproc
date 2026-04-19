use app_core::app::Window;
mod actor;
pub mod domain;

pub use actor::{Init, InstallAgent, WslEnvActor};

use app_contracts::features::environments::{
    UiEnvironmentsBindings, UiEnvironmentsPort, WslAgentRuntimeEvent,
};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::feature::{WindowFeature, WindowFeatureInitContext};
use macros::window_feature;

#[window_feature]
pub struct WslFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for WslFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiEnvironmentsPort + UiEnvironmentsBindings + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let ui_port = (self.make_port)(ctx.ui);
        let token = ctx.ui.new_token();
        let addr = Addr::new(WslEnvActor::new(ui_port.clone()), token, &self.tracker);

        let a = addr.clone();
        ui_port.on_install_agent(move |distro| a.send(InstallAgent(distro)));

        EventBus::subscribe::<_, WslAgentRuntimeEvent>(addr.clone(), &self.tracker);
        addr.send(Init);
        Ok(())
    }
}
