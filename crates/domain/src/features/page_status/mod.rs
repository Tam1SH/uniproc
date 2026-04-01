use app_core::app::{Feature, Window};
use app_core::reactor::Reactor;
use app_core::SharedState;
use context::page_status::PageStatusRegistry;
use std::sync::Arc;

pub struct PageStatusFeature;

impl<TWindow> Feature<TWindow> for PageStatusFeature
where
    TWindow: Window,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        _ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let registry = Arc::new(PageStatusRegistry::new());
        shared.insert_arc(registry);
        Ok(())
    }
}
