use crate::features::processes::icone_process::extract_icon_raw;
use slint::{Image, SharedString};
use std::time::Duration;
use ttl_cache::TtlCache;

pub struct NameProvider {
    cache: TtlCache<String, SharedString>,
}

impl NameProvider {
    pub fn new() -> Self {
        Self {
            cache: TtlCache::new(1000),
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
            .insert(clean.to_string(), shared.clone(), Duration::from_secs(300));
        shared
    }
}
pub struct IconProvider {
    cache: TtlCache<String, Image>,
    default_icon: Image,
}

impl IconProvider {
    pub fn new() -> Self {
        Self {
            cache: TtlCache::new(256),
            default_icon: Image::load_from_svg_data(include_bytes!("../../../ui/assets/app.svg"))
                .unwrap(),
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

        self.cache
            .insert(path.to_string(), icon.clone(), Duration::from_secs(120));
        icon
    }
}
