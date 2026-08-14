use std::sync::OnceLock;

use amethystate::amethystate;
use guinea::router::RouterRx;
use windows_reactor::{Component, Element, RenderCx};

use crate::routes::Route;

#[amethystate(prefix = "shell")]
pub struct RouteSettings {
    #[amestate(default = "/".to_string())]
    last_route: String,
}

/// Built on first use, not in `main`: the store plugin opens the store inside
/// `App::run`, after the hook and the root component have been handed over.
static SETTINGS: OnceLock<Option<RouteSettings>> = OnceLock::new();

fn settings() -> Option<&'static RouteSettings> {
    SETTINGS
        .get_or_init(|| match RouteSettings::new() {
            Ok(settings) => Some(settings),
            Err(err) => {
                tracing::warn!(?err, "could not open the route settings");
                None
            }
        })
        .as_ref()
}

fn route_from_path(path: &str) -> Route {
    match path {
        "/services" => Route::Services {},
        "/wsl" => Route::Wsl {},
        _ => Route::Processes {},
    }
}

pub fn remember(path: &str) {
    let Some(settings) = settings() else {
        return;
    };
    if let Err(err) = settings.last_route().set(path.to_string()) {
        tracing::warn!(?err, path, "could not remember the current route");
    }
}

fn restore() -> Route {
    settings()
        .map(|settings| route_from_path(&settings.last_route().get()))
        .unwrap_or(Route::Processes {})
}

pub struct RememberedRoute;

impl Component for RememberedRoute {
    fn render(&self, _props: &(), cx: &mut RenderCx) -> Element {
        let initial = cx.use_memo((), restore);
        RouterRx::<Route>::render(cx, initial)
    }
}
