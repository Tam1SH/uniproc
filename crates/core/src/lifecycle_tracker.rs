use crate::actor::addr::Addr;
use crate::actor::event_bus::subscribe::SubscriptionId;
use crate::actor::event_bus::EventBus;
use crate::actor::UiThreadToken;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct FeatureLifecycle {
    inner: Rc<RefCell<LifecycleInner>>,
}

#[derive(Default)]
struct LifecycleInner {
    subs: Vec<SubscriptionId>,
    actor_counters: Vec<Rc<&'static str>>,
    anchors: Vec<Box<dyn Any>>,
}

impl FeatureLifecycle {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(LifecycleInner {
                subs: Vec::new(),
                actor_counters: Vec::new(),
                anchors: Vec::new(),
            })),
        }
    }

    pub fn shutdown(self, token: &UiThreadToken) {
        let mut inner = self.inner.borrow_mut();

        for sub_id in inner.subs.drain(..) {
            EventBus::unsubscribe(token, sub_id);
        }

        let counters = std::mem::take(&mut inner.actor_counters);

        inner.anchors.clear();

        for counter in counters {
            let count = Rc::strong_count(&counter);
            if count > 1 {
                tracing::error!(
                    "LEAK: Actor<{}> still alive (refs: {})",
                    *counter,
                    count - 1
                );
            }
        }
    }

    pub fn track_loop<T: 'static>(&self, handle: T) {
        self.inner.borrow_mut().anchors.push(Box::new(handle));
    }

    pub fn track_actor<A: 'static>(&self, addr: &Addr<A>) {
        self.inner
            .borrow_mut()
            .actor_counters
            .push(addr.strong_count_ptr());
    }

    pub fn track_sub(&self, id: SubscriptionId) {
        self.inner.borrow_mut().subs.push(id);
    }
}
