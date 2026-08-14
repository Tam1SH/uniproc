use guinea::feature::FeatureInitContext;
use guinea::router::{Page, PageCx};
use guinea::uri::AppUri;
use windows_reactor::{text_block, Element, ElementExt};

pub struct Processes;

impl Page for Processes {
    const CACHE_STATE_IN_MEMORY: bool = true;

    fn install(ctx: &FeatureInitContext, _uri: &AppUri) -> anyhow::Result<()> {
        domain::features::processes::install(ctx)
    }

    // TODO: the view opens the store itself and hands three raw `ReactiveMap`s
    // down into `ui`, so the UI layer knows about persistence and the page holds
    // a `RefCell` of settings across renders. `amethystate-reactor` replaces the
    // whole thing: `cx.use_ame_state::<ProcessesSettings>()` for the slice and
    // `use_ame_entry` per key, with subscriptions bound and dropped by the hook.
    // Blocked on the amethystate 0.10 bump, which is blocked on guinea's pin -
    // guinea v0.1.1 holds amethystate 0.9.4 and `guinea_core::signal::Signal`
    // *is* amethystate-core's `Signal`, so the two versions cannot coexist.
    // The hand-rolled two-way sync in `ui/pages/processes/components/column_layout.rs`
    // goes away at the same time.
    fn view(cx: &mut PageCx) -> Element {
        let settings = cx.use_memo((), || {
            domain::features::processes::settings::ProcessesSettings::new()
                .inspect_err(|err| tracing::error!(?err, "processes settings did not open"))
                .ok()
        });
        let Some(settings) = settings else {
            return text_block("Settings are unavailable").into();
        };
        let map = settings.columns().configs();
        let grouping = settings.grouping();
        ui::pages::processes::processes_view(
            cx,
            &map,
            &grouping.expanded_groups(),
            &grouping.collapsed_sections(),
        )
    }
}
