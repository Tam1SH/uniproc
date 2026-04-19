use app_core::actor::registry::ActorRegistry;
use app_core::feature::{AppFeature, AppFeatureInitContext};

#[derive(Clone)]
pub struct TestDiscoveryFeature;

impl AppFeature for TestDiscoveryFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        ctx.shared.insert(ActorRegistry::default());
        Ok(())
    }
}
