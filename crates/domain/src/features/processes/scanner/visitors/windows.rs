use crate::features::processes::scanner::base::{
    Field, ProcessVisitor, ScanResult, VisitorContext,
};
use crate::processes_impl::scanner::base::DisplayNameRequest;
use crate::processes_impl::scanner::consts::*;
use crate::processes_impl::scanner::ctx::StatefulContext;
use crate::processes_impl::scanner::field_value::{FieldValue, FieldValueFormat, FieldValueKind};
use slint::SharedString;
use std::fmt::Write;
use std::sync::Arc;
use uniproc_protocol::{WindowsProcessStats, WindowsReport};

pub struct WindowsScanResult {
    pub report: WindowsReport,
    pub ctx: Arc<StatefulContext>,
}

impl ProcessVisitor for WindowsProcessStats {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn name(&self, ctx: &dyn VisitorContext) -> SharedString {
        ctx.resolve_display_name(
            DisplayNameRequest::builder()
                .pid(self.pid)
                .process_name(&self.name)
                .package_full_name(&self.package_full_name)
                .build(),
        )
    }

    fn package_name(&self, ctx: &dyn VisitorContext) -> Option<SharedString> {
        if self.package_full_name.is_empty() {
            return None;
        }

        Some(ctx.intern(&*self.package_full_name))
    }

    fn parent_pid(&self) -> u32 {
        self.parent_pid
    }

    fn exe_path(&self, ctx: &dyn VisitorContext) -> SharedString {
        ctx.intern(&self.cmdline[0])
    }

    fn visit(&self, ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field)) {
        let pid = self.pid();

        let mut cpu = ctx.get_field_value(pid, "cpu", FieldValueKind::Percent(self.cpu_percent));
        cpu.kind = FieldValueKind::Percent(self.cpu_percent);
        cpu.to_text();
        visitor(Field {
            id: ID_CPU.clone(),
            label: LBL_CPU.clone(),
            value: cpu,
            stat_detail: None,
            show_indicator: true,
            numeric: self.cpu_percent,
            threshold: 50.0,
        });

        let mut mem = ctx.get_field_value(
            pid,
            "mem",
            FieldValueKind::Bytes(self.working_set_kb * 1024),
        );
        mem.kind = FieldValueKind::Bytes(self.working_set_kb * 1024);
        mem.to_text();
        visitor(Field {
            id: ID_MEM.clone(),
            label: LBL_MEM.clone(),
            value: mem,
            stat_detail: None,
            show_indicator: true,
            numeric: self.working_set_kb as f32 / (1024.0 * 1024.0),
            threshold: 1.0,
        });

        let disk_total = self.disk_read_bytes.saturating_add(self.disk_write_bytes);
        let mut disk = ctx.get_field_value(pid, "disk", FieldValueKind::Bytes(disk_total));
        disk.kind = FieldValueKind::Bytes(disk_total);
        disk.to_text();
        visitor(Field {
            id: ID_DISK.clone(),
            label: LBL_DISK.clone(),
            value: disk,
            stat_detail: None,
            show_indicator: false,
            numeric: disk_total as f32,
            threshold: 0.1,
        });

        let net_total = self.net_rx_bytes + self.net_tx_bytes;
        let mut net = ctx.get_field_value(pid, "net", FieldValueKind::Bytes(net_total));
        net.kind = FieldValueKind::Bytes(net_total);
        net.to_text();
        visitor(Field {
            id: ID_NET.clone(),
            label: LBL_NET.clone(),
            value: net,
            stat_detail: None,
            show_indicator: false,
            numeric: net_total as f32,
            threshold: 0.1,
        });
    }
}

