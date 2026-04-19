use app_core::feature::{AppFeature, AppFeatureInitContext};
use context::page_status::PageStatusRegistry;
use std::sync::Arc;

pub struct PageStatusFeature;

impl AppFeature for PageStatusFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        let registry = Arc::new(PageStatusRegistry::new());
        ctx.shared.insert_arc(registry);
        Ok(())
    }
}
