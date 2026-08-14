mod layouts;
mod pages;
mod route_memory;
mod routes;
mod tracing_init;

mod meta {
    guinea_meta::manifest!();
}

use route_memory::RememberedRoute;
use windows_reactor::{App, Backdrop};

fn main() -> anyhow::Result<()> {
    tracing_init::init()?;

    let runtime = tokio::runtime::Runtime::new()?;
    let _guard = runtime.enter();

    guinea::app::App::new()
        .plugin(guinea_plugin_store::StorePlugin::for_app(
            meta::APP_NAME,
            "settings",
        ))
        .plugin(guinea_plugin_l10n::L10nPlugin::<app_contracts::l10n::L10n>::new("en"))
        .feature(domain::features::agents::AgentsFeature)
        .on_route_change(|_, to| route_memory::remember(to))
        .run(
            App::new()
                .title(meta::WINDOW_TITLE)
                .inner_size(1000.0, 700.0)
                .backdrop(Backdrop::Mica),
            RememberedRoute,
        )
}
