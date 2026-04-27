use crate::actor::event_bus::builder::EventSubscription;
use crate::actor::{short_type_name, Context};

pub trait Message: 'static {}

pub trait Handler<M: Message>: 'static {
    fn handle(&mut self, msg: M, ctx: &Context<Self>)
    where
        Self: Sized;
}

pub trait DirectHandler<A> {}
impl<A, M> DirectHandler<A> for M
where
    A: Handler<M> + 'static,
    M: Message,
{
}

pub trait ManagedActor: Sized + 'static {
    type Bus: EventSubscription<Self>;
    type Handlers: DirectHandler<Self>;
}


#[derive(Debug, Clone)]
pub struct NoOp;
impl Message for NoOp {}

impl<T: 'static> Handler<NoOp> for T {
    fn handle(&mut self, _: NoOp, _: &Context<Self>) {}
}
