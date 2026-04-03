use bon::Builder;
use crate::icons::Icons;
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

#[derive(Builder)]
pub struct IconRequest<'a> {
    pub path: &'a str,

    #[cfg(windows)]
    pub package_full_name: Option<&'a str>,
}

impl IconProvider {
    pub fn global<R>(f: impl FnOnce(&Self) -> R) -> R {
        ICON_PROVIDER.with(f)
    }

    fn new(ttl: Duration) -> Self {
        Self {
            cache: RefCell::new(TtlCache::new(512)),
            default_icon: Icons::get("app"),
            ttl,
        }
    }

    pub fn get_icon(&self, req: IconRequest) -> Image {
        if req.path.is_empty() {
            return self.default_icon.clone();
        }

        let mut cache = self.cache.borrow_mut();

        if let Some(cached) = cache.get(req.path) {
            return cached.clone();
        }

        let icon = {
            #[cfg(windows)]
            {
                let pkg_name = req.package_full_name.unwrap_or_default();
                if !pkg_name.is_empty() {
                    windows::extract_appx_icon(pkg_name, 16)
                        .unwrap_or_else(|| self.default_icon.clone())
                } else {
                    windows::extract_icon_raw(req.path).unwrap_or_else(|| self.default_icon.clone())
                }
            }
            #[cfg(not(windows))]
            {
                self.default_icon.clone()
            }
        };

        cache.insert(req.path.to_string(), icon.clone(), self.ttl);
        icon
    }
}

