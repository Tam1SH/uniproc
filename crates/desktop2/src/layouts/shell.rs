use app_contracts2::features::sidebar::SidebarReducer;
use guinea::feature::FeatureInitContext;
use guinea::router::{Layout, LayoutCx, UseNavigate, UseRoute};
use guinea::uri::AppUri;
use windows_reactor::{Element, SetState};

use crate::routes::Route;

pub struct ShellLayout;

impl Layout for ShellLayout {
    fn install(ctx: &FeatureInitContext, _uri: &AppUri) -> anyhow::Result<()> {
        ctx.install(domain2::features::sidebar::install)?;
        // Installed here (layout scope), not from the Processes page: the
        // sidebar's metric tiles render as part of the shell, an ancestor
        // scope of every page. `ctx.install` (not a bare call) marks this
        // scope as metrics' owner so any descendant page that only wants
        // to *read* it must say so explicitly via `ctx.inherit(...)` -
        // previously a page-scoped install here left a descendant's
        // `use_reducer::<MetricsReducer>()` silently resolving to a
        // disconnected local instance with no actor behind it, orphaning
        // the real one and flatlining its chart.
        ctx.install(domain2::features::metrics::install)
    }

    fn view(cx: &mut LayoutCx) -> Element {
        let (state, dispatch) = cx.use_reducer::<SidebarReducer>();
        let current = cx.use_route::<Route>();
        let nav = cx.use_navigate::<Route>();

        let selected_tag = match current {
            Route::Processes { .. } => "processes",
            Route::Services { .. } => "services",
        };

        let content = cx.outlet();
        let resize_dispatch = dispatch.clone();
        let set_width = SetState::new(move |w: f64| {
            resize_dispatch.emit_on_set_width(w.round() as u64);
        });
        let open_dispatch = dispatch.clone();
        let set_open = SetState::new(move |open: bool| {
            open_dispatch.emit_on_set_open(open);
        });

        ui2::shell_view(
            cx,
            state.open,
            selected_tag,
            state.width as f64,
            content,
            move |tag: String| match tag.as_str() {
                "processes" => nav.to(Route::Processes {}),
                "services" => nav.to(Route::Services {}),
                _ => {}
            },
            set_width,
            set_open,
        )
    }
}
