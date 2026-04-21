use crate::actor::traits::{Context, Handler, Message};
use crate::actor::{short_type_name, should_trace_actor_message};
use crate::trace::{DispatchMeta, install_current_meta, is_message_enabled, is_scope_enabled};

pub trait Envelope<A> {
    fn handle(&mut self, actor: &mut A, ctx: &Context<A>);
}

pub struct MessageEnvelope<M: Message> {
    pub(super) message: Option<M>,
    pub(super) meta: DispatchMeta,
}

impl<A, M: Message> Envelope<A> for MessageEnvelope<M>
where
    A: Handler<M>,
{
    fn handle(&mut self, actor: &mut A, ctx: &Context<A>) {
        if let Some(m) = self.message.take() {
            let _meta_guard = install_current_meta(self.meta.clone());
            let message_name = short_type_name::<M>();
            if is_scope_enabled("core.actor.handle")
                && should_trace_actor_message(message_name)
                && is_message_enabled(message_name)
            {
                tracing::debug!(
                    parent: &self.meta.span,
                    actor = short_type_name::<A>(),
                    message = message_name,
                    op_id = self.meta.op_id,
                    correlation_id = self.meta.correlation_id.as_deref().unwrap_or(""),
                    "actor.handle"
                );
            }
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
