use crate::processes_impl::scanner::field_value::{FieldValue, FieldValueKind};
use lazy_static::lazy_static;
use slint::SharedString;
use std::sync::Mutex;

lazy_static! {
    pub static ref ID_CPU: SharedString = "cpu".into();
    pub static ref ID_MEM: SharedString = "memory".into();
    pub static ref ID_NET: SharedString = "net".into();
    pub static ref ID_DISK: SharedString = "disk".into();
    pub static ref LBL_CPU: SharedString = "CPU".into();
    pub static ref LBL_MEM: SharedString = "Memory".into();
    pub static ref LBL_NET: SharedString = "Net".into();
    pub static ref LBL_DISK: SharedString = "Disk".into();
    pub static ref MEM_DETAIL_CACHE: Mutex<FieldValue> =
        Mutex::new(FieldValue::new(FieldValueKind::U64(0)));
    pub static ref CPU_DETAIL_CACHE: Mutex<FieldValue> =
        Mutex::new(FieldValue::new(FieldValueKind::U64(0)));
}
