use app_core::settings::reactive::ReactiveSettingSubscription;
use app_core::signal::Signal;
use dashmap::DashMap;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

pub trait TableSettingsProvider {
    fn default_width(&self) -> anyhow::Result<u64>;
    fn initial_widths(&self) -> anyhow::Result<DashMap<String, u64>>;
    fn min_widths(&self) -> anyhow::Result<DashMap<String, u64>>;
    fn subscribe_widths<F>(&self, callback: F) -> ReactiveSettingSubscription
    where
        F: Fn(DashMap<String, u64>) + Send + Sync + 'static;

    fn setup<ID>(self, layout: &mut TableLayout<ID>) -> anyhow::Result<ReactiveSettingSubscription>
    where
        ID: From<String> + Eq + Hash + Clone + Send + Sync + 'static,
        Self: Sized,
    {
        setup_table_layout(layout, &self)
    }
}

pub struct TableLayout<ID> {
    pub widths: HashMap<ID, Arc<Signal<u64>>>,
}

impl<ID> Default for TableLayout<ID>
where
    ID: Hash + Eq + Clone + Send + Sync + 'static,
 {
    fn default() -> Self {
        Self::new()
    }
}

impl<ID> TableLayout<ID>
where
    ID: Hash + Eq + Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            widths: HashMap::new(),
        }
    }

    pub fn snapshot(&self) -> Vec<(ID, u64)> {
        self.widths
            .iter()
            .map(|(id, sig)| (id.clone(), *sig.get_arc()))
            .collect()
    }

    pub fn add_column(&mut self, id: ID, width_signal: Arc<Signal<u64>>) {
        self.widths.insert(id, width_signal);
    }

    pub fn get_width(&self, id: &ID) -> u64 {
        self.widths.get(id).map(|s| *s.get_arc()).unwrap_or(0)
    }

    pub fn set_width(&self, id: &ID, new_width: u64) {
        if let Some(sig) = self.widths.get(id) {
            sig.set(new_width);
        }
    }

    pub fn apply_to_vms<VM>(&self, vms: &mut [VM], mut patch_fn: impl FnMut(&mut VM, &ID, u64)) {
        for (id, sig) in &self.widths {
            let w = *sig.get_arc();
            for vm in vms.iter_mut() {
                patch_fn(vm, id, w);
            }
        }
    }
}

pub fn setup_table_layout<ID>(
    layout: &mut TableLayout<ID>,
    provider: &impl TableSettingsProvider,
) -> anyhow::Result<ReactiveSettingSubscription>
where
    ID: From<String> + Eq + Hash + Clone + Send + Sync + 'static,
{
    let def_width = provider.default_width()?;
    let initial_widths = provider.initial_widths()?;
    let min_widths = provider.min_widths()?;

    initial_widths.iter().for_each(|pair| {
        let (id, &w) = pair.pair();
        let min_w = min_widths.get(id).map(|a| *a.value()).unwrap_or(def_width);
        layout.add_column(ID::from(id.clone()), Arc::new(Signal::new(w.max(min_w))));
    });

    let signal_map = layout.widths.clone();

    Ok(provider.subscribe_widths(move |new_map| {
        for (id, w) in new_map {
            if let Some(sig) = signal_map.get(&ID::from(id)) {
                sig.set(w);
            }
        }
    }))
}
