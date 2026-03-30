use app_core::app::Window;
mod actors;

use crate::features::cosmetics::accent_from;
use actors::{Drag, Execute, RunTaskActor, Show};
use app_contracts::features::run_task::{RunTaskPort, RunTaskRequest};
use app_core::SharedState;
use app_core::actor::addr::Addr;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;

pub struct RunTaskFeature<F> {
    make_port: F,
}

impl<F> RunTaskFeature<F> {
    pub fn new(make_port: F) -> Self {
        Self { make_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for RunTaskFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> anyhow::Result<P> + 'static,
    P: RunTaskPort,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let port = (self.make_port)(ui)?;
        let addr = Addr::new(RunTaskActor { port: port.clone() }, ui.as_weak());

        port.on_open(addr.handler(Show));
        port.on_drag(addr.handler(Drag));
        port.on_run_task(move |req: RunTaskRequest| addr.send(Execute(req)));
        port.apply_dialog_effects();

        if let Some(accent) = accent_from(shared) {
            port.set_dialog_accent(accent);
        }

        Ok(())
    }
}
