use crate::actor::addr::Addr;
use crate::actor::traits::{Handler, Message};
use crate::settings::SubscriptionId;
use slint::ComponentHandle;
use std::any::Any;
use std::marker::PhantomData;

pub trait Event: Message + Clone {}
impl<T: Message + Clone> Event for T {}

pub trait UntypedSubscriber: 'static {
    fn deliver(&self, msg: Box<dyn Any>);
    fn id(&self) -> SubscriptionId;
}

pub struct Subscriber<A: Handler<M, TWindow>, M: Event, TWindow: ComponentHandle + 'static> {
    pub(super) id: SubscriptionId,
    pub(super) addr: Addr<A, TWindow>,
    pub(super) _marker: PhantomData<M>,
}

impl<A, M, TWindow: ComponentHandle + 'static> UntypedSubscriber for Subscriber<A, M, TWindow>
where
    A: Handler<M, TWindow> + 'static,
    M: Event,
{
    fn deliver(&self, msg: Box<dyn Any>) {
        if let Ok(concrete_msg) = msg.downcast::<M>() {
            self.addr.send((*concrete_msg).clone());
        }
    }

    fn id(&self) -> SubscriptionId {
        self.id
    }
}
