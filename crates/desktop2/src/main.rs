mod app_features;
mod layouts;
mod pages;
mod routes;
mod tracing_init;

mod meta {
    guinea_meta::manifest!();
}

use app_features::AppFeatures;
use guinea::router::RouterRx;
use guinea_core::actor::UiThreadToken;
use guinea_core::{set_ui_dispatcher, UiDispatcher, UiTask};
use routes::Route;
use windows_reactor::{border, App, Backdrop, Element, ElementExt, Interface, RenderCx, UiMarshaller, WinUIDispatcher};

struct ReactorDispatcher(UiMarshaller);

impl UiDispatcher for ReactorDispatcher {
    fn init(&self) {}

    fn dispatch(&self, task: UiTask) {
        self.0.dispatch(task);
    }
}

fn ensure_app_features() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let dispatcher = WinUIDispatcher::for_current_thread().expect(
            "root() must run on the UI thread, which should already have a DispatcherQueue",
        );
        set_ui_dispatcher(ReactorDispatcher(dispatcher.marshaller()));

        let token = UiThreadToken::dangerously_create_token_unchecked();
        let app_features = AppFeatures::new(amethystate::global_store());
        app_features
            .install(
                token,
                vec![Box::new(domain2::features::agents::AgentsFeature)],
            )
            .expect("install app features");
        // Must outlive every actor/loop it spawned - leaked deliberately, not
        // a bug: it lives for the process, same as the app itself.
        Box::leak(Box::new(app_features));
    });
}

fn root(cx: &mut RenderCx) -> Element {
    ensure_app_features();
    // Tables render through a native ListView, whose row height comes from the
    // `ListViewItemMinHeight` theme resource; the default leaves rows far
    // taller than a dense process/service list wants.
    //
    // Declared on the tree rather than poked into
    // `Application::Current().Resources()`: WinUI resolves a resource by
    // walking up, so a subtree entry reaches every ListView under it just the
    // same, without needing the reactor to expose `Application` at all and
    // without mutating global state behind the framework's back.
    //
    // The `border` is load-bearing, not decoration: `Element::Component` has no
    // modifier slot at all (`modifiers_mut` returns `None` for it), and every
    // `ElementExt` setter silently returns the element untouched in that case.
    // Hanging this straight off `RouterRx::render` compiled fine and did
    // exactly nothing.
    border(RouterRx::render(cx, Route::Processes {}, amethystate::global_store()))
        .resources([("ListViewItemMinHeight", 30.0)])
        .into()
}

fn main() -> anyhow::Result<()> {
    tracing_init::init()?;

    let runtime = tokio::runtime::Runtime::new()?;
    let _guard = runtime.enter();

    amethystate::init_global(
        amethystate::StoreBuilder::for_app(meta::APP_NAME, "settings")
            .expect("resolve app config dir"),
    );

    guinea_core::l10n::L10n::<app_contracts2::l10n::L10n>::load(app_contracts2::l10n::L10n::new(
        unic_langid::langid!("en"),
    ));

    App::new()
        .title(meta::WINDOW_TITLE)
        .inner_size(1000.0, 700.0)
        .backdrop(Backdrop::Mica)
        .render(root)
        .map_err(|e| anyhow::anyhow!("windows-reactor app failed: {e:?}"))
}
