use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::{Event, Subscriber, UntypedSubscriber};
use crate::actor::traits::Handler;
use crate::actor::UiThreadGuard;
use crate::app::Window;
use crate::settings::SubscriptionId;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub mod subscribe;

struct Registry {
    counts: RwLock<HashMap<TypeId, usize>>,
    next_id: AtomicUsize,
}

static REGISTRY: Lazy<Registry> = Lazy::new(|| Registry {
    counts: RwLock::new(HashMap::new()),
    next_id: AtomicUsize::new(0),
});

thread_local! {
    static LOCAL_SUBSCRIBERS: RefCell<HashMap<TypeId, Vec<Box<dyn UntypedSubscriber>>>> = RefCell::new(HashMap::new());
}

pub struct EventBus;

impl EventBus {
    pub fn subscribe<A, M, TWindow: Window>(
        _guard: &UiThreadGuard,
        addr: Addr<A, TWindow>,
    ) -> SubscriptionId
    where
        A: Handler<M, TWindow> + 'static,
        M: Event,
    {
        let type_id = TypeId::of::<M>();
        let id = REGISTRY.next_id.fetch_add(1, Ordering::SeqCst) as SubscriptionId;

        *REGISTRY.counts.write().entry(type_id).or_insert(0) += 1;

        let subscriber = Box::new(Subscriber {
            id,
            addr,
            _marker: std::marker::PhantomData,
        });

        LOCAL_SUBSCRIBERS.with(|s| {
            s.borrow_mut()
                .entry(type_id)
                .or_insert_with(Vec::new)
                .push(subscriber);
        });

        id
    }

    pub fn has_subscribers<M: Event>() -> bool {
        let type_id = TypeId::of::<M>();
        REGISTRY
            .counts
            .read()
            .get(&type_id)
            .map_or(false, |&c| c > 0)
    }

    pub fn publish<M: Event>(msg: M) {
        if !Self::has_subscribers::<M>() {
            return;
        }

        let _ = slint::invoke_from_event_loop(move || {
            let type_id = TypeId::of::<M>();
            LOCAL_SUBSCRIBERS.with(|s| {
                if let Some(subs) = s.borrow().get(&type_id) {
                    for sub in subs {
                        sub.deliver(Box::new(msg.clone()));
                    }
                }
            });
        });
    }

    pub fn unsubscribe(_guard: &UiThreadGuard, id: SubscriptionId) {
        LOCAL_SUBSCRIBERS.with(|s| {
            let mut s = s.borrow_mut();
            for (type_id, list) in s.iter_mut() {
                let start_len = list.len();
                list.retain(|sub| sub.id() != id);
                let removed = start_len - list.len();

                if removed > 0 {
                    *REGISTRY.counts.write().entry(*type_id).or_insert(0) -= removed;
                }
            }
        });
    }
}
