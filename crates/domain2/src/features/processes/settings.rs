use amethystate::{ReactiveMap, amethystate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, amethystate::AmeType)]
pub struct ColumnConfig {
    pub width: u64,
    pub min_width: u64,
    pub visible: bool,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            width: 110,
            min_width: 80,
            visible: true,
        }
    }
}



#[amethystate(prefix = "processes")]
pub struct ProcessesSettings {
    #[amestate(default = 500u64)]
    scan_interval_ms: u64,

    #[amestate(nested)]
    columns: ProcessesColumnsSettings,
}

#[amethystate]
pub struct ProcessesColumnsSettings {
    #[amestate(default = {
        "name": ColumnConfig { width: 280, min_width: 200, visible: true },
        "cpu": ColumnConfig { width: 80, min_width: 60, visible: true },
        "memory": ColumnConfig { width: 110, min_width: 80, visible: true },
        "disk": ColumnConfig { width: 110, min_width: 80, visible: true },
        "net": ColumnConfig { width: 110, min_width: 80, visible: true },
    })]
    configs: ReactiveMap<String, ColumnConfig>,
}
