use app_core::app::Window;
mod actor;

use crate::features::window_actions::actor::BreakpointChanged;
use actor::{Close, Drag, Maximize, Minimize, Resize, WindowActor};
use app_contracts::features::window_actions::{UiWindowActionsBindings, UiWindowActionsPort};
use app_core::actor::addr::Addr;
use app_core::feature::{WindowFeature, WindowFeatureInitContext};
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
        let addr = Addr::new(WindowActor { port: port.clone() }, token, &self.tracker);
        let a = addr.clone();

        port.on_drag(addr.handler(Drag));
        port.on_close(addr.handler(Close));
        port.on_minimize(addr.handler(Minimize));
        port.on_maximize(addr.handler(Maximize));
        port.on_start_resize(move |edge| addr.send(Resize(edge)));

        port.on_config_changed(move |bp, width| {
            a.send(BreakpointChanged(bp, width));
        });

        Ok(())
    }
}
