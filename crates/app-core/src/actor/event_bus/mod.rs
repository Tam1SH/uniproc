use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::{Event, Subscriber, UntypedSubscriber};
use crate::actor::traits::Handler;
use crate::settings::SubscriptionId;
use slint::ComponentHandle;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;

pub mod subscribe;

pub struct EventBus {
    subscribers: RefCell<HashMap<TypeId, Vec<Box<dyn UntypedSubscriber>>>>,
    next_id: RefCell<usize>,
}

impl EventBus {
    fn new() -> Self {
        Self {
            subscribers: RefCell::new(HashMap::new()),
            next_id: RefCell::new(0),
        }
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        let mut subs = self.subscribers.borrow_mut();
        for list in subs.values_mut() {
            list.retain(|subscriber| subscriber.id() != id);
        }
    }

    pub fn subscribe<A, M, TWindow: ComponentHandle + 'static>(
        &self,
        addr: Addr<A, TWindow>,
    ) -> SubscriptionId
    where
        A: Handler<M, TWindow> + 'static,
        M: Event,
    {
        let type_id = TypeId::of::<M>();
        let id: SubscriptionId = {
            let mut next = self.next_id.borrow_mut();
            let id = *next;
            *next += 1;
            id as SubscriptionId
        };

        let subscriber = Box::new(Subscriber {
            id,
            addr,
            _marker: std::marker::PhantomData,
        });

        self.subscribers
            .borrow_mut()
            .entry(type_id)
            .or_insert_with(Vec::new)
            .push(subscriber);

        id
    }

    pub fn publish<M: Event>(&self, msg: M) {
        let type_id = TypeId::of::<M>();
        if let Some(subs) = self.subscribers.borrow().get(&type_id) {
            for sub in subs {
                sub.deliver(Box::new(msg.clone()));
            }
        }
    }

    pub fn has_subscribers<M: Event>(&self) -> bool {
        let type_id = TypeId::of::<M>();
        self.subscribers
            .borrow()
            .get(&type_id)
            .map(|subs| !subs.is_empty())
            .unwrap_or(false)
    }
}

thread_local! {
    pub static EVENT_BUS: EventBus = EventBus::new();
}
