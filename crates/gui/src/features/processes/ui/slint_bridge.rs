use crate::features::processes::scanner::base::{FieldValue, ScanResult};
use crate::{FieldDef, ProcessField};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
pub struct VisitorSharedState {
    widths_px: Arc<RwLock<HashMap<&'static str, u32>>>,
    min_widths_px: Arc<RwLock<HashMap<&'static str, u32>>>,
    default_width_px: u32,
}

impl VisitorSharedState {
    pub fn new() -> Self {
        let mut widths_px = HashMap::new();
        widths_px.insert("memory", 140);

        let mut min_widths_px = HashMap::new();
        min_widths_px.insert("memory", 140);

        Self {
            widths_px: Arc::new(RwLock::new(widths_px)),
            min_widths_px: Arc::new(RwLock::new(min_widths_px)),
            default_width_px: 70,
        }
    }

    pub fn set_width_px(&self, key: &'static str, width: u32) {
        self.widths_px.write().unwrap().insert(key, width);
    }

    pub fn set_min_width_px(&self, key: &'static str, min: u32) {
        self.min_widths_px.write().unwrap().insert(key, min);
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

fn format_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    match b {
        b if b >= GB => format!("{:.1} GB", b as f64 / GB as f64),
        b if b >= MB => format!("{:.1} MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.1} KB", b as f64 / KB as f64),
        b => format!("{} B", b),
    }
}

fn field_to_text(value: &FieldValue) -> String {
    match value {
        FieldValue::Bytes(b) => format_bytes(*b),
        FieldValue::Percent(p) => format!("{:.1}%", p),
        FieldValue::U64(v) => v.to_string(),
        FieldValue::F32(v) => format!("{:.1}", v),
        FieldValue::Str(s) => s.clone(),
        FieldValue::Duration(d) => format!("{}ms", d.as_millis()),
    }
}

pub struct SlintProcess {
    pub pid: u32,
    pub name: String,
    pub parent_pid: u32,
    pub exe_path: Option<String>,
    pub fields: Vec<ProcessField>,
}

pub struct BridgeSnapshot {
    pub column_defs: Vec<FieldDef>,
    pub processes: Vec<SlintProcess>,
}

pub fn build_snapshot(result: &dyn ScanResult, shared: &VisitorSharedState) -> BridgeSnapshot {
    let mut column_defs: Vec<FieldDef> = vec![];

    result.visit_stats(&mut |field| {
        column_defs.push(FieldDef {
            id: field.id.into(),
            label: field.label.into(),
            stat_text: field_to_text(&field.value).into(),
            stat_numeric: field.numeric,
            threshold: field.threshold,
            width_px: shared.get_width_px(field.id) as i32,
        });
    });

    let ctx = result.context();
    let mut processes: Vec<SlintProcess> = vec![];

    result.visit_processes(&mut |proc| {
        let mut fields: Vec<ProcessField> = vec![];
        proc.visit(&*ctx, &mut |field| {
            fields.push(ProcessField {
                id: field.id.into(),
                text: field_to_text(&field.value).into(),
                width_px: shared.get_width_px(field.id) as i32,
                numeric: field.numeric,
                threshold: field.threshold,
            });
        });

        processes.push(SlintProcess {
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
