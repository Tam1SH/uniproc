use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum FieldValue {
    U64(u64),
    F32(f32),
    Str(String),
    Bytes(u64),
    Percent(f32),
    Duration(std::time::Duration),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub id: &'static str,
    pub label: &'static str,
    pub value: FieldValue,
    pub numeric: f32,
    pub threshold: f32,
}

pub trait VisitorContext {
    fn get(&self, key: &str) -> Option<f32>;
}

pub trait ProcessVisitor {
    fn pid(&self) -> u32;
    fn name(&self) -> &str;
    fn parent_pid(&self) -> u32;
    fn exe_path(&self) -> Option<&str>;
    fn visit(&self, ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field));
}

pub trait ScanResult: Send {
    fn context(&self) -> Box<dyn VisitorContext>;
    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor));
    fn visit_stats(&self, visitor: &mut dyn FnMut(Field));
}

#[async_trait]
pub trait ProcessScanner: Send {
    fn schema_id(&self) -> &'static str;
    async fn scan(&mut self) -> Box<dyn ScanResult>;
}
