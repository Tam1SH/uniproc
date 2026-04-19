use crate::actor::addr::{Addr, REGISTRY};

use crate::actor::short_type_name;
use crate::trace::{current_meta, install_current_meta, DispatchMeta};

pub trait Message: 'static {}

pub trait Handler<M: Message>: 'static {
    fn handle(&mut self, msg: M, ctx: &Context<Self>)
    where
        Self: Sized;
}

pub struct Context<A: 'static> {
    pub(super) addr: Addr<A>,
}

impl<A: 'static> Context<A> {
    pub fn addr(&self) -> Addr<A> {
        self.addr.clone()
    }

    pub fn spawn_bg<M, Fut>(&self, fut: Fut)
    where
        M: Message + 'static + Send,
        A: Handler<M>,
        Fut: Future<Output = M> + 'static + Send,
    {
        let id = self.addr.id;
        let meta = current_meta().unwrap_or_else(|| DispatchMeta::capture_or_root("core.actor.bg"));
        let span = tracing::debug_span!(
            parent: &meta.span,
            "actor.bg",
            actor = short_type_name::<A>(),
            result = short_type_name::<M>(),
            op_id = meta.op_id,
            correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
        );

        #[cfg(feature = "test-utils")]
        use crate::actor::event_bus::ACTIVE_TASKS;

        #[cfg(feature = "test-utils")]
        ACTIVE_TASKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        tokio::spawn(async move {
            let _meta_guard = install_current_meta(meta.clone());
            let result = {
                let _enter = span.enter();
                fut.await
            };

            let return_task = move || {
                REGISTRY.with(|reg| {
                    if let Some(boxed_addr) = reg.borrow().get(&id) {
                        if let Some(addr) = boxed_addr.downcast_ref::<Addr<A>>() {
                            addr.send_with_meta(
                                result,
                                meta.child("core.actor.bg.result", None, None),
                            );
                        }
                    }

                    #[cfg(feature = "test-utils")]
                    ACTIVE_TASKS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                });
            };

            #[cfg(not(feature = "test-utils"))]
            let _ = slint::invoke_from_event_loop(return_task);

            #[cfg(feature = "test-utils")]
            crate::actor::event_bus::EventBus::queue_test_task(Box::new(return_task));
        });
    }
}

#[derive(Debug, Clone)]
pub struct NoOp;
impl Message for NoOp {}

impl<T: 'static> Handler<NoOp> for T {
    fn handle(&mut self, _: NoOp, _: &Context<Self>) {}
}
