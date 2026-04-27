use crate::actor::{short_type_name, Context};
use crate::actor::traits::{Handler, Message};
use crate::trace::{install_current_meta, is_message_enabled, is_scope_enabled, DispatchMeta};
use std::marker::PhantomData;

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
            if is_scope_enabled("core.actor.handle") && is_message_enabled(message_name) {
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

pub struct FnEnvelope<A, F>
where
    F: FnOnce(&mut A, &Context<A>) + Send + 'static,
{
    pub(super) func: Option<F>,
    pub(super) meta: DispatchMeta,
    pub(super) phantom: PhantomData<A>,
}

impl<A, F> Envelope<A> for FnEnvelope<A, F>
where
    F: FnOnce(&mut A, &Context<A>) + Send + 'static,
{
    fn handle(&mut self, actor: &mut A, ctx: &Context<A>) {
        if let Some(f) = self.func.take() {
            let _meta_guard = install_current_meta(self.meta.clone());
            let _enter = self.meta.span.enter();
            f(actor, ctx);
        }
    }
}
