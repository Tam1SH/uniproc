use crate::ratelimit;
use anyhow::{anyhow, bail, Context};
use serde_json::{Map, Value};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{collections::BTreeSet, thread};
use tracing::{debug, error, info, instrument, trace, warn};

pub type SubscriptionId = u64;
pub type SettingsCallback = Arc<dyn Fn(&SettingEvent) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingOp {
    Set,
    Patch,
    Delete,
}

#[derive(Debug, Clone)]
pub struct SettingEvent {
    pub path: String,
    pub op: SettingOp,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionKind {
    Any,
    ExactPath(Arc<str>),
    Prefix(Arc<str>),
}

struct SubscriptionEntry {
    id: SubscriptionId,
    kind: SubscriptionKind,
    callback: SettingsCallback,
}

pub struct SettingsStore {
    path: PathBuf,
    inner: Arc<RwLock<Map<String, Value>>>,
    subscriptions: Arc<RwLock<Vec<SubscriptionEntry>>>,
    next_subscription_id: AtomicU64,
    save_tx: mpsc::Sender<()>,
}

impl Debug for SettingsStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsStore")
            .field("path", &self.path)
            .finish()
    }
}

impl SettingsStore {
    const DEFAULT_SAVE_DEBOUNCE_MS: u64 = 300;
    const DEFAULT_WATCH_INTERVAL_MS: u64 = 500;

