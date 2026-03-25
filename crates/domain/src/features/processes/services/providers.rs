use crate::features::processes::services::icone_process::extract_icon_raw;
use app_core::icons::Icons;
use slint::{Image, SharedString};
use std::time::Duration;
use ttl_cache::TtlCache;

pub struct NameProvider {
    cache: TtlCache<String, SharedString>,
    ttl: Duration,
}

impl NameProvider {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: TtlCache::new(1000),
            ttl,
        }
    }

    pub fn get_clean(&mut self, raw_name: &str) -> SharedString {
        let clean = raw_name
            .strip_suffix(".exe")
            .or(raw_name.strip_suffix(".EXE"))
            .unwrap_or(raw_name);

        if let Some(cached) = self.cache.get(clean) {
            return cached.clone();
        }

        let shared = SharedString::from(clean);

        self.cache
            .insert(clean.to_string(), shared.clone(), self.ttl);
        shared
    }

    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
    }
}
pub struct IconProvider {
    cache: TtlCache<String, Image>,
    default_icon: Image,
    ttl: Duration,
}

impl IconProvider {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: TtlCache::new(256),
            default_icon: Icons::get("app"),
            ttl,
        }
    }

    pub fn get_icon(&mut self, path: &str) -> Image {
        if path.is_empty() {
            return self.default_icon.clone();
        }

        if let Some(cached) = self.cache.get(path) {
            return cached.clone();
        }

        let icon = extract_icon_raw(path).unwrap_or_else(|| self.default_icon.clone());

        self.cache.insert(path.to_string(), icon.clone(), self.ttl);
        icon
    }

    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
    }
}
