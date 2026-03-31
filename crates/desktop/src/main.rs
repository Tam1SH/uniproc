#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_core::app::App;
use domain::features::context_menu::ContextMenuFeature;
use domain::features::cosmetics::CosmeticsFeature;
use domain::features::l10n::L10nFeature;
use domain::features::navigation::NavigationFeature;
use domain::features::run_task::RunTaskFeature;
use domain::features::settings::SettingsFeature;
use domain::features::window_actions::WindowActionsFeature;
use domain_agents::features::agents::AgentsFeature;
use domain_environments::features::environments::EnvironmentsFeature;
use domain_processes::processes_impl::ProcessFeature;
use slint::ComponentHandle;
use slint_adapter::AppWindow;
use slint_adapter::adapters::context_menu::ContextMenuUiAdapter;
use slint_adapter::adapters::cosmetics::CosmeticsAdapter;
use slint_adapter::adapters::environments::EnvironmentsUiAdapter;
use slint_adapter::adapters::l10n::SlintL10nPort;
use slint_adapter::adapters::navigation::NavigationUiAdapter;
use slint_adapter::adapters::processes::ProcessesUiAdapter;
use slint_adapter::adapters::run_task::RunTaskAdapter;
use slint_adapter::adapters::window_actions::WindowActionsAdapter;
use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

macro_rules! with_adapter {
    ($feature:ident => $adapter:ident) => {
        $feature::new(|ui: &AppWindow| $adapter::new(ui.as_weak()))
    };
}

fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    let _profiler = dhat::Profiler::new_heap();

    let targets = Targets::new()
        .with_default(Level::DEBUG)
        .with_target("ogurpchik", Level::WARN)
        .with_target("app_core::settings::store", Level::WARN);

    tracing_subscriber::registry()
        .with(targets)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();
    let ui = AppWindow::new()?;

    let app = App::new(ui)
        .feature(SettingsFeature::default())?
        .feature(AgentsFeature)?
        .feature(with_adapter!(CosmeticsFeature => CosmeticsAdapter))?
        .feature(with_adapter!(WindowActionsFeature => WindowActionsAdapter))?
        .feature(with_adapter!(RunTaskFeature => RunTaskAdapter))?
        .feature(with_adapter!(ContextMenuFeature => ContextMenuUiAdapter))?
        .feature(with_adapter!(EnvironmentsFeature => EnvironmentsUiAdapter))?
        .feature(with_adapter!(NavigationFeature => NavigationUiAdapter))?
        .feature(with_adapter!(L10nFeature => SlintL10nPort))?
        .feature(with_adapter!(ProcessFeature => ProcessesUiAdapter))?;

    app.run()
}