    pub fn default_settings_path() -> PathBuf {
        if cfg!(target_os = "windows") {
            if let Ok(base) = std::env::var("APPDATA") {
                let p = PathBuf::from(base).join("Uniproc").join("settings.json");
                debug!(path = %p.display(), "resolved default settings path (Windows/APPDATA)");
                return p;
            }
        }

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let p = PathBuf::from(xdg).join("uniproc").join("settings.json");
            debug!(path = %p.display(), "resolved default settings path (XDG_CONFIG_HOME)");
            return p;
        }

        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home)
                .join(".config")
                .join("uniproc")
                .join("settings.json");
            debug!(path = %p.display(), "resolved default settings path (~/.config)");
            return p;
        }

        warn!("could not resolve any known config directory, falling back to cwd settings.json");
        PathBuf::from("settings.json")
    }

    #[instrument(fields(path = %path.display()))]
    pub fn load_or_default(path: PathBuf) -> anyhow::Result<Self> {
        let initial = if path.exists() {
            info!("loading existing settings file");
            Self::load_map(&path)?
        } else {
            info!("settings file not found, starting with empty store");
            Map::new()
        };
        Ok(Self::new(path, initial))
    }

    #[instrument(skip(self), fields(path))]
    pub fn get(&self, path: &str) -> Option<Value> {
        tracing::Span::current().record("path", path);
        let guard = self.inner.read().ok()?;
        let result = get_at_path(&guard, split_path(path)).cloned();
        trace!(found = result.is_some(), "get");
        result
    }

    #[instrument(skip(self, value), fields(path, value_type))]
    pub fn set(&self, path: &str, value: Value) -> anyhow::Result<()> {
        tracing::Span::current().record("path", path);
        tracing::Span::current().record("value_type", value_type_name(&value));
        let path_str = normalize_path(path)?;

        let old = {
            let mut guard = self
                .inner
                .write()
                .map_err(|_| anyhow!("settings lock poisoned"))?;
            set_at_path(&mut guard, split_path(&path_str), value.clone())?
        };

        debug!(old_existed = old.is_some(), "set value at path");

        self.emit(SettingEvent {
            path: path_str,
            op: SettingOp::Set,
            old,
            new: Some(value),
        });
        self.schedule_save();
        Ok(())
    }

    #[instrument(skip(self, patch), fields(path))]
    pub fn patch(&self, path: &str, patch: Value) -> anyhow::Result<()> {
        tracing::Span::current().record("path", path);
        let patch_obj = patch
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow!("patch value must be a JSON object"))?;
        let path_str = normalize_path(path)?;

        let (old, new) = {
            let mut guard = self
                .inner
                .write()
                .map_err(|_| anyhow!("settings lock poisoned"))?;
            let old = get_at_path(&guard, split_path(&path_str)).cloned();

            let target = ensure_object_at_path(&mut guard, split_path(&path_str))?;
            merge_objects(target, &patch_obj);

            let new = get_at_path(&guard, split_path(&path_str)).cloned();
            (old, new)
        };

        debug!(patch_keys = patch_obj.len(), "patched object at path");

        self.emit(SettingEvent {
            path: path_str,
            op: SettingOp::Patch,
            old,
            new,
        });
        self.schedule_save();
        Ok(())
    }

    #[instrument(skip(self), fields(path))]
    pub fn delete(&self, path: &str) -> anyhow::Result<()> {
        tracing::Span::current().record("path", path);
        let path_str = normalize_path(path)?;
        let old = {
            let mut guard = self
                .inner
                .write()
                .map_err(|_| anyhow!("settings lock poisoned"))?;
            delete_at_path(&mut guard, split_path(&path_str))?
        };

        if old.is_none() {
            debug!("delete called but path did not exist");
        } else {
            debug!("deleted value at path");
        }

        self.emit(SettingEvent {
            path: path_str,
            op: SettingOp::Delete,
            old,
            new: None,
        });
        self.schedule_save();
        Ok(())
    }

    pub fn snapshot(&self) -> Map<String, Value> {
        let snap = self.inner.read().map(|g| g.clone()).unwrap_or_else(|_| {
            error!("settings lock poisoned during snapshot, returning empty map");
            Map::new()
        });
        trace!(keys = snap.len(), "snapshot taken");
        snap
    }

    #[instrument(skip(self), fields(path = %self.path.display()))]
    pub fn save_now(&self) -> anyhow::Result<()> {
        info!("saving settings synchronously");
        let snapshot = self.snapshot();
        persist_atomic(&self.path, &snapshot)
    }

    pub fn get_u64(&self, path: &str) -> Option<u64> {
        self.get(path).and_then(|v| v.as_u64())
    }

    #[instrument(skip(self, callback), fields(kind = ?kind))]
    pub fn subscribe(&self, kind: SubscriptionKind, callback: SettingsCallback) -> SubscriptionId {
        let id = self.next_subscription_id.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut subs) = self.subscriptions.write() {
            subs.push(SubscriptionEntry { id, kind, callback });
            debug!(
                subscription_id = id,
                total = subs.len(),
                "subscription registered"
            );
        } else {
            error!(
                subscription_id = id,
                "failed to register subscription: lock poisoned"
            );
        }
        id
    }

    #[instrument(skip(self))]
    pub fn unsubscribe(&self, id: SubscriptionId) {
        if let Ok(mut subs) = self.subscriptions.write() {
            let before = subs.len();
            subs.retain(|s| s.id != id);
            let removed = before - subs.len();
            if removed == 0 {
                warn!(subscription_id = id, "unsubscribe called for unknown id");
            } else {
                debug!(
                    subscription_id = id,
                    remaining = subs.len(),
                    "subscription removed"
                );
            }
        } else {
            error!(subscription_id = id, "failed to unsubscribe: lock poisoned");
        }
    }

    pub fn on_state_changed(&self, callback: SettingsCallback) -> SubscriptionId {
        self.subscribe(SubscriptionKind::Any, callback)
    }

    pub fn on_field_changed(&self, path: Arc<str>, callback: SettingsCallback) -> SubscriptionId {
        self.subscribe(SubscriptionKind::ExactPath(path), callback)
    }

    pub fn on_subfield_changed(
        &self,
        path: Arc<str>,
        callback: SettingsCallback,
    ) -> SubscriptionId {
        self.subscribe(SubscriptionKind::Prefix(path.into()), callback)
    }

    pub(crate) fn new(path: PathBuf, initial: Map<String, Value>) -> Self {
        let save_debounce_ms = get_u64_from_map(&initial, "settings.persistence.save_debounce_ms")
            .unwrap_or(Self::DEFAULT_SAVE_DEBOUNCE_MS);
        let save_debounce = Duration::from_millis(save_debounce_ms.max(1));
        let watch_interval_ms =
            get_u64_from_map(&initial, "settings.persistence.watch_interval_ms")
                .unwrap_or(Self::DEFAULT_WATCH_INTERVAL_MS);
        let watch_interval = Duration::from_millis(watch_interval_ms.max(50));

        info!(
            path = %path.display(),
            initial_keys = initial.len(),
            save_debounce_ms,
            watch_interval_ms,
            "initializing SettingsStore"
        );

        let inner = Arc::new(RwLock::new(initial));
        let subscriptions = Arc::new(RwLock::new(Vec::<SubscriptionEntry>::new()));
        let (save_tx, save_rx) = mpsc::channel::<()>();

        let persist_path = path.clone();
        let persist_inner = Arc::clone(&inner);

        let debounce = save_debounce;
        std::thread::spawn(move || {
            let span = tracing::info_span!("settings_save_worker", path = %persist_path.display());
            let _enter = span.enter();
            debug!("save worker started");

            while save_rx.recv().is_ok() {
                let mut coalesced = 0usize;
                while save_rx.recv_timeout(debounce).is_ok() {
                    coalesced += 1;
                }
                if coalesced > 0 {
                    debug!(coalesced, "debounced rapid save requests");
                }

                let snapshot = persist_inner.read().map(|g| g.clone()).unwrap_or_else(|_| {
                    error!("settings lock poisoned in save worker, writing empty map");
                    Map::new()
                });

                debug!(keys = snapshot.len(), "persisting settings to disk");
                if let Err(err) = persist_atomic(&persist_path, &snapshot) {
                    warn!("settings save failed: {err:#}");
                } else {
                    trace!("settings saved successfully");
                }
            }

            debug!("save worker shutting down (channel closed)");
        });

        let watch_path = path.clone();
        let watch_inner = Arc::clone(&inner);
        let watch_subs = Arc::clone(&subscriptions);
        thread::spawn(move || {
            let span = tracing::info_span!("settings_watch_worker", path = %watch_path.display());
            let _enter = span.enter();
            debug!("watch worker started");

            loop {
                thread::sleep(watch_interval);

                let on_disk = if watch_path.exists() {
                    match Self::load_map(&watch_path) {
                        Ok(map) => map,
                        Err(err) => {
                            warn!("settings watch reload failed: {err:#}");
                            continue;
                        }
                    }
                } else {
                    trace!("settings file absent on watch tick, treating as empty");
                    Map::new()
                };

                let events = {
                    let mut guard = match watch_inner.write() {
                        Ok(guard) => guard,
                        Err(_) => {
                            warn!("settings watch skipped: lock poisoned");
                            continue;
                        }
                    };

                    if *guard == on_disk {
                        trace!("watch tick: no changes detected");
                        Vec::new()
                    } else {
                        info!("external settings change detected, computing diff");
                        let old = guard.clone();
                        *guard = on_disk.clone();
                        diff_settings_maps(&old, &on_disk)
                    }
                };

                if !events.is_empty() {
                    debug!(
                        event_count = events.len(),
                        "emitting events for external changes"
                    );
                    emit_events(&watch_subs, events);
                }
            }
        });

        Self {
            path,
            inner,
            subscriptions,
            next_subscription_id: AtomicU64::new(1),
            save_tx,
        }
    }

    #[instrument(fields(path = %path.display()))]
    fn load_map(path: &Path) -> anyhow::Result<Map<String, Value>> {
        ratelimit!(3600, debug!("reading settings file from disk"));
        let raw = std::fs::read(path)
            .with_context(|| format!("failed to read settings file: {}", path.display()))?;
        trace!(bytes = raw.len(), "read settings file");
        let value: Value = serde_json::from_slice(&raw)
            .with_context(|| format!("failed to parse settings file: {}", path.display()))?;
        match value {
            Value::Object(map) => {
                ratelimit!(3600, debug!(keys = map.len(), "settings file loaded"));
                Ok(map)
            }
            _ => bail!("settings root must be a JSON object: {}", path.display()),
        }
    }

    fn schedule_save(&self) {
        trace!("scheduling debounced save");
        let _ = self.save_tx.send(());
    }

    fn emit(&self, event: SettingEvent) {
        let callbacks = self
            .subscriptions
            .read()
            .map(|subs| {
                subs.iter()
                    .filter(|sub| matches_subscription(&sub.kind, &event.path))
                    .map(|sub| Arc::clone(&sub.callback))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|_| {
                error!(
                    "subscriptions lock poisoned, skipping callbacks for path={}",
                    event.path
                );
                Vec::new()
            });

        trace!(
            path = %event.path,
            op = ?event.op,
            callbacks = callbacks.len(),
            "emitting setting event"
        );

        for cb in callbacks {
            cb(&event);
        }
    }
}

