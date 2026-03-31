use crate::features::processes::scanner::base::{
    Field, ProcessVisitor, ScanResult, VisitorContext,
};
use crate::processes_impl::scanner::base::DisplayNameRequest;
use crate::processes_impl::scanner::consts::*;
use crate::processes_impl::scanner::ctx::StatefulContext;
use crate::processes_impl::scanner::field_value::FieldValueKind;
use slint::SharedString;
use std::sync::Arc;
use uniproc_protocol::{LinuxMachineStats, LinuxProcessStats as WslProcessStat};

pub struct WslScanResult {
    pub processes: Vec<WslProcessStat>,
    pub machine: LinuxMachineStats,
    pub ctx: Arc<StatefulContext>,
}

impl ProcessVisitor for WslProcessStat {
    fn pid(&self) -> u32 {
        self.global_pid
    }

    fn name(&self, ctx: &dyn VisitorContext) -> SharedString {
        let len = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name.len());
        let s = std::str::from_utf8(&self.name[..len]).unwrap_or("<invalid>");

        ctx.resolve_display_name(
            DisplayNameRequest::builder()
                .pid(self.local_pid)
                .process_name(s)
                .build(),
        )
    }

    fn package_name(&self, _ctx: &dyn VisitorContext) -> Option<SharedString> {
        None
    }

    fn parent_pid(&self) -> u32 {
        0
    }

    fn exe_path(&self, ctx: &dyn VisitorContext) -> SharedString {
        let len = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name.len());
        let s = std::str::from_utf8(&self.name[..len]).unwrap_or("<invalid>");

        ctx.intern(s)
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

        let mut mem = ctx.get_field_value(pid, "mem", FieldValueKind::Bytes(self.rss_kb * 1024));
        mem.kind = FieldValueKind::Bytes(self.rss_kb * 1024);
        mem.to_text();
        visitor(Field {
            id: ID_MEM.clone(),
            label: LBL_MEM.clone(),
            value: mem,
            stat_detail: None,
            show_indicator: true,
            numeric: self.rss_kb as f32 / (1024.0 * 1024.0),
            threshold: 1.0,
        });

        let mut dr = ctx.get_field_value(pid, "disk", FieldValueKind::Bytes(self.disk_read_bytes));
        dr.kind = FieldValueKind::Bytes(self.disk_read_bytes + self.disk_write_bytes);
        dr.to_text();
        visitor(Field {
            id: ID_DISK.clone(),
            label: LBL_DISK.clone(),
            value: dr,
            stat_detail: None,
            show_indicator: false,
            numeric: self.disk_read_bytes as f32,
            threshold: 0.1,
        });

        let net_total = self.vsock_rx_bytes
            + self.vsock_tx_bytes
            + self.p9_rx_bytes
            + self.p9_tx_bytes
            + self.tcp_tx_bytes
            + self.tcp_rx_bytes
            + self.udp_tx_bytes
            + self.udp_rx_bytes
            + self.uds_tx_bytes
            + self.uds_rx_bytes;
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

impl ScanResult for WslScanResult {
    fn context(&self) -> &dyn VisitorContext {
        self.ctx.as_ref()
    }

    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor)) {
        for p in &self.processes {
            visitor(p);
        }
    }

    fn visit_stats(&self, visitor: &mut dyn FnMut(Field)) {
        let total = self.machine.total_kb.max(1);
        let ram_pct = (self.machine.used_kb as f32 / total as f32) * 100.0;
        let mut mem = self
            .ctx
            .get_field_value(0, "m_mem", FieldValueKind::Percent(ram_pct));
        mem.kind = FieldValueKind::Percent(ram_pct);
        mem.to_text();
        visitor(Field {
            id: ID_MEM.clone(),
            label: LBL_MEM.clone(),
            value: mem,
            stat_detail: None,
            show_indicator: true,
            numeric: ram_pct,
            threshold: 1.0,
        });

        let net_total = self.machine.vsock_rx_bytes
            + self.machine.vsock_tx_bytes
            + self.machine.p9_rx_bytes
            + self.machine.p9_tx_bytes
            + self.machine.tcp_tx_bytes
            + self.machine.tcp_rx_bytes
            + self.machine.udp_tx_bytes
            + self.machine.udp_rx_bytes
            + self.machine.uds_tx_bytes
            + self.machine.uds_rx_bytes;
        let mut net = self
            .ctx
            .get_field_value(0, "m_net", FieldValueKind::Bytes(net_total));
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

        let mut dr = self.ctx.get_field_value(
            0,
            "m_disk",
            FieldValueKind::Bytes(self.machine.disk_read_bytes),
        );
        dr.kind =
            FieldValueKind::Bytes(self.machine.disk_read_bytes + self.machine.disk_write_bytes);
        dr.to_text();
        visitor(Field {
            id: ID_DISK.clone(),
            label: ID_DISK.clone(),
            value: dr,
            stat_detail: None,
            show_indicator: false,
            numeric: self.machine.disk_read_bytes as f32,
            threshold: 0.1,
        });
    }
}
