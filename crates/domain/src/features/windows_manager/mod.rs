use crate::features::windows_manager::actor::WindowManagerActor;
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::app::Feature;
use app_core::app::Window;
use app_core::reactor::Reactor;
use app_core::SharedState;
use context::native_windows::slint_factory::{
    OpenWindow, SlintWindowRegistry, WindowClosed, WindowRegistry,
};

mod actor;

pub struct WindowManagerFeature;

impl<TWindow> Feature<TWindow> for WindowManagerFeature
where
    TWindow: Window,
{
    fn install(self, _: &mut Reactor, ui: &TWindow, shared: &SharedState) -> anyhow::Result<()> {
        let reg = SlintWindowRegistry::new();
        shared.insert(reg);
        let reg = shared.get::<SlintWindowRegistry>().unwrap();

        let actor = WindowManagerActor::new(reg);
        let addr = Addr::new(actor, ui.as_weak());

        let guard = ui.new_token();
        EventBus::subscribe::<_, WindowClosed, _>(&guard, addr.clone());
        EventBus::subscribe::<_, OpenWindow, _>(&guard, addr);

        Ok(())
    }
}
