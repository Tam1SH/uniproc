use crate::processes_impl::scanner::field_value::{FieldValue, FieldValueKind};
use slint::SharedString;

#[derive(Debug, Clone)]
pub struct Field {
    pub id: SharedString,
    pub label: SharedString,
    pub value: FieldValue,
    pub stat_detail: Option<SharedString>,
    pub show_indicator: bool,
    pub numeric: f32,
    pub threshold: f32,
}

pub trait VisitorContext {
    fn get(&self, key: &str) -> Option<f32>;
    fn get_field_value(&self, pid: u32, field_id: &'static str, kind: FieldValueKind)
    -> FieldValue;

    fn intern_name(&self, pid: u32, raw_bytes: &[u8]) -> SharedString;
}

pub trait ProcessVisitor {
    fn pid(&self) -> u32;
    fn name(&self, ctx: &dyn VisitorContext) -> SharedString;
    fn parent_pid(&self) -> u32;
    fn exe_path(&self) -> Option<&str>;
    fn visit(&self, ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field));
}

pub trait ScanResult: Send + Sync {
    fn context(&self) -> &dyn VisitorContext;
    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor));
    fn visit_stats(&self, visitor: &mut dyn FnMut(Field));
}
