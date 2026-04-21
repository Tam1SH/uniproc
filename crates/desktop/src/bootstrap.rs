use app_core::app::App;
use context::settings::SettingsStore;
use domain::features::cosmetics::CosmeticsFeature;
use domain::features::l10n::L10nFeature;
use domain::features::navigation::NavigationFeature;
use domain::features::page_status::PageStatusFeature;
use domain::features::services::ServicesFeature;
use domain::features::settings::SettingsFeature;
use domain::features::sidebar::SidebarFeature;
use domain::features::trace_settings::TraceSettingsFeature;
use domain::features::window_actions::WindowActionsFeature;
use domain::features::windows_manager::WindowManagerFeature;
use domain_agents::features::agents::AgentsFeature;
use domain_environments::features::environments::EnvironmentsFeature;
use domain_processes::processes_impl::ProcessFeature;
use slint::ComponentHandle;
use slint_adapter::AppWindow;
use slint_adapter::features::cosmetics::UiCosmeticsAdapter;
use slint_adapter::features::environments::UiEnvironmentsAdapter;
use slint_adapter::features::l10n::SlintL10nPort;
use slint_adapter::features::navigation::UiNavigationAdapter;
use slint_adapter::features::processes::UiProcessesAdapter;
use slint_adapter::features::services::UiServicesAdapter;
use slint_adapter::features::sidebar::UiSidebarAdapter;
use slint_adapter::features::window_actions::UiWindowActionsAdapter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::{BoxMakeWriter, MakeWriterExt};

macro_rules! with_adapter {
    ($feature:ident => $adapter:ident) => {
        || $feature::new(|ui: &AppWindow| $adapter::new(ui.as_weak()))
    };
}

pub fn run() -> anyhow::Result<()> {
    let _tracing = init_tracing()?;

    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();
    let ui = AppWindow::new()?;

    let app = App::new(ui)
        .app_feature(SettingsFeature::default())?
        .app_feature(TraceSettingsFeature)?
        .app_feature(AgentsFeature)?
        .app_feature(PageStatusFeature)?
        .app_feature(WindowManagerFeature)?
        .feature(with_adapter!(CosmeticsFeature => UiCosmeticsAdapter))
        .feature(with_adapter!(WindowActionsFeature => UiWindowActionsAdapter))
        .feature(with_adapter!(EnvironmentsFeature => UiEnvironmentsAdapter))
        .feature(with_adapter!(NavigationFeature => UiNavigationAdapter))
        .feature(with_adapter!(SidebarFeature => UiSidebarAdapter))
        .feature(with_adapter!(L10nFeature => SlintL10nPort))
        .feature(with_adapter!(ServicesFeature => UiServicesAdapter))
        .feature(with_adapter!(ProcessFeature => UiProcessesAdapter));
    app.run()
}

struct TracingRuntime {
    _guard: WorkerGuard,
}

fn init_tracing() -> anyhow::Result<TracingRuntime> {
    let settings_path = SettingsStore::default_settings_path()?;
    let logs_dir = settings_path
        .parent()
        .map(|parent| parent.join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));

    std::fs::create_dir_all(&logs_dir)?;

    let file_appender = tracing_appender::rolling::daily(logs_dir, "desktop.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    #[cfg(debug_assertions)]
    let writer = BoxMakeWriter::new(writer.and(std::io::stderr));
    #[cfg(not(debug_assertions))]
    let writer = BoxMakeWriter::new(writer);

    context::trace::init_subscriber(&settings_path, writer)?;

    Ok(TracingRuntime { _guard: guard })
}
