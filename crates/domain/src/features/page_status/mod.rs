use context::page_status::RouteStatusRegistry;
use framework::feature::{AppFeature, AppFeatureInitContext};
use std::sync::Arc;

pub struct PageStatusFeature;

impl AppFeature for PageStatusFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let registry = Arc::new(RouteStatusRegistry::new());
        ctx.shared.insert_arc(registry);
        Ok(())
    }
}
