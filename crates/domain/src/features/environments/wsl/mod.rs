use framework::app::Window;
mod actor;
pub mod domain;

pub use actor::{Init, InstallAgent, WslEnvActor};

use app_contracts::features::environments::{
    EnvironmentsBinder, UiEnvironmentsBindings, UiEnvironmentsPort,
};
use app_core::actor::addr::Addr;
use framework::feature::{WindowFeature, WindowFeatureInitContext};
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
        let addr = Addr::new_managed(WslEnvActor::new(ui_port.clone()), token, &self.tracker);

        EnvironmentsBinder::new(&addr, &ui_port).on_install_agent(InstallAgent);

        addr.send(Init);
        Ok(())
    }
}
