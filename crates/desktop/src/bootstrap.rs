use app_core::app::App;
use context::settings::SettingsStore;
use domain::features::cosmetics::CosmeticsFeature;
use domain::features::l10n::L10nFeature;
use domain::features::navigation::NavigationFeature;
use domain::features::page_status::PageStatusFeature;
use domain::features::run_task::RunTaskFeature;
use domain::features::services::ServicesFeature;
use domain::features::settings::SettingsFeature;
use domain::features::window_actions::WindowActionsFeature;
use domain_agents::features::agents::AgentsFeature;
use domain_environments::features::environments::EnvironmentsFeature;
use domain_processes::processes_impl::ProcessFeature;
use slint::ComponentHandle;
use slint_adapter::adapters::cosmetics::CosmeticsAdapter;
use slint_adapter::adapters::environments::EnvironmentsUiAdapter;
use slint_adapter::adapters::l10n::SlintL10nPort;
use slint_adapter::adapters::navigation::NavigationUiAdapter;
use slint_adapter::adapters::processes::ProcessesUiAdapter;
use slint_adapter::adapters::run_task::RunTaskAdapter;
use slint_adapter::adapters::services::ServicesUiAdapter;
use slint_adapter::adapters::window_actions::WindowActionsAdapter;
use slint_adapter::AppWindow;
use tracing_appender::non_blocking::WorkerGuard;

macro_rules! with_adapter {
    ($feature:ident => $adapter:ident) => {
        $feature::new(|ui: &AppWindow| $adapter::new(ui.as_weak()))
    };
}

pub fn run() -> anyhow::Result<()> {
    let _tracing = init_tracing()?;

    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();
    let ui = AppWindow::new()?;

    let app = App::new(ui)
        .feature(SettingsFeature::default())?
        .feature(AgentsFeature)?
        .feature(PageStatusFeature)?
        .feature(with_adapter!(CosmeticsFeature => CosmeticsAdapter))?
        .feature(with_adapter!(WindowActionsFeature => WindowActionsAdapter))?
        .feature(with_adapter!(RunTaskFeature => RunTaskAdapter))?
        .feature(with_adapter!(EnvironmentsFeature => EnvironmentsUiAdapter))?
        .feature(with_adapter!(NavigationFeature => NavigationUiAdapter))?
        .feature(with_adapter!(L10nFeature => SlintL10nPort))?
        .feature(with_adapter!(ServicesFeature => ServicesUiAdapter))?
        .feature(with_adapter!(ProcessFeature => ProcessesUiAdapter))?;

    app.run()
}

struct TracingRuntime {
    _guard: WorkerGuard,
}

fn init_tracing() -> anyhow::Result<TracingRuntime> {
    let settings_path = SettingsStore::default_settings_path();
    let logs_dir = settings_path
        .parent()
        .map(|parent| parent.join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));

    std::fs::create_dir_all(&logs_dir)?;

    let file_appender = tracing_appender::rolling::daily(logs_dir, "desktop.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    context::trace::init_subscriber(&settings_path, writer)?;

    Ok(TracingRuntime { _guard: guard })
}
