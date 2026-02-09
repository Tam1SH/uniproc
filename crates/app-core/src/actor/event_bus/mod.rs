use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::{Event, Subscriber, UntypedSubscriber};
use crate::actor::traits::Handler;
use slint::ComponentHandle;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;

pub mod subscribe;

pub struct EventBus {
    subscribers: RefCell<HashMap<TypeId, Vec<Box<dyn UntypedSubscriber>>>>,
}

impl EventBus {
    fn new() -> Self {
        Self {
            subscribers: RefCell::new(HashMap::new()),
        }
    }

    pub fn subscribe<A, M, TWindow: ComponentHandle + 'static>(&self, addr: Addr<A, TWindow>)
    where
        A: Handler<M, TWindow> + 'static,
        M: Event,
    {
        let type_id = TypeId::of::<M>();
        let subscriber = Box::new(Subscriber {
            addr,
            _marker: std::marker::PhantomData,
        });

        self.subscribers
            .borrow_mut()
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(subscriber);
    }

    pub fn publish<M: Event>(&self, msg: M) {
        let type_id = TypeId::of::<M>();
        if let Some(subs) = self.subscribers.borrow().get(&type_id) {
            for sub in subs {
                sub.deliver(Box::new(msg.clone()));
            }
        }
    }
}

thread_local! {
    pub static EVENT_BUS: EventBus = EventBus::new();
}
