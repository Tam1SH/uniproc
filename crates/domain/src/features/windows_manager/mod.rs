use crate::features::windows_manager::actor::WindowManagerActor;
use app_core::actor::addr::Addr;
use framework::feature::{AppFeature, AppFeatureInitContext};
use framework::lifecycle_tracker::FeatureLifecycle;
use framework::native_windows::slint_factory::SlintWindowRegistry;

mod actor;

pub struct WindowManagerFeature;

impl AppFeature for WindowManagerFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let reg = SlintWindowRegistry::new();

        ctx.shared.insert(reg);
        let reg = ctx.shared.get::<SlintWindowRegistry>().unwrap();

        let actor = WindowManagerActor::new(reg);
        let _ = Addr::new_managed(actor, ctx.token.clone(), &FeatureLifecycle::new());

        Ok(())
    }
}