fn diff_settings_maps(old: &Map<String, Value>, new: &Map<String, Value>) -> Vec<SettingEvent> {
    let mut events = Vec::new();
    let keys = old
        .keys()
        .chain(new.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for key in keys {
        collect_value_diff(old.get(&key), new.get(&key), key.as_str(), &mut events);
    }

    trace!(diff_events = events.len(), "diff computed");
    events
}

fn collect_value_diff(
    old: Option<&Value>,
    new: Option<&Value>,
    path: &str,
    events: &mut Vec<SettingEvent>,
) {
    match (old, new) {
        (None, None) => {}
        (Some(ov), Some(nv)) if ov == nv => {}
        (Some(Value::Object(om)), Some(Value::Object(nm))) => {
            let keys = om.keys().chain(nm.keys()).cloned().collect::<BTreeSet<_>>();
            for key in keys {
                let child = join_path(path, &key);
                collect_value_diff(om.get(&key), nm.get(&key), &child, events);
            }
        }
        (None, Some(nv)) => {
            trace!(path, "diff: new key added");
            events.push(SettingEvent {
                path: path.to_string(),
                op: SettingOp::Set,
                old: None,
                new: Some(nv.clone()),
            })
        }
        (Some(ov), None) => {
            trace!(path, "diff: key removed");
            events.push(SettingEvent {
                path: path.to_string(),
                op: SettingOp::Delete,
                old: Some(ov.clone()),
                new: None,
            })
        }
        (Some(ov), Some(nv)) => {
            trace!(
                path,
                old_type = value_type_name(ov),
                new_type = value_type_name(nv),
                "diff: value changed"
            );
            events.push(SettingEvent {
                path: path.to_string(),
                op: SettingOp::Set,
                old: Some(ov.clone()),
                new: Some(nv.clone()),
            })
        }
    }
}

fn join_path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

fn emit_events(subscriptions: &Arc<RwLock<Vec<SubscriptionEntry>>>, events: Vec<SettingEvent>) {
    for event in events {
        let callbacks = subscriptions
            .read()
            .map(|subs| {
                subs.iter()
                    .filter(|sub| matches_subscription(&sub.kind, &event.path))
                    .map(|sub| sub.callback.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|_| {
                error!(
                    "subscriptions lock poisoned in emit_events for path={}",
                    event.path
                );
                Vec::new()
            });

        debug!(
            path = %event.path,
            op = ?event.op,
            callbacks = callbacks.len(),
            "dispatching event to subscribers"
        );

        for cb in callbacks {
            cb(&event);
        }
    }
}

fn normalize_path(path: &str) -> anyhow::Result<String> {
    let normalized = path
        .split('.')
        .filter(|seg| !seg.trim().is_empty())
        .collect::<Vec<_>>()
        .join(".");
    if normalized.is_empty() {
        bail!("path cannot be empty");
    }
    Ok(normalized)
}

fn split_path(path: &str) -> Vec<&str> {
    path.split('.').filter(|s| !s.is_empty()).collect()
}

fn matches_subscription(kind: &SubscriptionKind, changed_path: &str) -> bool {
    match kind {
        SubscriptionKind::Any => true,
        SubscriptionKind::ExactPath(path) => **path == *changed_path,
        SubscriptionKind::Prefix(prefix) => {
            *changed_path == **prefix
                || changed_path
                    .strip_prefix(&**prefix)
                    .is_some_and(|tail| tail.starts_with('.'))
        }
    }
}

fn get_at_path<'a>(map: &'a Map<String, Value>, parts: Vec<&str>) -> Option<&'a Value> {
    let mut iter = parts.into_iter();
    let first = iter.next()?;
    let mut current = map.get(first)?;
    for key in iter {
        let obj = current.as_object()?;
        current = obj.get(key)?;
    }
    Some(current)
}

fn set_at_path(
    map: &mut Map<String, Value>,
    parts: Vec<&str>,
    value: Value,
) -> anyhow::Result<Option<Value>> {
    let mut iter = parts.into_iter().peekable();
    let first = iter.next().ok_or_else(|| anyhow!("path cannot be empty"))?;

    if iter.peek().is_none() {
        return Ok(map.insert(first.to_string(), value));
    }

    let mut current = map
        .entry(first.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    while let Some(key) = iter.next() {
        if iter.peek().is_none() {
            let obj = current
                .as_object_mut()
                .ok_or_else(|| anyhow!("path segment '{}' is not an object", key))?;
            return Ok(obj.insert(key.to_string(), value));
        }

        let obj = current
            .as_object_mut()
            .ok_or_else(|| anyhow!("path segment '{}' is not an object", key))?;
        current = obj
            .entry(key.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }

    bail!("invalid path")
}

fn ensure_object_at_path<'a>(
    map: &'a mut Map<String, Value>,
    parts: Vec<&str>,
) -> anyhow::Result<&'a mut Map<String, Value>> {
    let mut iter = parts.into_iter();
    let first = iter.next().ok_or_else(|| anyhow!("path cannot be empty"))?;

    let mut current = map
        .entry(first.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    for key in iter {
        let obj = current
            .as_object_mut()
            .ok_or_else(|| anyhow!("path segment '{}' is not an object", key))?;
        current = obj
            .entry(key.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }

    current
        .as_object_mut()
        .ok_or_else(|| anyhow!("target path is not an object"))
}

fn delete_at_path(map: &mut Map<String, Value>, parts: Vec<&str>) -> anyhow::Result<Option<Value>> {
    let mut iter = parts.into_iter().peekable();
    let first = iter.next().ok_or_else(|| anyhow!("path cannot be empty"))?;

    if iter.peek().is_none() {
        return Ok(map.remove(first));
    }

    let mut current = map
        .get_mut(first)
        .ok_or_else(|| anyhow!("path not found: '{}'", first))?;

    while let Some(key) = iter.next() {
        if iter.peek().is_none() {
            let obj = current
                .as_object_mut()
                .ok_or_else(|| anyhow!("path segment '{}' is not an object", key))?;
            return Ok(obj.remove(key));
        }

        let obj = current
            .as_object_mut()
            .ok_or_else(|| anyhow!("path segment '{}' is not an object", key))?;
        current = obj
            .get_mut(key)
            .ok_or_else(|| anyhow!("path not found: '{}'", key))?;
    }

    bail!("invalid path")
}

fn merge_objects(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (k, v) in patch {
        match (target.get_mut(k), v) {
            (Some(Value::Object(target_obj)), Value::Object(patch_obj)) => {
                merge_objects(target_obj, patch_obj)
            }
            _ => {
                target.insert(k.clone(), v.clone());
            }
        }
    }
}

fn get_u64_from_map(map: &Map<String, Value>, path: &str) -> Option<u64> {
    get_at_path(map, split_path(path)).and_then(|v| v.as_u64())
}

#[instrument(fields(path = %path.display()))]
fn persist_atomic(path: &Path, map: &Map<String, Value>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create settings directory: {}",
                parent.to_string_lossy()
            )
        })?;
    }

    let tmp_path = path.with_extension("tmp");
    let data = serde_json::to_vec_pretty(&Value::Object(map.clone()))
        .context("failed to serialize settings")?;

    trace!(bytes = data.len(), tmp = %tmp_path.display(), "writing temp file");
    std::fs::write(&tmp_path, data)
        .with_context(|| format!("failed to write temp settings file: {}", tmp_path.display()))?;

    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to replace settings file: {}", path.display()))?;
    }

    std::fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to persist settings file: {}", path.display()))?;

    debug!("settings atomically persisted");
    Ok(())
}

