use crate::processes_impl::scanner::field_value::{FieldValue, FieldValueKind};
use bon::Builder;
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

#[derive(Builder)]
pub struct DisplayNameRequest<'a> {
    pub pid: u32,
    pub process_name: &'a str,
    pub exe_path: Option<&'a str>,

    #[cfg(windows)]
    pub package_full_name: Option<&'a str>,
}

pub trait VisitorContext {
    fn get_field_value(&self, pid: u32, field_id: &'static str, kind: FieldValueKind)
    -> FieldValue;

    fn resolve_display_name(&self, req: DisplayNameRequest) -> SharedString;

    fn tick(&self);

    fn intern(&self, s: &str) -> SharedString;
}

pub trait ProcessVisitor {
    fn pid(&self) -> u32;
    fn name(&self, ctx: &dyn VisitorContext) -> SharedString;
    #[cfg(windows)]
    fn package_name(&self, ctx: &dyn VisitorContext) -> Option<SharedString>;
    fn parent_pid(&self) -> u32;
    fn exe_path(&self, ctx: &dyn VisitorContext) -> SharedString;
    // fn icon_path(&self, ctx: &dyn VisitorContext) -> SharedString;
    fn visit(&self, ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field));
}

pub trait ScanResult: Send + Sync {
    fn context(&self) -> &dyn VisitorContext;
    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor));
    fn visit_stats(&self, visitor: &mut dyn FnMut(Field));
}
