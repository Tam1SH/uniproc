use crate::actor::addr::{Addr, REGISTRY};
use crate::actor::short_type_name;
use crate::app::Window;
use crate::trace::{current_meta, install_current_meta, DispatchMeta};

pub trait Message: 'static {}

pub trait Handler<M: Message, TWindow: Window>: 'static {
    fn handle(&mut self, msg: M, ctx: &Context<Self, TWindow>)
    where
        Self: Sized;
}

pub struct Context<A: 'static, TWindow: Window> {
    pub(super) addr: Addr<A, TWindow>,
    pub ui_weak: slint::Weak<TWindow>,
}

impl<A: 'static, TWindow: Window> Context<A, TWindow> {
    pub fn addr(&self) -> Addr<A, TWindow> {
        self.addr.clone()
    }

    pub fn spawn_task<M, Fut, S>(&self, fut: Fut, mut loading_setter: S)
    where
        M: Message + 'static + Send,
        A: Handler<M, TWindow>,
        Fut: Future<Output = M> + 'static + Send,
        S: FnMut(&TWindow, bool) + 'static + Send,
    {
        let ui_weak = self.ui_weak.clone();

        if let Some(ui) = ui_weak.upgrade() {
            loading_setter(&ui, true);
        }

        let mut loading_setter = loading_setter;

        let wrapped_fut = async move {
            let result = fut.await;

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    loading_setter(&ui, false);
                }
            });
            result
        };

        self.spawn_bg(wrapped_fut);
    }

    pub fn spawn_bg<M, Fut>(&self, fut: Fut)
    where
        M: Message + 'static + Send,
        A: Handler<M, TWindow>,
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

        tokio::spawn(async move {
            let _meta_guard = install_current_meta(meta.clone());
            let result = {
                let _enter = span.enter();
                fut.await
            };

            let _ = slint::invoke_from_event_loop(move || {
                REGISTRY.with(|reg| {
                    if let Some(boxed_addr) = reg.borrow().get(&id) {
                        if let Some(addr) = boxed_addr.downcast_ref::<Addr<A, TWindow>>() {
                            addr.send_with_meta(result, meta.child("core.actor.bg.result", None, None));
                        }
                    }
                });
            });
        });
    }
}

#[derive(Debug, Clone)]
pub struct NoOp;
impl Message for NoOp {}

impl<T: 'static, TWindow: Window> Handler<NoOp, TWindow> for T {
    fn handle(&mut self, _: NoOp, _: &Context<Self, TWindow>) {}
}
