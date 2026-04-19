use crate::features::windows_manager::actor::WindowManagerActor;
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::feature::{AppFeature, AppFeatureInitContext};
use app_core::lifecycle_tracker::FeatureLifecycle;
use context::native_windows::slint_factory::{OpenWindow, SlintWindowRegistry, WindowClosed};

mod actor;

pub struct WindowManagerFeature;

impl AppFeature for WindowManagerFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let reg = SlintWindowRegistry::new();

        ctx.shared.insert(reg);
        let reg = ctx.shared.get::<SlintWindowRegistry>().unwrap();

        let actor = WindowManagerActor::new(reg);
        let addr = Addr::new(actor, ctx.token.clone(), &FeatureLifecycle::new());

        EventBus::subscribe::<_, WindowClosed>(addr.clone(), &FeatureLifecycle::new());
        EventBus::subscribe::<_, OpenWindow>(addr, &FeatureLifecycle::new());

        Ok(())
    }
}
