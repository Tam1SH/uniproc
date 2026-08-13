use guinea::feature::FeatureInitContext;
use guinea::router::{Page, PageCx};
use guinea::uri::AppUri;
use windows_reactor::Element;

pub struct Wsl;

impl Page for Wsl {
    const CACHE_STATE_IN_MEMORY: bool = true;

    fn install(ctx: &FeatureInitContext, _uri: &AppUri) -> anyhow::Result<()> {
        domain::features::wsl::install(ctx)
    }

    fn view(cx: &mut PageCx) -> Element {
        ui::pages::wsl::wsl_view(cx)
    }
}
