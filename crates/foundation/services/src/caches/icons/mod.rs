use app_core::icons::Icons;
use slint::Image;
use std::cell::RefCell;
use std::time::Duration;
use ttl_cache::TtlCache;

#[cfg(windows)]
pub mod windows;

pub struct IconProvider {
    cache: RefCell<TtlCache<String, Image>>,
    default_icon: Image,
    ttl: Duration,
}

thread_local! {
    static ICON_PROVIDER: IconProvider = IconProvider::new(Duration::from_secs(3600));
}

impl IconProvider {
    pub fn global<R>(f: impl FnOnce(&Self) -> R) -> R {
        ICON_PROVIDER.with(f)
    }

    fn new(ttl: Duration) -> Self {
        Self {
            cache: RefCell::new(TtlCache::new(256)),
            default_icon: Icons::get("app"),
            ttl,
        }
    }

    pub fn get_icon(&self, path: &str) -> Image {
        if path.is_empty() {
            return self.default_icon.clone();
        }

        let mut cache = self.cache.borrow_mut();

        if let Some(cached) = cache.get(path) {
            return cached.clone();
        }

        let icon = {
            #[cfg(windows)]
            {
                crate::caches::icons::windows::extract_icon_raw(path)
                    .unwrap_or_else(|| self.default_icon.clone())
            }
            #[cfg(not(windows))]
            {
                self.default_icon.clone()
            }
        };

        cache.insert(path.to_string(), icon.clone(), self.ttl);
        icon
    }
}
