use slint::SharedString;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use ttl_cache::TtlCache;

pub struct StringsProvider {
    cache: Mutex<TtlCache<String, SharedString>>,
    ttl: Duration,
}

impl StringsProvider {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<StringsProvider> = OnceLock::new();
        INSTANCE.get_or_init(|| Self::new(Duration::from_secs(1200), 1500))
    }

    fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            cache: Mutex::new(TtlCache::new(capacity)),
            ttl,
        }
    }

    pub fn intern(&self, s: &str) -> SharedString {
        let mut cache = self.cache.lock().unwrap();

        if let Some(cached) = cache.get(s) {
            return cached.clone();
        }

        let shared = SharedString::from(s);
        cache.insert(s.to_string(), shared.clone(), self.ttl);
        shared
    }

    pub fn get_stripped(&self, raw: &str) -> SharedString {
        let clean = match raw.rfind('.') {
            Some(idx) if idx > 0 => &raw[..idx],
            _ => raw,
        };

        self.intern(clean)
    }
}
