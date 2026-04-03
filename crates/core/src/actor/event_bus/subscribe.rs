use crate::actor::addr::Addr;
use crate::actor::short_type_name;
use crate::actor::traits::{Handler, Message};
use crate::app::Window;
use crate::trace::DispatchMeta;

use std::any::Any;
use std::marker::PhantomData;

pub type SubscriptionId = u64;

pub trait Event: Message + Send + Clone {}
impl<T: Message + Clone + Send> Event for T {}

pub trait UntypedSubscriber: 'static {
    fn deliver(&self, msg: Box<dyn Any>, meta: DispatchMeta);
    fn id(&self) -> SubscriptionId;
}

pub struct Subscriber<A: Handler<M, TWindow>, M: Event, TWindow: Window> {
    pub(super) id: SubscriptionId,
    pub(super) addr: Addr<A, TWindow>,
    pub(super) _marker: PhantomData<M>,
}

impl<A, M, TWindow: Window> UntypedSubscriber for Subscriber<A, M, TWindow>
where
    A: Handler<M, TWindow> + 'static,
    M: Event,
{
    fn deliver(&self, msg: Box<dyn Any>, meta: DispatchMeta) {
        if let Ok(concrete_msg) = msg.downcast::<M>() {
            tracing::debug!(
                parent: &meta.span,
                event = short_type_name::<M>(),
                actor = short_type_name::<A>(),
                op_id = meta.op_id,
                correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                "bus.deliver"
            );
            self.addr
                .send_with_meta((*concrete_msg).clone(), meta.child("core.bus.deliver", None, None));
        }
    }

    fn id(&self) -> SubscriptionId {
        self.id
    }
}
