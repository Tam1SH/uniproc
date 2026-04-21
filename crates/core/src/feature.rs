use crate::SharedState;
use crate::actor::UiThreadToken;
use crate::actor::traits::{Context, Handler, Message};
use crate::app::Window;
use crate::reactor::Reactor;

pub struct WindowFeatureInitContext<'a, TWindow: Window> {
    pub window_id: usize,
    pub ui: &'a TWindow,
    pub shared: &'a SharedState,
    pub reactor: &'a mut Reactor,
}

pub struct AppFeatureInitContext<'a> {
    pub token: UiThreadToken,
    pub reactor: &'a mut Reactor,
    pub shared: &'a SharedState,
}

#[derive(Clone, Debug)]
pub struct WindowContextActivated {
    pub window_id: usize,
    pub context_key: String,
    pub capability_id: String,
}

#[derive(Clone, Debug)]
pub struct WindowContextDeactivated {
    pub window_id: usize,
}

impl Message for WindowContextDeactivated {}
impl Message for WindowContextActivated {}

pub trait WindowFeature<TWindow: Window> {
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()>;

    fn uninstall(self: Box<Self>, ui: &TWindow) -> anyhow::Result<()>;
}

pub trait AppFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()>;
}

pub trait ContextLifecycleListener: Sized + 'static {
    fn context_state(&mut self) -> &mut FeatureContextState;
    fn on_activated(&mut self, context_key: &str, ctx: &Context<Self>);
    fn on_deactivated(&mut self, ctx: &Context<Self>);
}

impl<A> Handler<WindowContextActivated> for A
where
    A: ContextLifecycleListener + 'static,
{
    fn handle(&mut self, msg: WindowContextActivated, ctx: &Context<Self>) {
        if let Some(key) = self.context_state().handle_activation(&msg) {
            let key_owned = key.to_string();
            self.on_activated(&key_owned, ctx);
        }
    }
}

impl<A> Handler<WindowContextDeactivated> for A
where
    A: ContextLifecycleListener + 'static,
{
    fn handle(&mut self, msg: WindowContextDeactivated, ctx: &Context<Self>) {
        if self.context_state().handle_deactivation(&msg) {
            self.on_deactivated(ctx);
        }
    }
}

#[derive(Clone, Debug)]
pub struct FeatureContextState {
    window_id: usize,
    required_capability: &'static str,
    pub active_context_key: Option<String>,
}

impl FeatureContextState {
    pub fn new(window_id: usize, required_capability: &'static str) -> Self {
        Self {
            window_id,
            required_capability,
            active_context_key: None,
        }
    }

    pub fn handle_activation<'a>(&mut self, msg: &'a WindowContextActivated) -> Option<&'a str> {
        if msg.window_id == self.window_id && msg.capability_id == self.required_capability {
            self.active_context_key = Some(msg.context_key.clone());
            Some(&msg.context_key)
        } else {
            self.active_context_key = None;
            None
        }
    }

    pub fn handle_deactivation(&mut self, msg: &WindowContextDeactivated) -> bool {
        if msg.window_id == self.window_id && self.active_context_key.is_some() {
            self.active_context_key = None;
            true
        } else {
            false
        }
    }

    pub fn is_active_for(&self, context_key: &str) -> bool {
        self.active_context_key.as_deref() == Some(context_key)
    }
}
