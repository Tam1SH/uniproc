use crate::features::processes::scanner::base::{
    Field, FieldValue, ProcessVisitor, ScanResult, VisitorContext,
};
use crate::features::processes::scanner::windows::types::{
    WindowsProcessStat, WindowsScanResult, WindowsStats,
};
use std::collections::HashMap;

pub struct WindowsVisitorContext {
    values: HashMap<&'static str, f32>,
}

impl WindowsVisitorContext {
    pub fn new(stats: &WindowsStats) -> Self {
        let mut values = HashMap::new();
        values.insert("total_memory", stats.total_memory as f32);
        values.insert("net_total_bandwidth", stats.net_total_bandwidth as f32);
        values.insert("disk_threshold", 5.0 * 1024.0 * 1024.0);
        Self { values }
    }
}

impl VisitorContext for WindowsVisitorContext {
    fn get(&self, key: &str) -> Option<f32> {
        self.values.get(key).copied()
    }
}

impl ProcessVisitor for WindowsProcessStat {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parent_pid(&self) -> u32 {
        self.parent_pid
    }

    fn exe_path(&self) -> Option<&str> {
        self.exe_path.as_deref()
    }

    fn visit(&self, ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field)) {
        let net_bw = ctx.get("net_total_bandwidth").unwrap_or(1.0);
        let disk_thr = ctx.get("disk_threshold").unwrap_or(1.0);

        visitor(Field {
            id: "cpu",
            label: "CPU",
            value: FieldValue::Percent(self.cpu_usage),
            numeric: self.cpu_usage,
            threshold: 50.0,
        });

        visitor(Field {
            id: "memory",
            label: "Memory",
            value: FieldValue::Bytes(self.memory_usage),
            numeric: self.memory_usage as f32 / (1024.0 * 1024.0 * 1024.0),
            threshold: 1.0,
        });

        if let Some(net) = self.net_usage {
            visitor(Field {
                id: "net",
                label: "Network",
                value: FieldValue::Bytes(net),
                numeric: (net as f32 / net_bw) * 100.0,
                threshold: 70.0,
            });
        }

        let disk_total = self
            .disk_read
            .unwrap_or(0)
            .saturating_add(self.disk_write.unwrap_or(0));

        if disk_total > 0 {
            visitor(Field {
                id: "disk_read",
                label: "Disk",
                value: FieldValue::Bytes(disk_total),
                numeric: (disk_total as f32 / disk_thr) * 100.0,
                threshold: 70.0,
            });
        }
    }
}

impl ScanResult for WindowsScanResult {
    fn context(&self) -> Box<dyn VisitorContext> {
        Box::new(WindowsVisitorContext::new(&self.stats))
    }

    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor)) {
        for p in &self.processes {
            visitor(p);
        }
    }

    fn visit_stats(&self, visitor: &mut dyn FnMut(Field)) {
        visitor(Field {
            id: "cpu",
            label: "CPU",
            value: FieldValue::Percent(self.stats.cpu_percent),
            numeric: self.stats.cpu_percent,
            threshold: 50.0,
        });

        visitor(Field {
            id: "memory",
            label: "Memory",
            value: FieldValue::Percent(self.stats.ram_percent),
            numeric: self.stats.ram_percent,
            threshold: 80.0,
        });

        visitor(Field {
            id: "disk_read",
            label: "Disk",
            value: FieldValue::Percent(self.stats.disk_percent),
            numeric: self.stats.disk_percent,
            threshold: 70.0,
        });

        visitor(Field {
            id: "net",
            label: "Network",
            value: FieldValue::Percent(self.stats.net_percent),
            numeric: self.stats.net_percent,
            threshold: 70.0,
        });
    }
}
