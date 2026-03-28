use crate::processes_impl::scanner::base::VisitorContext;
use crate::processes_impl::scanner::field_value::{FieldValue, FieldValueKind};
use dashmap::DashMap;
use slint::SharedString;

pub struct StatefulContext {
    pub cache: DashMap<(u32, &'static str), FieldValue>,
    pub names: DashMap<u32, SharedString>,
}

impl StatefulContext {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            names: DashMap::new(),
        }
    }

    pub fn clear_dead_processes(&self, active_pids: &[u32]) {
        self.cache
            .retain(|(pid, _), _| *pid == 0 || active_pids.contains(pid));

        self.names
            .retain(|pid, _| *pid == 0 || active_pids.contains(pid));
    }
}

impl VisitorContext for StatefulContext {
    fn get(&self, _key: &str) -> Option<f32> {
        None
    }

    fn get_field_value(
        &self,
        pid: u32,
        field_id: &'static str,
        kind: FieldValueKind,
    ) -> FieldValue {
        self.cache
            .entry((pid, field_id))
            .or_insert_with(|| FieldValue::new(kind))
            .clone()
    }

    fn intern_name(&self, pid: u32, raw_bytes: &[u8]) -> SharedString {
        let end = raw_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(raw_bytes.len());
        let s = std::str::from_utf8(&raw_bytes[..end]).unwrap_or("<invalid>");

        if let Some(existing) = self.names.get(&pid) {
            if existing.as_str() == s {
                return existing.clone();
            }
        }

        let new_shared = SharedString::from(s);
        self.names.insert(pid, new_shared.clone());
        new_shared
    }
}
