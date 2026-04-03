use crate::actor::short_type_name;
use crate::actor::traits::{Context, Handler, Message};
use crate::app::Window;
use crate::trace::{install_current_meta, DispatchMeta};

pub trait Envelope<A, TWindow: Window> {
    fn handle(&mut self, actor: &mut A, ctx: &Context<A, TWindow>);
}

pub struct MessageEnvelope<M: Message> {
    pub(super) message: Option<M>,
    pub(super) meta: DispatchMeta,
}

impl<A, M: Message, TWindow: Window> Envelope<A, TWindow> for MessageEnvelope<M>
where
    A: Handler<M, TWindow>,
{
    fn handle(&mut self, actor: &mut A, ctx: &Context<A, TWindow>) {
        if let Some(m) = self.message.take() {
            let _meta_guard = install_current_meta(self.meta.clone());
            let span = tracing::debug_span!(
                parent: &self.meta.span,
                "actor.handle",
                actor = short_type_name::<A>(),
                message = short_type_name::<M>(),
                op_id = self.meta.op_id,
                correlation_id = self.meta.correlation_id.as_deref().unwrap_or(""),
            );
            let _enter = span.enter();
            actor.handle(m, ctx);
        }
    }
}
