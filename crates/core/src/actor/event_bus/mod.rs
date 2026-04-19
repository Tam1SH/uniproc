use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::{Event, Subscriber, SubscriptionId, UntypedSubscriber};
use crate::actor::short_type_name;
use crate::actor::traits::Handler;
use crate::actor::UiThreadToken;
use crate::trace::{current_meta, is_scope_enabled, DispatchMeta};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::actor::event_bus::builder::EventBusBuilder;
use crate::lifecycle_tracker::FeatureLifecycle;
use tracing::{debug, warn};

pub mod builder;
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

#[cfg(feature = "test-utils")]
pub static TEST_TASK_QUEUE: std::sync::LazyLock<std::sync::Mutex<Vec<Box<dyn FnOnce() + Send>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

#[cfg(feature = "test-utils")]
pub static ACTIVE_TASKS: AtomicUsize = AtomicUsize::new(0);

pub struct EventBus;

impl EventBus {
    pub fn subscribe_to<A>(addr: Addr<A>, tracker: &FeatureLifecycle) -> EventBusBuilder<'_, A> {
        EventBusBuilder { addr, tracker }
    }

    pub fn subscribe<A, M>(addr: Addr<A>, tracker: &FeatureLifecycle)
    where
        A: Handler<M> + 'static,
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

        if is_scope_enabled("core.bus.subscribe") {
            debug!(
                event = short_type_name::<M>(),
                actor = short_type_name::<A>(),
                "bus.subscribe"
            );
        }

        tracker.track_sub(id);
    }

    pub fn count_subscribers<M: Event>() -> usize {
        let type_id = TypeId::of::<M>();
        *REGISTRY.counts.read().get(&type_id).unwrap_or(&0)
    }

    pub fn has_subscribers<M: Event>() -> bool {
        Self::count_subscribers::<M>() > 0
    }

    pub fn publish<M: Event>(msg: M) {
        let meta =
            current_meta().unwrap_or_else(|| DispatchMeta::capture_or_root("core.bus.publish"));

        if !Self::has_subscribers::<M>() {
            warn!(
                parent: &meta.span,
                event = short_type_name::<M>(),
                op_id = meta.op_id,
                correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                "no subscribers"
            );

            return;
        }

        if is_scope_enabled("core.bus.publish") {
            debug!(
                parent: &meta.span,
                event = short_type_name::<M>(),
                op_id = meta.op_id,
                correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                "bus.publish"
            );
        }

        let task = move || {
            let type_id = TypeId::of::<M>();
            LOCAL_SUBSCRIBERS.with(|s| {
                if let Some(subs) = s.borrow().get(&type_id) {
                    for sub in subs {
                        sub.deliver(
                            Box::new(msg.clone()),
                            meta.child("core.bus.publish", None, None),
                        );
                    }
                }
            });
        };

        #[cfg(not(feature = "test-utils"))]
        let _ = slint::invoke_from_event_loop(task);
        #[cfg(feature = "test-utils")]
        Self::queue_test_task(Box::new(task));
    }

    pub fn unsubscribe(_guard: &UiThreadToken, id: SubscriptionId) {
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

#[cfg(feature = "test-utils")]
impl EventBus {
    pub fn queue_test_task(task: Box<dyn FnOnce() + Send>) {
        TEST_TASK_QUEUE.lock().unwrap().push(task);
    }
    pub fn process_queue() {
        let tasks: Vec<_> = std::mem::take(&mut *TEST_TASK_QUEUE.lock().unwrap());
        for task in tasks {
            task();
        }
    }

    pub fn is_queue_empty() -> bool {
        TEST_TASK_QUEUE.lock().unwrap().is_empty()
    }

    pub fn task_count() -> usize {
        ACTIVE_TASKS.load(Ordering::SeqCst)
    }
}
