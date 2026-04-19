use crate::actor::UiThreadToken;
use crate::feature::{AppFeature, AppFeatureInitContext, WindowFeature, WindowFeatureInitContext};
use crate::reactor::Reactor;
use crate::trace::in_named_scope;
use crate::SharedState;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub trait UiContext {
    fn new_token(&self) -> UiThreadToken;
}

impl<TWindow: ComponentHandle + 'static> UiContext for TWindow {
    fn new_token(&self) -> UiThreadToken {
        UiThreadToken::new()
    }
}

pub trait Window: ComponentHandle + UiContext + 'static {}
impl<TWindow: ComponentHandle + UiContext + 'static> Window for TWindow {}

pub struct App<TWindow> {
    ui: TWindow,
    reactor: Reactor,
    shared: SharedState,
    window_factories: Vec<Box<dyn Fn() -> Box<dyn WindowFeature<TWindow>> + 'static>>,
    next_window_id: AtomicUsize,
}

impl<TWindow: Window> App<TWindow> {
    pub fn new(ui: TWindow) -> Self {
        Self {
            ui,
            reactor: Reactor::new(),
            shared: SharedState::new(),
            window_factories: Vec::new(),
            next_window_id: AtomicUsize::new(1),
        }
    }

    pub fn app_feature<F: AppFeature>(mut self, feature: F) -> anyhow::Result<Self> {
        let full_name = std::any::type_name::<F>();

        let clean_name = full_name
            .split('<')
            .next()
            .unwrap_or(full_name)
            .split("::")
            .last()
            .unwrap_or("Unknown");

        in_named_scope(
            "core.app.feature_install",
            Some("feature,status,level"),
            Some(format!("{}|ok|app", clean_name)),
            || match feature.install(&mut AppFeatureInitContext {
                token: self.ui.new_token(),
                reactor: &mut self.reactor,
                shared: &self.shared,
            }) {
                Ok(_) => {
                    tracing::info!(
                        feature = clean_name,
                        status = "ok",
                        level = "app",
                        "feature.install"
                    );
                    Ok(self)
                }
                Err(e) => {
                    tracing::error!(feature = clean_name, status = "error", level = "app", error = %e, "feature.install");
                    Err(e)
                }
            },
        )
    }

    pub fn feature<F, Builder>(mut self, builder: Builder) -> Self
    where
        Builder: Fn() -> F + 'static,
        F: WindowFeature<TWindow> + 'static,
    {
        self.window_factories
            .push(Box::new(move || Box::new(builder())));
        self
    }

    pub fn spawn_window(&mut self, ui: TWindow) -> anyhow::Result<()> {
        let window_id = self.next_window_id.fetch_add(1, Ordering::Relaxed);
        let mut active_features = Vec::new();

        for factory in &self.window_factories {
            let mut feature = factory();

            feature.install(&mut WindowFeatureInitContext {
                window_id,
                ui: &self.ui,
                shared: &self.shared,
                reactor: &mut self.reactor,
            })?;
            active_features.push(feature);
        }

        let features_storage = Rc::new(RefCell::new(active_features));
        let ui_clone = ui.clone_strong();

        ui.window().on_close_requested(move || {
            let features = std::mem::take(&mut *features_storage.borrow_mut());

            for feature in features {
                if let Err(e) = feature.uninstall(&ui_clone) {
                    tracing::error!("Error uninstalling feature: {}", e);
                }
            }

            slint::CloseRequestResponse::HideWindow
        });

        Ok(())
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        self.spawn_window(self.ui.clone_strong())?;
        self.ui.run()?;
        Ok(())
    }
}
