use app_contracts::features::processes::{FieldDefDto, ProcessNodeDto};

#[derive(Clone)]
pub struct BridgeSnapshot {
    pub column_defs: Vec<FieldDefDto>,
    pub processes: Vec<ProcessNodeDto>,
}
