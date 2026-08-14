use amethystate::{ReactiveCell, ReactiveMap, Store, WritableMode};
use app_contracts::features::processes::ColumnConfig;
use guinea::widgets::table::Width;

const COLUMN_IDS: &[&str] = &["name", "cpu", "memory", "net", "disk"];

pub(crate) struct ColumnLayout {
    pub(crate) entries: Vec<ColumnLayoutEntry>,
}

pub(crate) struct ColumnLayoutEntry {
    pub(crate) id: &'static str,
    config: ReactiveCell<ColumnConfig>,
}

impl ColumnLayout {
    pub(crate) fn new<S: Store>(map: &ReactiveMap<String, ColumnConfig, S, WritableMode>) -> Self {
        let entries = COLUMN_IDS
            .iter()
            .map(|&id| ColumnLayoutEntry {
                id,
                config: map.entry_cell(id.to_string(), ColumnConfig::default()),
            })
            .collect();
        Self { entries }
    }
}

impl ColumnLayoutEntry {
    pub(crate) fn width(&self) -> Width {
        let read = self.config.clone();
        let write = self.config.clone();
        Width::bound(
            move || read.get().width,
            move |width| {
                if let Err(err) = write.update(|config| ColumnConfig { width, ..config }) {
                    tracing::warn!(?err, "column width write failed");
                }
            },
        )
    }

    pub(crate) fn min_width(&self) -> f64 {
        self.config.get().min_width as f64
    }

    pub(crate) fn visible(&self) -> bool {
        self.config.get().visible
    }
}
