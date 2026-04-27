use framework::app::Window;
mod actor;

use crate::features::window_actions::actor::BreakpointChanged;
use actor::{Close, Drag, Maximize, Minimize, Resize, WindowActor};
use app_contracts::features::window_actions::{
    UiWindowActionsBindings, UiWindowActionsPort, WindowActionsBinder,
};
use app_core::actor::addr::Addr;
use framework::feature::{WindowFeature, WindowFeatureInitContext};
use macros::window_feature;

#[window_feature]
pub struct WindowActionsFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for WindowActionsFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiWindowActionsPort + UiWindowActionsBindings,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let port = (self.make_port)(ctx.ui);
        let token = ctx.ui.new_token();
        let addr = Addr::new_managed(WindowActor { port: port.clone() }, token, &self.tracker);

        WindowActionsBinder::new(&addr, &port)
            .on_drag(Drag)
            .on_close(Close)
            .on_minimize(Minimize)
            .on_maximize(Maximize)
            .on_start_resize(Resize)
            .on_config_changed(BreakpointChanged);

        Ok(())
    }
}
