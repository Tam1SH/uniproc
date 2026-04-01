pub mod reactive;
pub mod store;

use std::sync::Arc;

use app_core::signal::Signal;
use app_core::SharedState;
pub use reactive::{ReactiveSetting, SettingSubscription};
use serde::de::DeserializeOwned;
use serde::Serialize;
pub use store::SettingEvent;
pub use store::SettingOp;
pub use store::SettingsCallback;
pub use store::SettingsStore;
pub use store::SubscriptionKind;
use tracing::error;

pub trait SettingsScope {
    const PREFIX: &'static str;
}

pub trait FeatureSettings: SettingsScope {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()>;

    fn ensure_default<TValue>(
        settings: &SettingsStore,
        key: &str,
        value: TValue,
    ) -> anyhow::Result<()>
    where
        Self: Sized,
        TValue: Serialize,
    {
        ensure_default::<Self, TValue>(settings, key, value)
    }

    fn setting_or<TValue>(
        settings: &Arc<SettingsStore>,
        key: &str,
        default: TValue,
    ) -> anyhow::Result<ReactiveSetting<TValue>>
    where
        Self: Sized,
        TValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    {
        setting_or::<Self, TValue>(settings, key, default)
    }
}

pub fn scoped_path<T: SettingsScope>(key: &str) -> String {
    if key.trim().is_empty() {
        return T::PREFIX.to_string();
    }

    format!(
        "{}.{}",
        T::PREFIX.trim_end_matches('.'),
        key.trim_start_matches('.')
    )
}

pub fn ensure_default<TScope, TValue>(
    settings: &SettingsStore,
    key: &str,
    value: TValue,
) -> anyhow::Result<()>
where
    TScope: SettingsScope,
    TValue: Serialize,
{
    let path = scoped_path::<TScope>(key);

    if settings.get(&path).is_none() {
        settings.set(&path, serde_json::to_value(value)?)?;
    }

    Ok(())
}

fn read_value_or_default<TValue>(settings: &SettingsStore, path: &str, default: &TValue) -> TValue
where
    TValue: DeserializeOwned + Clone,
{
    settings
        .get(path)
        .and_then(|value| serde_json::from_value::<TValue>(value).ok())
        .unwrap_or_else(|| default.clone())
}

pub fn setting_or<TScope, TValue>(
    settings: &Arc<SettingsStore>,
    key: &str,
    default: TValue,
) -> anyhow::Result<ReactiveSetting<TValue>>
where
    TScope: SettingsScope,
    TValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    ensure_default::<TScope, TValue>(settings, key, default.clone())?;
    let path: Arc<str> = scoped_path::<TScope>(key).into();
    let current = read_value_or_default(settings, &path, &default);

    let signal = Arc::new(Signal::new(current));

    let sig_clone = Arc::clone(&signal);
    let path_clone = path.clone();

    let store_sub_id = settings.subscribe(
        SubscriptionKind::ExactPath(path.clone()),
        Arc::new(move |event| {
            if let Some(new_val) = &event.new {
                match serde_json::from_value::<TValue>(new_val.clone()) {
                    Ok(parsed) => sig_clone.set(parsed),
                    Err(e) => {
                        error!(target: "settings", path = %path_clone, error = %e, "Failed to deserialize");
                    }
                }
            }
        })
    );

    Ok(ReactiveSetting {
        signal,
        path,
        _store_subscription: Arc::new(SettingSubscription {
            settings: Arc::clone(settings),
            id: store_sub_id,
        }),
    })
}

