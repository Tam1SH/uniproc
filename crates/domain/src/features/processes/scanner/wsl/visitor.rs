use crate::features::processes::scanner::base::{
    Field, ProcessVisitor, ScanResult, VisitorContext,
};
use uniproc_protocol::{MachineStats, ProcessStats as WslProcessStat};

pub struct WslScanResult {
    pub processes: Vec<WslProcessStat>,
    pub machine: MachineStats,
}

struct WslVisitorContext;

impl VisitorContext for WslVisitorContext {
    fn get(&self, _key: &str) -> Option<f32> {
        None
    }
}

impl ProcessVisitor for WslProcessStat {
    fn pid(&self) -> u32 {
        self.global_pid
    }

    fn name(&self) -> &str {
        self.name_str()
    }

    fn parent_pid(&self) -> u32 {
        0
    }

    fn exe_path(&self) -> Option<&str> {
        str::from_utf8(&self.name).ok()
    }

    fn visit(&self, _ctx: &dyn VisitorContext, visitor: &mut dyn FnMut(Field)) {
        // visitor(Field {
        //     id: "cpu",
        //     label: "CPU",
        //     value: FieldValue::Percent(self.cpu_percent),
        //     numeric: self.cpu_percent,
        //     threshold: 50.0,
        // });
        //
        // visitor(Field {
        //     id: "memory",
        //     label: "Memory",
        //     value: FieldValue::Bytes(self.rss_kb * 1024),
        //     numeric: self.rss_kb as f32 / (1024.0 * 1024.0),
        //     threshold: 1.0,
        // });
        //
        // visitor(Field {
        //     id: "disk_read",
        //     label: "Disk",
        //     value: FieldValue::Bytes(self.disk_read_bytes),
        //     numeric: self.disk_read_bytes as f32,
        //     threshold: 0.1,
        // });

        // visitor(Field {
        //     id: "disk_write",
        //     label: "Disk W",
        //     value: FieldValue::Bytes(self.disk_write_bytes),
        //     numeric: self.disk_write_bytes as f32,
        //     threshold: 0.1,
        // });

        // visitor(Field {
        //     id: "net_vsock",
        //     label: "vSock",
        //     value: FieldValue::Bytes(self.vsock_rx_bytes + self.vsock_tx_bytes),
        //     numeric: (self.vsock_rx_bytes + self.vsock_tx_bytes) as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "net_p9",
        //     label: "P9",
        //     value: FieldValue::Bytes(self.p9_rx_bytes + self.p9_tx_bytes),
        //     numeric: (self.p9_rx_bytes + self.p9_tx_bytes) as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "net_tcp",
        //     label: "TCP",
        //     value: FieldValue::Bytes(self.tcp_tx_remote_bytes + self.tcp_rx_remote_bytes),
        //     numeric: (self.tcp_tx_remote_bytes + self.tcp_rx_remote_bytes) as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "net_uds",
        //     label: "UDS",
        //     value: FieldValue::Bytes(self.uds_tx_bytes + self.uds_rx_bytes),
        //     numeric: (self.uds_tx_bytes + self.uds_rx_bytes) as f32,
        //     threshold: 0.1,
        // });
    }
}

impl ScanResult for WslScanResult {
    fn context(&self) -> Box<dyn VisitorContext> {
        Box::new(WslVisitorContext)
    }

    fn visit_processes(&self, visitor: &mut dyn FnMut(&dyn ProcessVisitor)) {
        for p in &self.processes {
            visitor(p);
        }
    }

    fn visit_stats(&self, visitor: &mut dyn FnMut(Field)) {
        let total = self.machine.total_kb.max(1);
        let ram_pct = (self.machine.used_kb as f32 / total as f32) * 100.0;

        // visitor(Field {
        //     id: "memory",
        //     label: "Memory",
        //     value: FieldValue::Percent(ram_pct),
        //     numeric: ram_pct,
        //     threshold: 1.0,
        // });
        //
        // visitor(Field {
        //     id: "net_vsock",
        //     label: "vSock",
        //     value: FieldValue::Bytes(self.machine.vsock_rx_bytes + self.machine.vsock_tx_bytes),
        //     numeric: (self.machine.vsock_rx_bytes + self.machine.vsock_tx_bytes) as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "net_p9",
        //     label: "P9",
        //     value: FieldValue::Bytes(self.machine.p9_rx_bytes + self.machine.p9_tx_bytes),
        //     numeric: (self.machine.p9_rx_bytes + self.machine.p9_tx_bytes) as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "net_tcp",
        //     label: "TCP",
        //     value: FieldValue::Bytes(
        //         self.machine.tcp_tx_remote_bytes + self.machine.tcp_rx_remote_bytes,
        //     ),
        //     numeric: (self.machine.tcp_tx_remote_bytes + self.machine.tcp_rx_remote_bytes) as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "net_uds",
        //     label: "UDS",
        //     value: FieldValue::Bytes(self.machine.uds_tx_bytes + self.machine.uds_rx_bytes),
        //     numeric: (self.machine.uds_tx_bytes + self.machine.uds_rx_bytes) as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "disk_read",
        //     label: "Disk R",
        //     value: FieldValue::Bytes(self.machine.disk_read_bytes),
        //     numeric: self.machine.disk_read_bytes as f32,
        //     threshold: 0.1,
        // });
        //
        // visitor(Field {
        //     id: "disk_write",
        //     label: "Disk W",
        //     value: FieldValue::Bytes(self.machine.disk_write_bytes),
        //     numeric: self.machine.disk_write_bytes as f32,
        //     threshold: 0.1,
        // });
    }
}
