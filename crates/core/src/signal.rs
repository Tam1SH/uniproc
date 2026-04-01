use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Signal<T> {
    pub(crate) value: Arc<ArcSwap<T>>,
    pub(crate) subscribers: Arc<Mutex<Vec<(u64, Arc<dyn Fn(&T) + Send + Sync + 'static>)>>>,
    pub(crate) next_id: Arc<AtomicU64>,
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

impl<T: 'static> Signal<T> {
    pub fn new(initial: T) -> Self {
        Self {
            value: Arc::new(ArcSwap::from_pointee(initial)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn get_arc(&self) -> Arc<T> {
        self.value.load_full()
    }

    pub fn store_arc(&self, arc: Arc<T>) {
        self.value.store(arc);
        self.emit();
    }

    pub fn set(&self, new_value: T) {
        self.value.store(Arc::new(new_value));
        self.emit();
    }

    fn emit(&self) {
        let val = self.value.load_full();

        let callbacks: Vec<_> = {
            let subs = self.subscribers.lock().unwrap();
            subs.iter().map(|(_, cb)| cb.clone()).collect()
        };

        for cb in callbacks {
            cb(&val);
        }
    }

    pub fn subscribe<F>(&self, callback: F) -> SignalSubscription
    where
        F: Fn(&T) + Send + Sync + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut subs = self.subscribers.lock().unwrap();
        subs.push((id, Arc::new(callback)));

        let subscribers = self.subscribers.clone();
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

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        self.value.load().as_ref().clone()
    }
}
