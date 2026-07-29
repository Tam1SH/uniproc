use app_contracts2::features::sidebar::SidebarReducer;
use guinea::feature::FeatureInitContext;
use guinea::router::{Layout, LayoutCx, UseNavigate, UseRoute};
use guinea::uri::AppUri;
use windows_reactor::{Element, SetState};

use crate::routes::Route;

pub struct ShellLayout;

impl Layout for ShellLayout {
    fn install(ctx: &FeatureInitContext, _uri: &AppUri) -> anyhow::Result<()> {
        domain2::features::sidebar::install(ctx)
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
        )
    }
}
