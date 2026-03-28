use crate::settings::{SettingsStore, SubscriptionId};
use crate::signal::{Signal, SignalSubscription};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub struct SettingSubscription {
    pub(crate) settings: Arc<SettingsStore>,
    pub(crate) id: SubscriptionId,
}

impl Drop for SettingSubscription {
    fn drop(&mut self) {
        self.settings.unsubscribe(self.id);
    }
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

    pub fn get_arc(&self) -> Arc<TValue> {
        self.signal.value.load_full()
    }

    pub fn subscribe<F>(&self, callback: F) -> SignalSubscription
    where
        F: Fn(TValue) + Send + Sync + 'static,
    {
        self.signal.subscribe(move |val: &TValue| {
            callback(val.clone());
        })
    }

    pub fn set(&self, value: TValue) -> anyhow::Result<()> {
        let json = serde_json::to_value(value)?;
        self._store_subscription.settings.set(&self.path, json)
    }

    pub fn as_signal(&self) -> Arc<Signal<TValue>> {
        self.signal.clone()
    }
}
