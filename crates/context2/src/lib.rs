//! Process/app icon extraction, cached to small `.png` files on disk.
//!
//! Deliberately UI-framework-agnostic: this crate hands back a [`PathBuf`]
//! to a cached bitmap, not any particular widget's image type. The old
//! `context` crate (`crates/context`, part of the retired Slint stack)
//! baked `slint::Image` construction directly into the extraction path,
//! which meant every consumer - and every future frontend - inherited that
//! dependency. Callers turn the path into whatever their renderer wants,
//! e.g. `windows_reactor::Image::new_with_uri(format!("file:///{path}"))`.

mod encode;
mod extract;

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;
use ttl_cache::TtlCache;

pub use extract::has_own_icon;

/// What to resolve an icon for.
///
/// Packaged apps (UWP/MSIX) resolve through the Shell `AppsFolder` via
/// `package_full_name` and need no file path. Regular executables need
/// `path` to point at the real `.exe` - a bare process name (`"chrome.exe"`)
/// is not enough; `SHGetFileInfoW` needs a resolvable location.
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

    /// Resolves `req` to a cached `.png` path, extracting and writing it on
    /// first use. `None` means extraction failed, or the request carried no
    /// usable path/package - callers should fall back to a default icon
    /// rather than treat this as an error.
    ///
    /// The disk file itself isn't optional - `windows_reactor::Image` only
    /// loads from a URI (no `from_bytes`/`from_pixels` in this fork), so
    /// extracted pixels have to be materialized as a real file for WinUI to
    /// read at all. What actually avoids repeat work within one run is the
    /// in-memory `TtlCache` checked first below, which skips re-running the
    /// expensive GDI/Shell extraction (`SHGetFileInfoW`+`GetDIBits`, or for
    /// packaged apps a real `IShellItemImageFactory` round trip) on every
    /// tick for the same exe.
    ///
    /// The disk file's *name* is a deterministic hash of `key`, so it also
    /// survives past this run - a subsequent process launch can find it
    /// already there and skip extraction entirely rather than starting
    /// from an empty in-memory cache and re-extracting everything. There's
    /// no invalidation on that path, though: if an exe's icon actually
    /// changes (an update replaces it), the stale cached file keeps being
    /// served until something clears `%TEMP%/uniproc-icons` by hand.
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

/// Cheap, stable-enough-for-a-cache-filename hash. A collision just means
/// two keys briefly share a cache slot until the TTL evicts one - not a
/// correctness problem, so `DefaultHasher` (not a cryptographic hash) is
/// the right tool here.
fn cache_key_hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
