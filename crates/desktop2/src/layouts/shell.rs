use app_contracts2::features::sidebar::SidebarReducer;
use guinea::feature::FeatureInitContext;
use guinea::router::{Layout, LayoutCx, UseNavigate, UseRoute};
use guinea::uri::AppUri;
use windows_reactor::{Element, Icon};

use crate::routes::Route;
use app_contracts2::icons;

const NAV_ICON_SIZE: f64 = 20.0;

fn icon_for(key: icons::IconKey) -> Icon {
    let path = icons::path_for(key).expect("icon key must resolve to a path");
    guicons::windows_reactor::icon_from_path(path, NAV_ICON_SIZE, NAV_ICON_SIZE)
}

fn nav_items() -> Vec<(&'static str, &'static str, Icon)> {
    vec![
        ("processes", "Processes", icon_for(icons::keys::APPS_LIST)),
        ("services", "Services", icon_for(icons::keys::PUZZLE)),
        ("settings", "Settings", icon_for(icons::keys::SETTINGS)),
    ]
}

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

        ui2::shell_view(
            state.open,
            selected_tag,
            icon_for(icons::keys::UNIPROC_LOGO),
            nav_items(),
            content,
            move || dispatch.emit_on_toggle(),
            move |tag: String| match tag.as_str() {
                "processes" => nav.to(Route::Processes {}),
                "services" => nav.to(Route::Services {}),
                _ => {}
            },
        )
    }
}
