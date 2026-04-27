use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::Event;
use crate::actor::event_bus::{EventBus, RpcCall, RpcRequest};
use crate::actor::traits::Handler;
use crate::actor::DirectHandler;
use crate::lifecycle_tracker::LifecycleTracker;

pub trait EventSubscription<A> {
    fn subscribe_into(addr: Addr<A>, tracker: &impl LifecycleTracker);
}

impl<A, M> EventSubscription<A> for M
where
    M: Event,
    A: Handler<M> + 'static,
{
    fn subscribe_into(addr: Addr<A>, tracker: &impl LifecycleTracker) {
        EventBus::subscribe::<A, M>(addr, tracker);
    }
}

impl<A> EventSubscription<A> for () {
    fn subscribe_into(_: Addr<A>, _: &impl LifecycleTracker) {}
}

pub trait EventBatch<A, L: LifecycleTracker> {
    fn subscribe_batch(builder: EventBusBuilder<A, L>) -> EventBusBuilder<A, L>;
}

pub struct EventBusBuilder<'a, A: 'static, L: LifecycleTracker> {
    pub(crate) addr: Addr<A>,
    pub(crate) tracker: &'a L,
}

impl<'a, A: 'static, L: LifecycleTracker> EventBusBuilder<'a, A, L> {
    pub fn batch<T>(self) -> Self
    where
        T: EventBatch<A, L>,
    {
        T::subscribe_batch(self)
    }

    pub fn subscribe<M>(self) -> Self
    where
        M: Event,
        A: Handler<M> + 'static,
    {
        EventBus::subscribe::<A, M>(self.addr.clone(), self.tracker);
        self
    }

    pub fn rpc<Req>(self) -> Self
    where
        Req: RpcCall,
        A: Handler<RpcRequest<Req>> + 'static,
    {
        EventBus::subscribe::<A, RpcRequest<Req>>(self.addr.clone(), self.tracker);
        self
    }
}

impl<A, L, M> EventBatch<A, L> for M
where
    A: Handler<M> + 'static,
    L: LifecycleTracker,
    M: Event,
{
    fn subscribe_batch(builder: EventBusBuilder<A, L>) -> EventBusBuilder<A, L> {
        builder.subscribe::<M>()
    }
}

macro_rules! impl_batch_for_tuple {
    ($($M:ident),+) => {
        impl<A, L, $($M),+> EventBatch<A, L> for ($($M,)+)
        where
            L: LifecycleTracker,
            $(A: Handler<$M> + 'static, $M: Event,)+
        {
            fn subscribe_batch(builder: EventBusBuilder<A, L>) -> EventBusBuilder<A, L> {
                $(
                    EventBus::subscribe::<A, $M>(builder.addr.clone(), builder.tracker);
                )+
                builder
            }
        }
        impl<A, $($M),+> DirectHandler<A> for ($($M,)+)
            where $( $M: DirectHandler<A> ),+
        {}

        impl<A, $($M),+> EventSubscription<A> for ($($M,)+)
        where
            $($M: EventSubscription<A>),+
        {
            fn subscribe_into(addr: Addr<A>, tracker: &impl LifecycleTracker) {
                $(
                    $M::subscribe_into(addr.clone(), tracker);
                )+
            }
        }

    };
}

impl_batch_for_tuple!(M1, M2);
impl_batch_for_tuple!(M1, M2, M3);
impl_batch_for_tuple!(M1, M2, M3, M4);
impl_batch_for_tuple!(M1, M2, M3, M4, M5);
