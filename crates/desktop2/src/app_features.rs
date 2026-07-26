use guinea::feature::{AppFeature, AppFeatureInitContext};
use guinea::lifecycle_tracker::AppLifecycle;
use guinea::reactor::Reactor;
use guinea_core::actor::UiThreadToken;
use guinea_core::SharedState;

pub struct AppFeatures {
    reactor: Reactor,
    shared: SharedState,
    tracker: AppLifecycle,
}

impl AppFeatures {
    pub fn new(store: amethystate::DefaultStore) -> Self {
        let shared = SharedState::new();
        shared.insert(store);
        Self {
            reactor: Reactor::new(),
            shared,
            tracker: AppLifecycle::new(),
        }
    }

    pub fn install(
        &self,
        token: UiThreadToken,
        features: Vec<Box<dyn AppFeature>>,
    ) -> anyhow::Result<()> {
        let mut ctx = AppFeatureInitContext {
            token,
            reactor: &self.reactor,
            shared: &self.shared,
            tracker: &self.tracker,
        };
        for mut feature in features {
            feature.install(&mut ctx)?;
        }
        Ok(())
    }
}
