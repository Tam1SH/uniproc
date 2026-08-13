mod encode;
mod extract;

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;
use ttl_cache::TtlCache;

pub use extract::has_own_icon;

pub struct IconRequest<'a> {
    pub path: &'a str,
    pub package_full_name: Option<&'a str>,
}

pub struct IconCache {
    cache: RefCell<TtlCache<String, PathBuf>>,
    cache_dir: PathBuf,
    ttl: Duration,
}

impl IconCache {
    pub fn new() -> Self {
        Self::with_cache_dir(std::env::temp_dir().join("uniproc-icons"), Duration::from_secs(3600))
    }

    pub fn with_cache_dir(cache_dir: PathBuf, ttl: Duration) -> Self {
        if let Err(err) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!(?err, ?cache_dir, "context2: failed to create icon cache dir");
        }
        Self {
            cache: RefCell::new(TtlCache::new(512)),
            cache_dir,
            ttl,
        }
    }

    pub fn icon_path(&self, req: IconRequest) -> Option<PathBuf> {
        let key = req
            .package_full_name
            .filter(|s| !s.is_empty())
            .unwrap_or(req.path);
        if key.is_empty() {
            return None;
        }

        if let Some(cached) = self.cache.borrow().get(key) {
            return Some(cached.clone());
        }

        let file_path = self.cache_dir.join(format!("{:x}.png", cache_key_hash(key)));
        if file_path.is_file() {
            self.cache
                .borrow_mut()
                .insert(key.to_string(), file_path.clone(), self.ttl);
            return Some(file_path);
        }

        let image = match req.package_full_name.filter(|s| !s.is_empty()) {
            Some(pkg) => extract::extract_appx_icon_rgba(pkg, 32),
            None => extract::extract_icon_rgba(req.path),
        }?;

        if let Err(err) = encode::write_png(&file_path, image.width, image.height, &image.pixels) {
            tracing::warn!(?err, key, "context2: failed to write cached icon png");
            return None;
        }

        self.cache
            .borrow_mut()
            .insert(key.to_string(), file_path.clone(), self.ttl);
        Some(file_path)
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

fn cache_key_hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