impl ScanResult for WindowsScanResult {
    fn context(&self) -> &dyn VisitorContext {
        self.ctx.as_ref()
    }

    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor)) {
        for process in &self.report.processes {
            visitor(process);
        }
    }

    fn visit_stats(&self, visitor: &mut dyn FnMut(Field)) {
        let machine = &self.report.machine;
        let pid = 0;

        let memory_detail = {
            let mut val = self
                .ctx
                .get_field_value(pid, "m_mem_d", FieldValueKind::U64(0));
            val.write_raw(|w| {
                FieldValue::write_part(
                    w,
                    machine.used_physical_kb as f64 / 1048576.0,
                    "",
                    &[
                        FieldValueFormat::WithoutUnit,
                        FieldValueFormat::WithoutSpaces,
                    ],
                )?;
                write!(w, "/")?;
                FieldValue::write_part(
                    w,
                    machine.total_physical_kb as f64 / 1048576.0,
                    "GB",
                    &[
                        FieldValueFormat::WithoutSpaces,
                        FieldValueFormat::RoundUp,
                        FieldValueFormat::WithoutDecimals,
                    ],
                )
            })
        };

        let cpu_clock_detail = {
            let mut val = self
                .ctx
                .get_field_value(pid, "m_cpu_d", FieldValueKind::U64(0));
            val.write_raw(|w| {
                FieldValue::write_part(
                    w,
                    machine.cpu_current_mhz as f64 / 1000.0,
                    "",
                    &[
                        FieldValueFormat::WithoutUnit,
                        FieldValueFormat::WithoutSpaces,
                    ],
                )?;
                write!(w, "/")?;
                FieldValue::write_part(
                    w,
                    machine.cpu_max_mhz as f64 / 1000.0,
                    "GHz",
                    &[FieldValueFormat::WithoutSpaces],
                )
            })
        };

        let mut cpu_v =
            self.ctx
                .get_field_value(pid, "m_cpu_v", FieldValueKind::Percent(machine.cpu_percent));
        cpu_v.kind = FieldValueKind::Percent(machine.cpu_percent);
        cpu_v.to_text();
        visitor(Field {
            id: ID_CPU.clone(),
            label: LBL_CPU.clone(),
            value: cpu_v,
            stat_detail: Some(cpu_clock_detail),
            show_indicator: true,
            numeric: machine.cpu_percent,
            threshold: 50.0,
        });

        let total = machine.total_physical_kb.max(1);
        let ram_pct = (machine.used_physical_kb as f32 / total as f32) * 100.0;
        let mut mem_v = self
            .ctx
            .get_field_value(pid, "m_mem_v", FieldValueKind::Percent(ram_pct));
        mem_v.kind = FieldValueKind::Percent(ram_pct);
        mem_v.to_text();
        visitor(Field {
            id: ID_MEM.clone(),
            label: LBL_MEM.clone(),
            value: mem_v,
            stat_detail: Some(memory_detail),
            show_indicator: true,
            numeric: ram_pct,
            threshold: 1.0,
        });

        let net_total = machine.net_rx_bytes + machine.net_tx_bytes;
        let mut net_v = self
            .ctx
            .get_field_value(pid, "m_net_v", FieldValueKind::Bytes(net_total));
        net_v.kind = FieldValueKind::Bytes(net_total);
        net_v.to_text();
        visitor(Field {
            id: ID_NET.clone(),
            label: LBL_NET.clone(),
            value: net_v,
            stat_detail: None,
            show_indicator: false,
            numeric: net_total as f32,
            threshold: 0.1,
        });

        let disk_total = machine
            .disk_read_bytes
            .saturating_add(machine.disk_write_bytes);
        let mut disk_v =
            self.ctx
                .get_field_value(pid, "m_disk_v", FieldValueKind::Bytes(disk_total));
        disk_v.kind = FieldValueKind::Bytes(disk_total);
        disk_v.to_text();
        visitor(Field {
            id: ID_DISK.clone(),
            label: LBL_DISK.clone(),
            value: disk_v,
            stat_detail: None,
            show_indicator: false,
            numeric: disk_total as f32,
            threshold: 0.1,
        });
    }
}