pub fn settings_from(shared: &SharedState) -> Arc<SettingsStore> {
    shared
        .get::<SettingsStore>()
        .expect("SettingsStore must be installed in SharedState before usage")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::tempdir;

    struct TestSettings;

    impl SettingsScope for TestSettings {
        const PREFIX: &'static str = "test";
    }

    impl FeatureSettings for TestSettings {
        fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn create_test_store() -> Arc<SettingsStore> {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("settings.json");
        Arc::new(SettingsStore::new(storage_path, Default::default()))
    }

    #[test]
    fn test_subscription_triggered_on_change() {
        let settings = create_test_store();
        let call_count = Arc::new(AtomicUsize::new(0));

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "test_key", "initial".to_string()).unwrap();

        let call_count_clone = call_count.clone();

        let _subscription = reactive.subscribe(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        settings
            .set("test.test_key", serde_json::to_value("changed").unwrap())
            .unwrap();

        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_subscription_removed_on_drop() {
        let settings = create_test_store();
        let call_count = Arc::new(AtomicUsize::new(0));

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "test_key2", "initial".to_string()).unwrap();

        let call_count_clone = call_count.clone();

        let subscription = reactive.subscribe(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        settings
            .set("test.test_key2", serde_json::to_value("changed1").unwrap())
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(call_count.load(Ordering::Relaxed), 1);

        drop(subscription);

        settings
            .set("test.test_key2", serde_json::to_value("changed2").unwrap())
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_multiple_subscriptions() {
        let settings = create_test_store();
        let call_count1 = Arc::new(AtomicUsize::new(0));
        let call_count2 = Arc::new(AtomicUsize::new(0));

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "test_key3", "initial".to_string()).unwrap();

        let call_count1_clone = call_count1.clone();
        let call_count2_clone = call_count2.clone();

        let _subscription1 = reactive.subscribe(Box::new(move |_| {
            call_count1_clone.fetch_add(1, Ordering::Relaxed);
        }));

        let _subscription2 = reactive.subscribe(Box::new(move |_| {
            call_count2_clone.fetch_add(1, Ordering::Relaxed);
        }));

        settings
            .set("test.test_key3", serde_json::to_value("changed").unwrap())
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count1.load(Ordering::Relaxed), 1);
        assert_eq!(call_count2.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_subscription_not_triggered_for_different_path() {
        let settings = create_test_store();
        let call_count = Arc::new(AtomicUsize::new(0));

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "specific_key", "initial".to_string())
                .unwrap();

        let call_count_clone = call_count.clone();

        let _subscription = reactive.subscribe(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        settings
            .set(
                "test.different_key",
                serde_json::to_value("changed").unwrap(),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 0);

        settings
            .set(
                "test.specific_key",
                serde_json::to_value("changed").unwrap(),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_subscription_with_captured_data() {
        let settings = create_test_store();

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "capture_key", "initial".to_string()).unwrap();

        let captured_value = Arc::new(AtomicUsize::new(0));
        let captured_clone = captured_value.clone();

        let _subscription = reactive.subscribe(Box::new(move |_| {
            captured_clone.fetch_add(1, Ordering::Relaxed);
        }));

        settings
            .set("test.capture_key", serde_json::to_value("updated").unwrap())
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(captured_value.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_reactive_setting_value_updates() {
        let settings = create_test_store();

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "update_key", "initial".to_string()).unwrap();

        assert_eq!(reactive.get(), "initial");

        settings
            .set("test.update_key", serde_json::to_value("updated").unwrap())
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(reactive.get(), "updated");

        let arc_value = reactive.get_arc();
        assert_eq!(*arc_value, "updated".to_string());
    }

    #[test]
    fn test_subscription_clone_behavior() {
        let settings = create_test_store();
        let call_count = Arc::new(AtomicUsize::new(0));

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "clone_key", "initial".to_string()).unwrap();

        let call_count_clone = call_count.clone();

        let _subscription = reactive.subscribe(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        let reactive_clone = reactive.clone();

        settings
            .set("test.clone_key", serde_json::to_value("changed").unwrap())
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 1);

        settings
            .set(
                "test.clone_key",
                serde_json::to_value("changed_again").unwrap(),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_subscription_with_multiple_values() {
        let settings = create_test_store();
        let values_seen = Arc::new(std::sync::Mutex::new(Vec::new()));

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "multi_key", "initial".to_string()).unwrap();

        let values_seen_clone = values_seen.clone();

        let _subscription = reactive.subscribe(Box::new(move |e| {
            values_seen_clone.lock().unwrap().push(e);
        }));

        let changes = vec!["value1", "value2", "value3"];
        for change in changes {
            settings
                .set("test.multi_key", serde_json::to_value(change).unwrap())
                .unwrap();
            std::thread::sleep(Duration::from_millis(50));
        }

        let seen = values_seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0], "value1");
        assert_eq!(seen[1], "value2");
        assert_eq!(seen[2], "value3");
    }
    #[test]
    fn test_subscription_doesnt_leak_memory() {
        let settings = create_test_store();

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "leak_test", "initial".to_string()).unwrap();

        for i in 0..100 {
            let call_count = Arc::new(AtomicUsize::new(0));
            let call_count_clone = call_count.clone();

            let subscription = reactive.subscribe(Box::new(move |_| {
                call_count_clone.fetch_add(1, Ordering::Relaxed);
            }));

            settings
                .set(
                    "test.leak_test",
                    serde_json::to_value(format!("value_{}", i)).unwrap(),
                )
                .unwrap();
            std::thread::sleep(Duration::from_millis(10));

            assert_eq!(call_count.load(Ordering::Relaxed), 1);

            drop(subscription);

            settings
                .set(
                    "test.leak_test",
                    serde_json::to_value(format!("value_{}_again", i)).unwrap(),
                )
                .unwrap();
            std::thread::sleep(Duration::from_millis(10));

            assert_eq!(call_count.load(Ordering::Relaxed), 1);
        }
    }

    #[test]
    fn test_subscription_after_reactive_clone_drop() {
        let settings = create_test_store();
        let call_count = Arc::new(AtomicUsize::new(0));

        let reactive: ReactiveSetting<String> =
            setting_or::<TestSettings, _>(&settings, "clone_drop_test", "initial".to_string())
                .unwrap();

        let call_count_clone = call_count.clone();

        let subscription = reactive.subscribe(Box::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::Relaxed);
        }));

        let _reactive_clone = reactive.clone();
        drop(reactive);

        settings
            .set(
                "test.clone_drop_test",
                serde_json::to_value("changed").unwrap(),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 1);

        drop(subscription);

        settings
            .set(
                "test.clone_drop_test",
                serde_json::to_value("changed_again").unwrap(),
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }
}
