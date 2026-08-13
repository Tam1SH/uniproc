use amethystate::MapChange;
use app_contracts::features::processes::ColumnConfig;
use guinea_core::signal::{Signal, SignalSubscription};
use uuid::Uuid;

const COLUMN_IDS: &[&str] = &["name", "cpu", "memory", "net", "disk"];
const COLUMN_DEFAULT: ColumnConfig = ColumnConfig {
    width: 110,
    min_width: 80,
    visible: true,
};

pub(crate) struct ColumnLayout {
    pub(crate) entries: Vec<ColumnLayoutEntry>,
}

pub(crate) struct ColumnLayoutEntry {
    pub(crate) id: &'static str,
    pub(crate) width: Signal<u64>,
    pub(crate) min_width: Signal<u64>,
    pub(crate) visible: Signal<bool>,
    _subs: (SignalSubscription, SignalSubscription, SignalSubscription),
}

impl ColumnLayout {
    pub(crate) fn new<S: amethystate::Store>(
        map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
    ) -> Self {
        let entries = COLUMN_IDS
            .iter()
            .map(|&id| ColumnLayoutEntry::new(id, map))
            .collect();
        Self { entries }
    }
}

impl ColumnLayoutEntry {
    fn new<S: amethystate::Store>(
        id: &'static str,
        map: &amethystate::ReactiveMap<String, ColumnConfig, S, amethystate::WritableMode>,
    ) -> Self {
        let initial = map
            .get(&id.to_string())
            .ok()
            .flatten()
            .unwrap_or(COLUMN_DEFAULT);
        let width = Signal::new(initial.width);
        let min_width = Signal::new(initial.min_width);
        let visible = Signal::new(initial.visible);

        let store_source = Uuid::new_v4();

        let width_for_read = width.clone();
        let min_width_for_read = min_width.clone();
        let visible_for_read = visible.clone();
        let read = map.subscribe_key(id.to_string(), move |change| {
            let next = match change {
                MapChange::Insert { value, .. }
                | MapChange::Update {
                    new_value: value, ..
                } => *value,
                MapChange::Remove { .. } | MapChange::Clear { .. } => COLUMN_DEFAULT,
            };
            width_for_read.set(next.width, Some(store_source));
            min_width_for_read.set(next.min_width, Some(store_source));
            visible_for_read.set(next.visible, Some(store_source));
        });

        let map_for_write_w = map.clone();
        let key_w = id.to_string();
        let write_w = width.subscribe_with_source(move |w, source| {
            if source != Some(store_source) {
                let current = map_for_write_w
                    .get(&key_w)
                    .ok()
                    .flatten()
                    .unwrap_or(COLUMN_DEFAULT);
                let next = ColumnConfig {
                    width: *w,
                    ..current
                };
                let _ = map_for_write_w.set_or_create(key_w.clone(), &next);
            }
        });

        let map_for_write_v = map.clone();
        let key_v = id.to_string();
        let write_v = visible.subscribe_with_source(move |v, source| {
            if source != Some(store_source) {
                let current = map_for_write_v
                    .get(&key_v)
                    .ok()
                    .flatten()
                    .unwrap_or(COLUMN_DEFAULT);
                let next = ColumnConfig {
                    visible: *v,
                    ..current
                };
                let _ = map_for_write_v.set_or_create(key_v.clone(), &next);
            }
        });

        Self {
            id,
            width,
            min_width,
            visible,
            _subs: (read, write_w, write_v),
        }
    }
}
