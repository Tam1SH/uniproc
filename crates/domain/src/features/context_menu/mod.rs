use app_core::app::Window;
mod actors;
mod settings;

use crate::features::context_menu::settings::ContextMenuSettings;
use crate::features::cosmetics::accent_from;
use actors::{ContextMenuActor, HandleAction, Hide, Show};
use app_contracts::features::context_menu::{ContextMenuUiBindings, ContextMenuUiPort};
use app_core::SharedState;
use app_core::actor::addr::Addr;
use app_core::app::Feature;
use app_core::reactor::Reactor;

pub struct ContextMenuFeature<F> {
    make_port: F,
}

impl<F> ContextMenuFeature<F> {
    pub fn new(make_port: F) -> Self {
        Self { make_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for ContextMenuFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> anyhow::Result<P> + 'static,
    P: ContextMenuUiPort + ContextMenuUiBindings + Clone,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = ContextMenuSettings::new(shared)?;
        let reveal_delay_ms = settings.reveal_delay_ms();

        let port = (self.make_port)(ui)?;
        if let Some(accent) = accent_from(shared) {
            port.set_menu_accent(accent);
        }

        let addr = Addr::new(
            ContextMenuActor {
                reveal_delay_ms,
                port: port.clone(),
            },
            ui.as_weak(),
        );

        let a = addr.clone();
        port.on_show_context_menu(move |x, y| a.send(Show { x, y }));
        port.on_close_menu(addr.handler(Hide));
        let a = addr.clone();
        port.on_action(move |action| a.send(HandleAction(action)));

        Ok(())
    }
}
