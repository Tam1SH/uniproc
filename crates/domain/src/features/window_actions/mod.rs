mod actors;

use actors::{Close, Drag, Maximize, Minimize, Resize, WindowActor};
use app_contracts::features::window_actions::WindowActionsPort;
use app_core::SharedState;
use app_core::actor::addr::Addr;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;

pub struct WindowActionsFeature<F> {
    make_port: F,
}

impl<F> WindowActionsFeature<F> {
    pub fn new(make_port: F) -> Self {
        Self { make_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for WindowActionsFeature<F>
where
    TWindow: ComponentHandle + 'static,
    F: Fn(&TWindow) -> P + 'static,
    P: WindowActionsPort,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        _shared: &SharedState,
    ) -> anyhow::Result<()> {
        let port = (self.make_port)(ui);
        let addr = Addr::new(WindowActor { port: port.clone() }, ui.as_weak());

        port.on_drag(addr.handler(Drag));
        port.on_close(addr.handler(Close));
        port.on_minimize(addr.handler(Minimize));
        port.on_maximize(addr.handler(Maximize));
        port.on_start_resize(move |edge| addr.send(Resize(edge)));

        Ok(())
    }
}
