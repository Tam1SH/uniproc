use crate::settings::{SettingsStore, SubscriptionId, SubscriptionKind};
use crate::signal::{Signal, SignalSubscription};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::{error, info};

pub struct SettingSubscription {
    pub settings: Arc<SettingsStore>,
    pub(crate) id: SubscriptionId,
}
pub struct ReactiveSettingSubscription {
    #[allow(unused)]
    pub(crate) signal_sub: SignalSubscription,
    #[allow(unused)]
    pub(crate) store_sub: SettingSubscription,
}

#[derive(Clone)]
pub struct ReactiveSetting<TValue> {
    pub signal: Arc<Signal<TValue>>,
    pub(crate) path: Arc<str>,
    pub(crate) _store_subscription: Arc<SettingSubscription>,
}

impl<TValue> ReactiveSetting<TValue>
where
    TValue: DeserializeOwned + Serialize + Send + Sync + Clone + 'static,
{
    pub fn get(&self) -> TValue {
        self.signal.value.load().as_ref().clone()
    }

    pub fn get_store_subscription(&self) -> Arc<SettingSubscription> {
        self._store_subscription.clone()
    }

    pub fn get_path(&self) -> Arc<str> {
        self.path.clone()
    }

    pub fn get_arc(&self) -> Arc<TValue> {
        self.signal.value.load_full()
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

        let settings = self._store_subscription.settings.clone();
        let signal = self.signal.clone();

        let path = self.path.clone();
        let settings_ref = settings.clone();
        let store_id = settings.subscribe(
            SubscriptionKind::Prefix(path.clone()),
            Arc::new(move |_event| {
                // TODO: codegen settings schema so that each DashMap key becomes a separate
                //       ReactiveSetting, eliminating the need to re-read the entire object on any child change
                match settings_ref.get(&path) {
                    Some(json) => match serde_json::from_value::<TValue>(json) {
                        Ok(parsed) => signal.set(parsed),
                        Err(e) => {
                            error!(
                                target: "settings",
                                path = %path,
                                error = %e,
                                "Failed to deserialize setting value"
                            );
                        }
                    },
                    None => {
                        info!(target: "settings", path = %path, "Setting value was removed");
                    }
                }
            }),
        );

        ReactiveSettingSubscription {
            signal_sub,
            store_sub: SettingSubscription {
                settings,
                id: store_id,
            },
        }
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
