mod actors;

use super::Feature;
use crate::core::actor::addr::Addr;
use crate::core::reactor::Reactor;
use crate::features::window_actions::actors::*;
use crate::{AppWindow, TitleBarActions};
use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;

pub struct WindowActionsFeature;

impl Feature for WindowActionsFeature {
    fn install(self, _: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        let addr = Addr::new(WindowActor, ui.as_weak());

        let actions = ui.global::<TitleBarActions>();

        actions.on_drag(addr.handler(Drag));
        actions.on_close(addr.handler(Close));
        actions.on_minimize(addr.handler(Minimize));
        actions.on_maximize(addr.handler(Maximize));

        ui.on_start_resize(addr.handler_with(Resize));

        Ok(())
    }
}
