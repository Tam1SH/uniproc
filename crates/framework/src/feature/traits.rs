use crate::app::Window;
use crate::feature::FeatureContextState;
use crate::navigation::{RouteActivated, RouteDeactivated};
use crate::reactor::Reactor;
use crate::uri::AppUri;
use app_core::actor::event_bus::builder::EventSubscription;
use app_core::actor::event_bus::EventBus;
use app_core::actor::Addr;
use app_core::actor::Context;
use app_core::actor::UiThreadToken;
use app_core::lifecycle_tracker::LifecycleTracker;
use app_core::trace::in_named_scope;
use app_core::SharedState;

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

pub trait WindowFeature<TWindow: Window> {
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()>;

    fn uninstall(self: Box<Self>, ui: &TWindow) -> anyhow::Result<()>;
}

pub trait AppFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()>;
}

pub trait FeatureComponent: Sized + 'static {
    fn context_state(&mut self) -> &mut FeatureContextState;
    fn on_activated(&mut self, uri: &AppUri, ctx: &Context<Self>);
    fn on_deactivated(&mut self, previous_uri: &AppUri, ctx: &Context<Self>);
}

pub struct Events<T>(std::marker::PhantomData<T>);

impl<A, T> EventSubscription<A> for Events<T>
where
    A: FeatureComponent + 'static,
    T: EventSubscription<A>,
{
    fn subscribe_into(addr: Addr<A>, tracker: &impl LifecycleTracker) {
        let a_act = addr.clone();
        EventBus::subscribe_fn::<RouteActivated>(
            move |msg| {
                a_act.apply(move |actor, ctx| {
                    in_named_scope("framework.navigation.activate", None, None, || {
                        if let Some(key) = actor.context_state().handle_activation(&msg) {
                            actor.on_activated(key, ctx);
                        }
                    });
                });
            },
            tracker,
        );

        let a_deact = addr.clone();
        EventBus::subscribe_fn::<RouteDeactivated>(
            move |msg| {
                a_deact.apply(move |actor, ctx| {
                    in_named_scope("framework.navigation.deactivate", None, None, || {
                        if actor.context_state().handle_deactivation(&msg) {
                            actor.on_deactivated(&msg.uri, ctx);
                        }
                    });
                });
            },
            tracker,
        );

        T::subscribe_into(addr, tracker);
    }
}