/// Returns a short static name for a JSON value kind — used as a tracing field.
fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_path(suffix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("uniproc-settings-{suffix}-{nanos}.json"))
    }

    #[test]
    fn set_get_delete_roundtrip() {
        let path = std::env::temp_dir().join("uniproc-settings-test-roundtrip.json");
        let store = SettingsStore::new(path, Map::new());

        store
            .set("ui.theme.dark", Value::Bool(true))
            .expect("set should succeed");
        assert_eq!(store.get("ui.theme.dark"), Some(Value::Bool(true)));

        store
            .delete("ui.theme.dark")
            .expect("delete should succeed");
        assert_eq!(store.get("ui.theme.dark"), None);
    }

    #[test]
    fn patch_merges_objects() {
        let path = std::env::temp_dir().join("uniproc-settings-test-patch.json");
        let store = SettingsStore::new(path, Map::new());

        store
            .set(
                "process.columns",
                serde_json::json!({"cpu":{"width":70}, "memory":{"width":120}}),
            )
            .expect("set should succeed");

        store
            .patch(
                "process.columns",
                serde_json::json!({"memory":{"width":140}}),
            )
            .expect("patch should succeed");

        assert_eq!(
            store.get("process.columns.cpu.width"),
            Some(Value::Number(70.into()))
        );
        assert_eq!(
            store.get("process.columns.memory.width"),
            Some(Value::Number(140.into()))
        );
    }

    #[test]
    fn subscriptions_any_exact_prefix_fire() {
        let path = std::env::temp_dir().join("uniproc-settings-test-subscriptions.json");
        let store = SettingsStore::new(path, Map::new());

        let any_hits = Arc::new(Mutex::new(Vec::<String>::new()));
        let exact_hits = Arc::new(Mutex::new(Vec::<String>::new()));
        let prefix_hits = Arc::new(Mutex::new(Vec::<String>::new()));

        let any_capture = Arc::clone(&any_hits);
        store.on_state_changed(Arc::new(move |evt| {
            any_capture
                .lock()
                .expect("mutex should not be poisoned")
                .push(evt.path.clone());
        }));

        let exact_capture = Arc::clone(&exact_hits);
        store.on_field_changed(
            Arc::from("ui.theme.dark"),
            Arc::new(move |evt| {
                exact_capture
                    .lock()
                    .expect("mutex should not be poisoned")
                    .push(evt.path.clone());
            }),
        );

        let prefix_capture = Arc::clone(&prefix_hits);
        store.on_subfield_changed(
            Arc::from("ui.theme"),
            Arc::new(move |evt| {
                prefix_capture
                    .lock()
                    .expect("mutex should not be poisoned")
                    .push(evt.path.clone());
            }),
        );

        store
            .set("ui.theme.dark", Value::Bool(true))
            .expect("set should succeed");
        store
            .set("ui.layout.sidebar_width", Value::Number(260.into()))
            .expect("set should succeed");

        assert_eq!(
            any_hits.lock().expect("mutex should not be poisoned").len(),
            2
        );
        assert_eq!(
            exact_hits
                .lock()
                .expect("mutex should not be poisoned")
                .as_slice(),
            ["ui.theme.dark"]
        );
        assert_eq!(
            prefix_hits
                .lock()
                .expect("mutex should not be poisoned")
                .as_slice(),
            ["ui.theme.dark"]
        );
    }

    #[test]
    fn unsubscribe_stops_callbacks() {
        let path = std::env::temp_dir().join("uniproc-settings-test-unsubscribe.json");
        let store = SettingsStore::new(path, Map::new());

        let hit_count = Arc::new(Mutex::new(0usize));
        let counter_capture = Arc::clone(&hit_count);
        let id = store.on_state_changed(Arc::new(move |_| {
            let mut guard = counter_capture
                .lock()
                .expect("mutex should not be poisoned");
            *guard += 1;
        }));

        store
            .set("ui.theme.dark", Value::Bool(true))
            .expect("set should succeed");
        store.unsubscribe(id);
        store
            .set("ui.theme.dark", Value::Bool(false))
            .expect("set should succeed");

        assert_eq!(*hit_count.lock().expect("mutex should not be poisoned"), 1);
    }

    #[test]
    fn file_watch_emits_set_for_external_change() {
        let path = unique_test_path("watch-set");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "settings": { "persistence": { "watch_interval_ms": 50 } },
                "ui": { "theme": { "dark": false } }
            }))
            .expect("json serialization should succeed"),
        )
        .expect("seed settings file should be written");

        let store = SettingsStore::load_or_default(path.clone()).expect("store should load");
        let (tx, rx) = mpsc::channel::<SettingEvent>();

        store.on_field_changed(
            Arc::from("ui.theme.dark"),
            Arc::new(move |evt| {
                let _ = tx.send(evt.clone());
            }),
        );

        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "settings": { "persistence": { "watch_interval_ms": 50 } },
                "ui": { "theme": { "dark": true } }
            }))
            .expect("json serialization should succeed"),
        )
        .expect("updated settings file should be written");

        let event = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("watcher should emit set event");
        assert_eq!(event.path, "ui.theme.dark");
        assert_eq!(event.op, SettingOp::Set);
        assert_eq!(event.old, Some(Value::Bool(false)));
        assert_eq!(event.new, Some(Value::Bool(true)));
    }

    #[test]
    fn file_watch_emits_delete_for_external_removal() {
        let path = unique_test_path("watch-delete");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "settings": { "persistence": { "watch_interval_ms": 50 } },
                "ui": { "theme": { "dark": true } }
            }))
            .expect("json serialization should succeed"),
        )
        .expect("seed settings file should be written");

        let store = SettingsStore::load_or_default(path.clone()).expect("store should load");
        let (tx, rx) = mpsc::channel::<SettingEvent>();

        store.on_field_changed(
            Arc::from("ui.theme.dark"),
            Arc::new(move |evt| {
                let _ = tx.send(evt.clone());
            }),
        );

        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "settings": { "persistence": { "watch_interval_ms": 50 } },
                "ui": { "theme": {} }
            }))
            .expect("json serialization should succeed"),
        )
        .expect("updated settings file should be written");

        let event = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("watcher should emit delete event");
        assert_eq!(event.path, "ui.theme.dark");
        assert_eq!(event.op, SettingOp::Delete);
        assert_eq!(event.old, Some(Value::Bool(true)));
        assert_eq!(event.new, None);
    }
}
