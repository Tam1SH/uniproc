use crate::actor::traits::{Context, Handler, Message};
use slint::ComponentHandle;

pub trait Envelope<A, TWindow: ComponentHandle + 'static> {
    fn handle(&mut self, actor: &mut A, ctx: &Context<A, TWindow>);
}

pub struct MessageEnvelope<M: Message> {
    pub(super) message: Option<M>,
}

impl<A, M: Message, TWindow: ComponentHandle + 'static> Envelope<A, TWindow> for MessageEnvelope<M>
where
    A: Handler<M, TWindow>,
{
    fn handle(&mut self, actor: &mut A, ctx: &Context<A, TWindow>) {
        if let Some(m) = self.message.take() {
            actor.handle(m, ctx);
        }
    }
}
