use slint::SharedString;
use std::cell::RefCell;
use std::time::Duration;
use ttl_cache::TtlCache;

pub struct StringsProvider {
    cache: RefCell<TtlCache<String, SharedString>>,
    ttl: Duration,
}

thread_local! {
    static NAME_PROVIDER: StringsProvider = StringsProvider::new(Duration::from_secs(60));
}

impl StringsProvider {
    pub fn global<R>(f: impl FnOnce(&Self) -> R) -> R {
        NAME_PROVIDER.with(f)
    }

    fn new(ttl: Duration) -> Self {
        Self {
            cache: RefCell::new(TtlCache::new(1000)),
            ttl,
        }
    }

    pub fn get_clean(&self, raw_name: &str) -> SharedString {
        let clean = raw_name
            .strip_suffix(".exe")
            .or_else(|| raw_name.strip_suffix(".EXE"))
            .unwrap_or(raw_name);

        let mut cache = self.cache.borrow_mut();

        if let Some(cached) = cache.get(clean) {
            return cached.clone();
        }

        let shared = SharedString::from(clean);
        cache.insert(clean.to_string(), shared.clone(), self.ttl);
        shared
    }
}
