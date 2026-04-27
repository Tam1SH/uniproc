use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::SubscriptionId;
use crate::actor::UiThreadToken;

pub trait LifecycleTracker {
    fn shutdown(self, token: &UiThreadToken);
    fn track_loop<T: 'static>(&self, handle: T);
    fn track_actor<A: 'static>(&self, addr: &Addr<A>);
    fn track_sub(&self, id: SubscriptionId);
}
