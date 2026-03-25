use crate::features::processes::scanner::base::ScanResult;
use app_contracts::features::processes::{FieldDefDto, ProcessFieldDto, ProcessNodeDto};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct ColumnWidthConfig {
    pub widths_px: HashMap<String, u32>,
    pub min_widths_px: HashMap<String, u32>,
    pub default_width_px: u32,
}

impl Default for ColumnWidthConfig {
    fn default() -> Self {
        let mut widths_px = HashMap::new();
        widths_px.insert("memory".to_string(), 120);
        widths_px.insert("cpu".to_string(), 85);

        let mut min_widths_px = HashMap::new();
        min_widths_px.insert("memory".to_string(), 120);
        min_widths_px.insert("cpu".to_string(), 85);

        Self {
            widths_px,
            min_widths_px,
            default_width_px: 70,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VisitorSharedState {
    widths_px: Arc<RwLock<HashMap<String, u32>>>,
    min_widths_px: Arc<RwLock<HashMap<String, u32>>>,
    default_width_px: u32,
}

impl VisitorSharedState {
    pub fn new() -> Self {
        Self::with_config(&ColumnWidthConfig::default())
    }

    pub fn with_config(config: &ColumnWidthConfig) -> Self {
        Self {
            widths_px: Arc::new(RwLock::new(config.widths_px.clone())),
            min_widths_px: Arc::new(RwLock::new(config.min_widths_px.clone())),
            default_width_px: config.default_width_px,
        }
    }

    pub fn set_width_px(&self, key: &str, width: u32) {
        self.widths_px.write().unwrap().insert(key.to_string(), width);
    }

    pub fn set_min_width_px(&self, key: &str, min: u32) {
        self.min_widths_px
            .write()
            .unwrap()
            .insert(key.to_string(), min);
    }

    pub fn get_width_px(&self, key: &str) -> u32 {
        let w = self
            .widths_px
            .read()
            .unwrap()
            .get(key)
            .copied()
            .unwrap_or(self.default_width_px);

        let min = self
            .min_widths_px
            .read()
            .unwrap()
            .get(key)
            .copied()
            .unwrap_or(self.default_width_px);

        w.max(min)
    }
}

#[derive(Clone)]
pub struct BridgeSnapshot {
    pub column_defs: Vec<FieldDefDto>,
    pub processes: Vec<ProcessNodeDto>,
}

pub fn build_snapshot(result: &dyn ScanResult, shared: &VisitorSharedState) -> BridgeSnapshot {
    let mut column_defs: Vec<FieldDefDto> = vec![];

    result.visit_stats(&mut |field| {
        column_defs.push(FieldDefDto {
            id: field.id.to_string(),
            label: field.label.to_string(),
            stat_text: field.value.to_text(),
            stat_numeric: field.numeric,
            threshold: field.threshold,
            stat_detail: field.stat_detail,
            show_indicator: field.show_indicator,
            width_px: shared.get_width_px(field.id) as i32,
        });
    });

    let ctx = result.context();
    let mut processes: Vec<ProcessNodeDto> = vec![];

    result.visit_processes(&mut |proc| {
        let mut fields: Vec<ProcessFieldDto> = vec![];
        proc.visit(&*ctx, &mut |field| {
            fields.push(ProcessFieldDto {
                id: field.id.to_string(),
                text: field.value.to_text(),
                width_px: shared.get_width_px(field.id) as i32,
                numeric: field.numeric,
                threshold: field.threshold,
            });
        });

        processes.push(ProcessNodeDto {
            pid: proc.pid(),
            name: proc.name().to_string(),
            parent_pid: proc.parent_pid(),
            exe_path: proc.exe_path().map(|s| s.to_string()),
            fields,
        });
    });

    BridgeSnapshot {
        column_defs,
        processes,
    }
}
