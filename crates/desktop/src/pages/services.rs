use guinea::feature::FeatureInitContext;
use guinea::router::{Page, PageCx};
use guinea::uri::AppUri;
use windows_reactor::Element;

pub struct Services;

impl Page for Services {
    const CACHE_STATE_IN_MEMORY: bool = true;

    fn install(ctx: &FeatureInitContext, _uri: &AppUri) -> anyhow::Result<()> {
        domain::features::services::install(ctx)
    }

    fn view(cx: &mut PageCx) -> Element {
        ui::pages::services::services_view(cx)
    }
}
