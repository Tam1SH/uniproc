use anyhow::Context as _;
use app_core::actor::event_bus::EventBus;
use app_core::actor::UiDispatcher;
use app_core::test_kit::Stabilizer;
use app_core::SharedState;
use framework::app::{App, UiContext, Window};
use framework::feature::{
    AppFeature, AppFeatureInitContext, WindowFeature, WindowFeatureInitContext,
};
use framework::reactor::Reactor;
use framework::settings::SettingsStore;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, Once};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

slint::slint! {
    export component DomainTestWindow inherits Window {}
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

pub struct FeatureHarness(pub Option<App<DomainTestWindow>>);

impl FeatureHarness {
    pub fn new(settings_path: PathBuf) -> Self {
        //do not change the sequence
        i_slint_backend_testing::init_no_event_loop();
        let ui = DomainTestWindow::new().expect("failed to create window");

        let app = App::with_dispatcher(ui, TestUiDispatcher { settings_path });
        Self(Some(app))
    }

    pub fn app_feature<F: AppFeature>(mut self, feature: F) -> anyhow::Result<Self> {
        let app = self.0.take().expect("App is missing");
        self.0 = Some(app.app_feature(feature)?);

        Ok(self)
    }

    pub fn window_feature<F, Builder>(mut self, builder: Builder) -> Self
    where
        Builder: Fn() -> F + 'static,
        F: WindowFeature<DomainTestWindow> + 'static,
    {
        let app = self.0.take().expect("App is missing");
        self.0 = Some(app.window_feature(builder));
        self
    }

    pub fn shared(&self) -> &SharedState {
        &self.0.as_ref().unwrap().shared()
    }
}

impl Drop for FeatureHarness {
    fn drop(&mut self) {
        if let Some(app) = self.0.take() {
            app.ui().window().hide();
            stabilize_ui();
        }
    }
}

impl Stabilizer for FeatureHarness {
    fn stabilize(&mut self) {
        stabilize_ui()
    }
}

impl Deref for FeatureHarness {
    type Target = App<DomainTestWindow>;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("App was moved out")
    }
}

impl std::ops::DerefMut for FeatureHarness {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("App was moved out")
    }
}

pub struct TestUiDispatcher {
    pub settings_path: std::path::PathBuf,
}

static TEST_ENV_INIT: Once = Once::new();

impl UiDispatcher for TestUiDispatcher {
    fn init(&self) {
        init_tracing(self.settings_path.clone()).unwrap();

        app_core::actor::set_ui_dispatcher(self.clone());
    }

    fn dispatch(&self, task: app_core::actor::UiTask) {
        let _ = slint::invoke_from_event_loop(task);
    }
}

impl Clone for TestUiDispatcher {
    fn clone(&self) -> Self {
        Self {
            settings_path: self.settings_path.clone(),
        }
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
