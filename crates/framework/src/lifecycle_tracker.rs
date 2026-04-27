use app_core::actor::addr::Addr;
use app_core::actor::event_bus::subscribe::SubscriptionId;
use app_core::actor::event_bus::EventBus;
use app_core::actor::UiThreadToken;
use app_core::lifecycle_tracker::LifecycleTracker;
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

impl LifecycleTracker for FeatureLifecycle {
    fn shutdown(self, token: &UiThreadToken) {
        FeatureLifecycle::shutdown(self, token);
    }

    fn track_loop<T: 'static>(&self, handle: T) {
        FeatureLifecycle::track_loop(self, handle);
    }

    fn track_actor<A: 'static>(&self, addr: &Addr<A>) {
        FeatureLifecycle::track_actor(self, addr);
    }

    fn track_sub(&self, id: SubscriptionId) {
        FeatureLifecycle::track_sub(self, id);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use app_core::actor::UiThreadToken;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct DropCheck(Arc<AtomicUsize>);
    impl Drop for DropCheck {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_lifecycle_anchors_cleanup() {
        let lifecycle = FeatureLifecycle::new();
        let counter = Arc::new(AtomicUsize::new(0));

        lifecycle.track_loop(DropCheck(counter.clone()));
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        let token = unsafe { UiThreadToken::new() };
        lifecycle.shutdown(&token);

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_lifecycle_subs_drain() {
        let lifecycle = FeatureLifecycle::new();
        lifecycle.track_sub(1);
        lifecycle.track_sub(2);

        {
            let inner = lifecycle.inner.borrow();
            assert_eq!(inner.subs.len(), 2);
        }

        let token = unsafe { UiThreadToken::new() };
        lifecycle.clone().shutdown(&token);

        {
            let inner = lifecycle.inner.borrow();
            assert_eq!(inner.subs.len(), 0);
        }
    }

    #[test]
    fn test_lifecycle_actor_leak_detection_logic() {
        let lifecycle = FeatureLifecycle::new();

        let counter_rc = Rc::new("test_actor");

        lifecycle
            .inner
            .borrow_mut()
            .actor_counters
            .push(counter_rc.clone());

        assert_eq!(Rc::strong_count(&counter_rc), 2);

        let token = unsafe { UiThreadToken::new() };

        lifecycle.clone().shutdown(&token);
        assert_eq!(Rc::strong_count(&counter_rc), 1);

        let lifecycle_leak = FeatureLifecycle::new();
        let leak_rc = Rc::new("leaked_actor");
        let _keep_alive = leak_rc.clone();

        lifecycle_leak
            .inner
            .borrow_mut()
            .actor_counters
            .push(leak_rc.clone());

        lifecycle_leak.shutdown(&token);

        assert_eq!(Rc::strong_count(&leak_rc), 2);
    }
}
