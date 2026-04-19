use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::Event;
use crate::actor::event_bus::EventBus;
use crate::actor::traits::Handler;
use crate::lifecycle_tracker::FeatureLifecycle;

pub trait EventSubscription<A> {
    fn subscribe_into(addr: Addr<A>, tracker: &FeatureLifecycle);
}

impl<A, M> EventSubscription<A> for M
where
    M: Event,
    A: Handler<M> + 'static,
{
    fn subscribe_into(addr: Addr<A>, tracker: &FeatureLifecycle) {
        EventBus::subscribe::<A, M>(addr, tracker);
    }
}

pub struct EventBusBuilder<'a, A: 'static> {
    pub(crate) addr: Addr<A>,
    pub(crate) tracker: &'a FeatureLifecycle,
}

impl<'a, A: 'static> EventBusBuilder<'a, A> {
    pub fn batch<T>(self) -> Self
    where
        T: EventSubscription<A>,
    {
        T::subscribe_into(self.addr.clone(), self.tracker);
        self
    }
}

macro_rules! impl_subscription_for_tuple {
    ($($M:ident),+) => {
        impl<A, $($M),+> EventSubscription<A> for ($($M,)+)
        where
            $(A: Handler<$M> + 'static, $M: Event,)+
        {
            fn subscribe_into(addr: Addr<A>, tracker: &FeatureLifecycle) {
                $(
                    EventBus::subscribe::<A, $M>(addr.clone(), tracker);
                )+
            }
        }
    };
}

impl_subscription_for_tuple!(M1, M2);
impl_subscription_for_tuple!(M1, M2, M3);
impl_subscription_for_tuple!(M1, M2, M3, M4);
impl_subscription_for_tuple!(M1, M2, M3, M4, M5);
impl_subscription_for_tuple!(M1, M2, M3, M4, M5, M6);
