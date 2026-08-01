use guinea::feature::FeatureInitContext;
use guinea::router::{Page, PageCx};
use guinea::uri::AppUri;
use windows_reactor::Element;

pub struct Processes;

impl Page for Processes {
    const CACHE_STATE_IN_MEMORY: bool = true;

    fn install(ctx: &FeatureInitContext, _uri: &AppUri) -> anyhow::Result<()> {
        // metrics::install lives on ShellLayout now - the sidebar's tiles
        // need it at an ancestor scope of every page, not just this one.
        domain2::features::processes::install(ctx)
    }

    fn view(cx: &mut PageCx) -> Element {
        let settings = domain2::features::processes::settings::ProcessesSettings::new()
            .expect("processes settings must construct");
        let settings = cx.use_ref(settings);
        let map = settings.borrow().columns().configs();
        ui2::pages::processes::processes_view(cx, &map)
    }
}
