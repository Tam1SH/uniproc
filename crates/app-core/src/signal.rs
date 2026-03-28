use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct Signal<T> {
    pub(crate) value: Arc<ArcSwap<T>>,
    pub(crate) subscribers: Arc<Mutex<Vec<(u64, Arc<dyn Fn(&T) + Send + Sync + 'static>)>>>,
    pub(crate) next_id: AtomicU64,
}

pub struct SignalSubscription {
    pub(crate) id: u64,
    pub(crate) cleanup: Arc<dyn Fn(u64) + Send + Sync + 'static>,
}

impl Drop for SignalSubscription {
    fn drop(&mut self) {
        (self.cleanup)(self.id);
    }
}

impl<T: Send + Sync + 'static> Signal<T> {
    pub fn new(initial: T) -> Self {
        Self {
            value: Arc::new(ArcSwap::from_pointee(initial)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
            next_id: AtomicU64::new(0),
        }
    }

    pub fn get_arc(&self) -> Arc<T> {
        self.value.load_full()
    }

    pub fn set(&self, new_value: T) {
        let arc_val = Arc::new(new_value);
        self.value.store(arc_val.clone());
        let subs = self.subscribers.lock().unwrap();
        for (_, cb) in subs.iter() {
            cb(&arc_val);
        }
    }

    pub fn subscribe<F>(&self, callback: F) -> SignalSubscription
    where
        F: Fn(&T) + Send + Sync + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut subs = self.subscribers.lock().unwrap();
        subs.push((id, Arc::new(callback)));

        let subscribers = Arc::clone(&self.subscribers);
        SignalSubscription {
            id,
            cleanup: Arc::new(move |id| {
                if let Ok(mut subs) = subscribers.lock() {
                    subs.retain(|(sid, _)| *sid != id);
                }
            }),
        }
    }
}

impl<T: Send + Sync + Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        self.value.load().as_ref().clone()
    }
}
