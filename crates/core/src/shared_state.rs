use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

type Service = Arc<dyn Any + Send + Sync>;

#[derive(Default)]
pub struct SharedState {
    inner: RwLock<HashMap<TypeId, Service>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert<T>(&self, value: T) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        let mut map = self.inner.write().ok()?;
        let previous = map.insert(id, Arc::new(value));
        previous.and_then(|svc| svc.downcast::<T>().ok())
    }

    pub fn insert_arc<T>(&self, value: Arc<T>) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        let mut map = self.inner.write().ok()?;
        let previous = map.insert(id, value);
        previous.and_then(|svc| svc.downcast::<T>().ok())
    }

    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        let map = self.inner.read().ok()?;
        map.get(&id)?.clone().downcast::<T>().ok()
    }

    pub fn remove<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        let mut map = self.inner.write().ok()?;
        map.remove(&id)?.downcast::<T>().ok()
    }

    pub fn contains<T>(&self) -> bool
    where
        T: Send + Sync + 'static,
    {
        let id = TypeId::of::<T>();
        self.inner
            .read()
            .map(|m| m.contains_key(&id))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::SharedState;
    use std::sync::Arc;

    #[test]
    fn insert_get_remove_roundtrip() {
        let shared = SharedState::new();

        assert!(!shared.contains::<String>());
        assert!(shared.insert("hello".to_string()).is_none());
        assert!(shared.contains::<String>());
        assert_eq!(
            shared.get::<String>().as_ref().map(|s| s.as_str()),
            Some("hello")
        );

        let removed = shared.remove::<String>();
        assert_eq!(removed.as_ref().map(|s| s.as_str()), Some("hello"));
        assert!(shared.get::<String>().is_none());
    }

    #[test]
    fn overwrite_returns_previous() {
        let shared = SharedState::new();
        shared.insert(7u32);
        let prev = shared.insert(9u32);
        assert_eq!(prev.map(|v| *v), Some(7u32));
        assert_eq!(shared.get::<u32>().map(|v| *v), Some(9u32));
    }

    #[test]
    fn insert_arc_keeps_identity() {
        let shared = SharedState::new();
        let service = Arc::new(42usize);
        let ptr = Arc::as_ptr(&service);
        shared.insert_arc(Arc::clone(&service));

        let fetched = shared.get::<usize>().expect("service should exist");
        assert_eq!(Arc::as_ptr(&fetched), ptr);
    }
}
