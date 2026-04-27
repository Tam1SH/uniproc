use crate::utils::DomainTestWindow;
use app_core::actor::addr::Addr;
use app_core::actor::{Context, ManagedActor};
use framework::app::UiContext;
use framework::feature::{
    Events, FeatureComponent, FeatureContextState, WindowFeature, WindowFeatureInitContext,
};
use framework::lifecycle_tracker::FeatureLifecycle;
use framework::uri::AppUri;
use macros::actor_manifest;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct TestFeatureState {
    pub is_active: Arc<AtomicBool>,
}

#[actor_manifest]
impl ManagedActor for MockFeatureActor {
    type Bus = Events<bus!()>;
    type Handlers = handlers!();
}

pub struct MockFeatureActor {
    pub state: TestFeatureState,
    pub ctx_state: FeatureContextState,
}

impl FeatureComponent for MockFeatureActor {
    fn context_state(&mut self) -> &mut FeatureContextState {
        &mut self.ctx_state
    }

    fn on_activated(&mut self, _uri: &AppUri, _ctx: &Context<Self>) {
        self.state.is_active.store(true, Ordering::SeqCst);
    }

    fn on_deactivated(&mut self, _uri: &AppUri, _ctx: &Context<Self>) {
        self.state.is_active.store(false, Ordering::SeqCst);
    }
}

pub struct MockWindowFeature {
    capability: &'static str,
    state: TestFeatureState,
    tracker: FeatureLifecycle,
}

impl MockWindowFeature {
    pub fn new(capability: &'static str, state: TestFeatureState) -> Self {
        Self {
            capability,
            state,
            tracker: FeatureLifecycle::new(),
        }
    }
}

impl WindowFeature<DomainTestWindow> for MockWindowFeature {
    fn install(
        &mut self,
        ctx: &mut WindowFeatureInitContext<DomainTestWindow>,
    ) -> anyhow::Result<()> {
        let actor = MockFeatureActor {
            state: self.state.clone(),
            ctx_state: FeatureContextState::new(ctx.window_id, self.capability),
        };

        let addr = Addr::new_managed(actor, ctx.ui.new_token(), &self.tracker);

        Ok(())
    }

    fn uninstall(self: Box<Self>, ui: &DomainTestWindow) -> anyhow::Result<()> {
        self.tracker.shutdown(&ui.new_token());
        Ok(())
    }
}
