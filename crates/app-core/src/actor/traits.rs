use crate::actor::addr::{Addr, REGISTRY};
use crate::actor::event_bus::subscribe::Event;
use crate::actor::event_bus::EVENT_BUS;
use crate::messages;
use slint::ComponentHandle;

pub trait Message: 'static {}

pub trait Handler<M: Message, TWindow: ComponentHandle + 'static>: 'static {
    fn handle(&mut self, msg: M, ctx: &Context<Self, TWindow>)
    where
        Self: Sized;
}

pub struct Context<A: 'static, TWindow: ComponentHandle + 'static> {
    pub(super) addr: Addr<A, TWindow>,
    pub ui_weak: slint::Weak<TWindow>,
}

impl<A: 'static, TWindow: ComponentHandle> Context<A, TWindow> {
    pub fn addr(&self) -> Addr<A, TWindow> {
        self.addr.clone()
    }

    pub fn with_ui<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&TWindow) -> R,
    {
        self.ui_weak.upgrade().map(|ui| f(&ui))
    }

    pub fn subscribe<M>(&self)
    where
        M: Event,
        A: Handler<M, TWindow>,
    {
        let addr = self.addr();
        EVENT_BUS.with(|bus| bus.subscribe::<A, M, TWindow>(addr));
    }

    pub fn publish<M: Event>(&self, msg: M) {
        EVENT_BUS.with(|bus| bus.publish(msg));
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

        tokio::spawn(async move {
            let result = fut.await;

            let _ = slint::invoke_from_event_loop(move || {
                REGISTRY.with(|reg| {
                    if let Some(boxed_addr) = reg.borrow().get(&id) {
                        if let Some(addr) = boxed_addr.downcast_ref::<Addr<A, TWindow>>() {
                            addr.send(result);
                        }
                    }
                });
            });
        });
    }
}

messages! { NoOp }
impl<T: 'static, TWindow: ComponentHandle + 'static> Handler<NoOp, TWindow> for T {
    fn handle(&mut self, _: NoOp, _: &Context<Self, TWindow>) {}
}
