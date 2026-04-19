use anyhow::Context as _;
use app_core::actor::event_bus::EventBus;
use app_core::app::UiContext;
use app_core::feature::{
    AppFeature, AppFeatureInitContext, WindowFeature, WindowFeatureInitContext,
};
use app_core::reactor::Reactor;
use app_core::test_kit::Stabilizer;
use app_core::SharedState;
use context::settings::SettingsStore;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, Once};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

slint::slint! {
    export component DomainTestWindow inherits Window {}
}

pub fn new_test_window() -> DomainTestWindow {
    i_slint_backend_testing::init_no_event_loop();
    DomainTestWindow::new().expect("failed to create DomainTestWindow")
}

pub fn pump_ui(ms: u64) {
    i_slint_core::tests::slint_mock_elapsed_time(ms);
    stabilize_ui();
    slint::platform::update_timers_and_animations();
}

pub fn stabilize_ui() {
    let mut iterations = 0;

    loop {
        EventBus::process_queue();

        i_slint_core::tests::slint_mock_elapsed_time(16);
        slint::platform::update_timers_and_animations();

        let bg_tasks = EventBus::task_count();
        let queue_empty = EventBus::is_queue_empty();

        if bg_tasks == 0 && queue_empty {
            thread::sleep(Duration::from_millis(16));
            if EventBus::task_count() == 0 && EventBus::is_queue_empty() {
                break;
            }
        }

        thread::sleep(Duration::from_millis(16));

        iterations += 1;
        if iterations > 500 {
            panic!(
                "UI stabilization timeout! Still have {} active tasks",
                bg_tasks
            );
        }
    }
}

pub struct FeatureHarness {
    pub ui: DomainTestWindow,
    pub reactor: Reactor,
    pub shared: SharedState,
}

impl Stabilizer for FeatureHarness {
    fn stabilize(&mut self) {
        stabilize_ui()
    }
}

impl FeatureHarness {
    pub fn new(settings_path: PathBuf) -> Self {
        init_tracing(settings_path).unwrap();
        Self {
            ui: new_test_window(),
            reactor: Reactor::new(),
            shared: SharedState::new(),
        }
    }

    pub fn step(&mut self, action: impl FnOnce()) {
        action();
        pump_ui(16);
    }

    pub fn app_install<F>(&mut self, feature: F) -> anyhow::Result<()>
    where
        F: AppFeature,
    {
        feature.install(&mut AppFeatureInitContext {
            token: self.ui.new_token(),
            reactor: &mut self.reactor,
            shared: &self.shared,
        })
    }

    pub fn install<F>(&mut self, mut feature: F) -> anyhow::Result<()>
    where
        F: WindowFeature<DomainTestWindow>,
    {
        feature.install(&mut WindowFeatureInitContext {
            window_id: 0,
            reactor: &mut self.reactor,
            ui: &self.ui,
            shared: &self.shared,
        })
    }

    pub fn install_settings_at(&mut self, path: PathBuf) -> anyhow::Result<()> {
        domain::features::settings::SettingsFeature::with_path(path)
            .install(&mut AppFeatureInitContext {
                token: self.ui.new_token(),
                reactor: &mut self.reactor,
                shared: &self.shared,
            })
            .context("failed to install SettingsFeature")
    }
}

pub fn init_tracing(settings_path: PathBuf) -> anyhow::Result<()> {
    let logs_dir = settings_path
        .parent()
        .map(|parent| parent.join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    std::fs::create_dir_all(&logs_dir)?;

    let writer = BoxMakeWriter::new(std::io::stderr);

    //May already be init'd.
    let _ = context::trace::init_test_subscriber(writer, TEST_CAPTURED_LOGS.clone());

    Ok(())
}

pub fn temp_settings_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("uniproc-domain-test-{nanos}.json"))
}

static TEST_CAPTURED_LOGS: LazyLock<Arc<Mutex<Vec<String>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

pub struct TestTrace;

impl TestTrace {
    pub fn clear() {
        if let Ok(mut logs) = TEST_CAPTURED_LOGS.lock() {
            logs.clear();
        }
    }

    pub fn contains(substring: &str) -> bool {
        if let Ok(logs) = TEST_CAPTURED_LOGS.lock() {
            return logs.iter().any(|l| l.contains(substring));
        }
        false
    }

    pub fn all() -> Vec<String> {
        TEST_CAPTURED_LOGS
            .lock()
            .map(|l| l.clone())
            .unwrap_or_default()
    }
}
