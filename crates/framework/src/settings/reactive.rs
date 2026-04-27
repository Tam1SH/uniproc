use crate::settings::SettingsStore;
use app_core::actor::event_bus::subscribe::SubscriptionId;
use app_core::signal::{Signal, SignalSubscription};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::info;

pub struct SettingSubscription {
    pub settings: Arc<SettingsStore>,
    pub id: SubscriptionId,
}
pub struct ReactiveSettingSubscription {
    #[allow(unused)]
    pub signal_sub: SignalSubscription,
}

#[derive(Clone)]
pub struct ReactiveSetting<TValue> {
    pub signal: Arc<Signal<TValue>>,
    pub path: Arc<str>,
    pub _store_subscription: Arc<SettingSubscription>,
}

impl<TValue> ReactiveSetting<TValue>
where
    TValue: DeserializeOwned + Serialize + Send + Sync + Clone + 'static,
{
    pub fn get(&self) -> TValue {
        self.signal.get()
    }

    pub fn get_store_subscription(&self) -> Arc<SettingSubscription> {
        self._store_subscription.clone()
    }

    pub fn get_path(&self) -> Arc<str> {
        self.path.clone()
    }

    pub fn get_arc(&self) -> Arc<TValue> {
        self.signal.get_arc()
    }

    pub fn subscribe<F>(&self, callback: F) -> ReactiveSettingSubscription
    where
        F: Fn(TValue) + Send + Sync + 'static,
    {
        let path = self.path.clone();

        let signal_sub = self.signal.subscribe(move |val: &TValue| {
            info!(
                "event, path: {}, val: {:?}",
                path,
                serde_json::to_value(val).unwrap()
            );
            callback(val.clone());
        });

        ReactiveSettingSubscription { signal_sub }
    }

    pub fn set(&self, value: TValue) -> anyhow::Result<()> {
        let json = serde_json::to_value(value)?;
        self._store_subscription.settings.set(&self.path, json)
    }

    pub fn as_signal(&self) -> Arc<Signal<TValue>> {
        self.signal.clone()
    }
}

impl<TValue: Debug + 'static> Debug for ReactiveSetting<TValue> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReactiveSetting")
            .field("signal_value", self.signal.get_arc().as_ref())
            .finish()
    }
}

impl Drop for SettingSubscription {
    fn drop(&mut self) {
        self.settings.unsubscribe(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingsStore;
    use serde_json::{json, Map};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    fn create_temp_store() -> Arc<SettingsStore> {
        let temp_path =
            std::env::temp_dir().join(format!("test_settings_{}.json", uuid::Uuid::new_v4()));
        Arc::new(SettingsStore::new(temp_path, Map::new()))
    }

    #[test]
    fn test_signal_basic_flow() {
        let signal = Signal::new(10);
        assert_eq!(signal.get(), 10);

        let received = Arc::new(Mutex::new(0));
        let r_clone = received.clone();

        let _sub = signal.subscribe(move |val| {
            *r_clone.lock().unwrap() = *val;
        });

        signal.set(20);
        assert_eq!(signal.get(), 20);
        assert_eq!(*received.lock().unwrap(), 20);

        signal.store_arc(Arc::new(30));
        assert_eq!(signal.get_arc().as_ref(), &30);
        assert_eq!(*received.lock().unwrap(), 30);
    }

    #[test]
    fn test_signal_subscription_cleanup() {
        let signal = Signal::new("a".to_string());
        let counter = Arc::new(Mutex::new(0));

        {
            let c = counter.clone();
            let _sub = signal.subscribe(move |_| {
                *c.lock().unwrap() += 1;
            });
            signal.set("b".to_string());
            assert_eq!(*counter.lock().unwrap(), 1);
        }

        signal.set("c".to_string());

        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn test_reactive_setting_full_integration() {
        let store = create_temp_store();
        let path = "ui.font_size";

        let signal = Arc::new(Signal::new(14));
        let store_sub = Arc::new(SettingSubscription {
            settings: store.clone(),
            id: 1,
        });

        let reactive = ReactiveSetting {
            signal: signal.clone(),
            path: Arc::from(path),
            _store_subscription: store_sub,
        };

        assert_eq!(reactive.get(), 14);
        assert_eq!(reactive.get_path().as_ref(), path);

        reactive.set(18).expect("Failed to set value");

        let store_val = store.get(path).expect("Value not found in store");
        assert_eq!(store_val, json!(18));

        let callback_val = Arc::new(Mutex::new(0));
        let cv = callback_val.clone();
        let _reactive_sub = reactive.subscribe(move |v| {
            *cv.lock().unwrap() = v;
        });

        signal.set(22);
        assert_eq!(*callback_val.lock().unwrap(), 22);
    }

    #[test]
    fn test_reactive_setting_debug_output() {
        let signal = Arc::new(Signal::new("test_val".to_string()));
        let store = create_temp_store();
        let store_sub = Arc::new(SettingSubscription {
            settings: store,
            id: 99,
        });

        let reactive = ReactiveSetting {
            signal,
            path: Arc::from("debug.path"),
            _store_subscription: store_sub,
        };

        let debug_str = format!("{:?}", reactive);
        assert!(debug_str.contains("ReactiveSetting"));
        assert!(debug_str.contains("test_val"));
    }

    #[test]
    fn test_setting_subscription_drop_really_unsubscribes() {
        let store = create_temp_store();
        let path = "test.field";

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();

        let sub_id = store.on_field_changed(
            Arc::from(path),
            Arc::new(move |_| {
                calls_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        {
            let _sub = SettingSubscription {
                settings: store.clone(),
                id: sub_id,
            };

            store.set(path, json!("hello")).unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }

        store.set(path, json!("world")).unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_as_signal() {
        let signal = Arc::new(Signal::new(100));
        let store = create_temp_store();
        let reactive = ReactiveSetting {
            signal: signal.clone(),
            path: Arc::from("path"),
            _store_subscription: Arc::new(SettingSubscription {
                settings: store,
                id: 1,
            }),
        };

        let extracted_signal = reactive.as_signal();
        assert!(Arc::ptr_eq(&signal, &extracted_signal));
    }
}
