use amethystate::{ReactiveMap, amethystate};
use app_contracts::features::processes::ColumnConfig;

#[amethystate(prefix = "processes")]
pub struct ProcessesSettings {
    #[amestate(default = 500u64)]
    scan_interval_ms: u64,

    #[amestate(nested)]
    columns: ProcessesColumnsSettings,

    #[amestate(nested)]
    grouping: ProcessesGroupingSettings,
}

#[amethystate]
pub struct ProcessesGroupingSettings {
    #[amestate(default = {})]
    expanded_groups: ReactiveMap<String, bool>,

    #[amestate(default = {})]
    collapsed_sections: ReactiveMap<String, bool>,
}

#[amethystate]
pub struct ProcessesColumnsSettings {
    #[amestate(default = {
        "name": ColumnConfig { width: 280, min_width: 200, visible: true },
        "cpu": ColumnConfig { width: 120, min_width: 90, visible: true },
        "memory": ColumnConfig { width: 140, min_width: 90, visible: true },
        "net": ColumnConfig { width: 110, min_width: 80, visible: true },
        "disk": ColumnConfig { width: 110, min_width: 80, visible: true },
    })]
    configs: ReactiveMap<String, ColumnConfig>,
}
