mod encode;
mod extract;

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;
use ttl_cache::TtlCache;

pub use extract::has_own_icon;

const MISS_TTL: Duration = Duration::from_secs(300);

pub struct IconRequest<'a> {
    pub path: &'a str,
    pub package_full_name: Option<&'a str>,
}

// TODO: this round-trips every icon through the filesystem - extract to RGBA,
// encode a PNG into %TEMP%, hand the path to `Image::new_with_uri`, let WinUI
// read it back. The pixels are already in memory; the disk hop exists only
// because windows-reactor's `Image` takes a URI and nothing else.
//
// Fixing it properly is a windows-reactor change: an `Image` that accepts raw
// RGBA (WriteableBitmap / SoftwareBitmapSource underneath). Until that lands,
// the cache below is the workaround, not the design.
pub struct IconCache {
    cache: RefCell<TtlCache<String, Option<PathBuf>>>,
    cache_dir: PathBuf,
    ttl: Duration,
    miss_ttl: Duration,
}

impl IconCache {
    pub fn new() -> Self {
        Self::with_cache_dir(std::env::temp_dir().join("uniproc-icons"), Duration::from_secs(3600))
    }

    pub fn with_cache_dir(cache_dir: PathBuf, ttl: Duration) -> Self {
        if let Err(err) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!(?err, ?cache_dir, "context: failed to create icon cache dir");
        }
        Self {
            cache: RefCell::new(TtlCache::new(512)),
            cache_dir,
            ttl,
            miss_ttl: MISS_TTL,
        }
    }

    fn remember_miss(&self, key: &str) -> Option<PathBuf> {
        self.cache
            .borrow_mut()
            .insert(key.to_string(), None, self.miss_ttl);
        None
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
            return cached.clone();
        }

        let file_path = self.cache_dir.join(format!("{:x}.png", cache_key_hash(key)));
        if file_path.is_file() {
            self.cache
                .borrow_mut()
                .insert(key.to_string(), Some(file_path.clone()), self.ttl);
            return Some(file_path);
        }

        let image = match req.package_full_name.filter(|s| !s.is_empty()) {
            Some(pkg) => extract::extract_appx_icon_rgba(pkg, 32),
            None => extract::extract_icon_rgba(req.path),
        };
        let Some(image) = image else {
            return self.remember_miss(key);
        };

        if let Err(err) = encode::write_png(&file_path, image.width, image.height, &image.pixels) {
            tracing::warn!(?err, key, "context: failed to write cached icon png");
            return self.remember_miss(key);
        }

        self.cache
            .borrow_mut()
            .insert(key.to_string(), Some(file_path.clone()), self.ttl);
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
