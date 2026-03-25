use crate::features::processes::scanner::base::{
    Field, FieldValue, FieldValueFormat, ProcessVisitor, ScanResult, VisitorContext,
};
use uniproc_protocol::{WindowsMachineStats, WindowsProcessStats, WindowsReport};

pub struct WindowsScanResult {
    pub report: WindowsReport,
}

struct WindowsVisitorContext;

impl VisitorContext for WindowsVisitorContext {
    fn get(&self, _key: &str) -> Option<f32> {
        None
    }
}

impl ProcessVisitor for WindowsProcessStats {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn name(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(self.name.len());
        std::str::from_utf8(&self.name[..end]).unwrap_or("<invalid>")
    }

    fn parent_pid(&self) -> u32 {
        self.parent_pid
    }

    fn exe_path(&self) -> Option<&str> {
        None
    }

    fn visit(&self, _ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field)) {
        visitor(Field {
            id: "cpu",
            label: "CPU",
            value: FieldValue::Percent(self.cpu_percent),
            stat_detail: None,
            show_indicator: true,
            numeric: self.cpu_percent,
            threshold: 50.0,
        });

        visitor(Field {
            id: "memory",
            label: "Memory",
            value: FieldValue::Bytes(self.working_set_kb * 1024),
            stat_detail: None,
            show_indicator: true,
            numeric: self.working_set_kb as f32 / (1024.0 * 1024.0),
            threshold: 1.0,
        });

        let disk_total = self.disk_read_bytes.saturating_add(self.disk_write_bytes);
        visitor(Field {
            id: "disk_read",
            label: "Disk",
            value: FieldValue::Bytes(disk_total),
            stat_detail: None,
            show_indicator: false,
            numeric: disk_total as f32,
            threshold: 0.1,
        });

        let net_total = self.net_rx_bytes + self.net_tx_bytes;
        visitor(Field {
            id: "net",
            label: "Net",
            value: FieldValue::Bytes(net_total),
            stat_detail: None,
            show_indicator: false,
            numeric: net_total as f32,
            threshold: 0.1,
        });
    }
}

impl ScanResult for WindowsScanResult {
    fn context(&self) -> Box<dyn VisitorContext> {
        Box::new(WindowsVisitorContext)
    }

    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor)) {
        for process in &self.report.processes {
            visitor(process);
        }
    }

    fn visit_stats(&self, visitor: &mut dyn FnMut(Field)) {
        visit_machine_stats(&self.report.machine, visitor);
    }
}

fn visit_machine_stats(machine: &WindowsMachineStats, visitor: &mut dyn FnMut(Field)) {
    let total = machine.total_physical_kb.max(1);
    let ram_pct = (machine.used_physical_kb as f32 / total as f32) * 100.0;
    let total_memory_gb = machine.total_physical_kb as f64 / (1024.0 * 1024.0);
    let used_memory_gb = machine.used_physical_kb as f64 / (1024.0 * 1024.0);

    let memory_detail = format!(
        "{}/{}",
        FieldValue::format_value_with_params(
            used_memory_gb,
            "GB",
            &[FieldValueFormat::WithoutUnit, FieldValueFormat::WithoutSpaces]
        ),
        FieldValue::format_value_with_params(
            total_memory_gb,
            "GB",
            &[FieldValueFormat::WithoutSpaces, FieldValueFormat::RoundUp, FieldValueFormat::WithoutDecimals]
        )
    );
    let cpu_current_ghz = machine.cpu_current_mhz as f64 / 1000.0;
    let cpu_max_ghz = machine.cpu_max_mhz as f64 / 1000.0;

    let cpu_clock_detail = format!(
        "{}/{}",
        FieldValue::format_value_with_params(
            cpu_current_ghz,
            "GHz",
            &[FieldValueFormat::WithoutUnit, FieldValueFormat::WithoutSpaces]
        ),
        FieldValue::format_value_with_params(
            cpu_max_ghz,
            "GHz",
            &[FieldValueFormat::WithoutSpaces]
        )
    );

    visitor(Field {
        id: "cpu",
        label: "CPU",
        value: FieldValue::Percent(machine.cpu_percent),
        stat_detail: Some(cpu_clock_detail),
        show_indicator: true,
        numeric: machine.cpu_percent,
        threshold: 50.0,
    });

    visitor(Field {
        id: "memory",
        label: "Memory",
        value: FieldValue::Percent(ram_pct),
        stat_detail: Some(memory_detail),
        show_indicator: true,
        numeric: ram_pct,
        threshold: 1.0,
    });

    let net_total = machine.net_rx_bytes + machine.net_tx_bytes;
    visitor(Field {
        id: "net",
        label: "Net",
        value: FieldValue::Bytes(net_total),
        stat_detail: None,
        show_indicator: false,
        numeric: net_total as f32,
        threshold: 0.1,
    });

    let disk_total = machine.disk_read_bytes.saturating_add(machine.disk_write_bytes);
    visitor(Field {
        id: "disk_read",
        label: "Disk",
        value: FieldValue::Bytes(disk_total),
        stat_detail: None,
        show_indicator: false,
        numeric: disk_total as f32,
        threshold: 0.1,
    });
}
