pub mod bandwidth_scanner;
pub mod net_provider;

pub mod phd_scanner;

use super::types::{ModelMachineStats, ProcessInfo, ProcessScanner, ScanResult};
use crate::scanner::windows::bandwidth_scanner::BandwidthScanner;
use crate::scanner::windows::net_provider::ProcessNetProvider;
use crate::scanner::windows::phd_scanner::PdhScanner;
use smallvec::SmallVec;
use sysinfo::{Networks, ProcessesToUpdate, System, Users};

pub struct WindowsScanner {
    sys: System,
    users: Users,
    networks: Networks,
    pdh: PdhScanner,
    bandwidth: BandwidthScanner,
    net_provider: ProcessNetProvider,
    cpu_count: usize,
}

impl WindowsScanner {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        let cpu_count = sys.cpus().len();

        Self {
            sys,
            users: sysinfo::Users::new_with_refreshed_list(),
            cpu_count: if cpu_count > 0 { cpu_count } else { 1 },
            pdh: PdhScanner,
            bandwidth: BandwidthScanner::new(),
            networks: Networks::new_with_refreshed_list(),
            net_provider: ProcessNetProvider::new(),
        }
    }
}
impl ProcessScanner for WindowsScanner {
    fn scan(&mut self) -> ScanResult {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.users.refresh();

        let mut total_rx = 0;
        let mut total_tx = 0;

        self.networks.refresh(true);

        for (_, data) in &self.networks {
            total_rx += data.received();
            total_tx += data.transmitted();
        }

        let current_traffic_bytes = total_rx + total_tx;

        let max_bandwidth_bytes = self.bandwidth.get_total_bandwidth();

        let net_percent = if max_bandwidth_bytes > 0 {
            ((current_traffic_bytes as f64 / max_bandwidth_bytes as f64) * 100.0) as f32
        } else {
            0.0
        };

        let mut active_pids: SmallVec<[u32; 512]> = SmallVec::new();

        let processes = self
            .sys
            .processes()
            .iter()
            .map(|(pid, proc)| {
                let pid_u32 = pid.as_u32();
                active_pids.push(pid_u32);

                let disk = proc.disk_usage();
                let net_usage = self.net_provider.get_usage(pid_u32);

                let user_name = proc
                    .user_id()
                    .and_then(|uid| self.users.get_user_by_id(uid))
                    .map(|u| u.name().to_string())
                    .unwrap_or_else(|| "N/A".to_string());

                ProcessInfo {
                    pid: pid_u32,
                    name: proc.name().to_string_lossy().to_string(),
                    cpu_usage: proc.cpu_usage() / self.cpu_count as f32,
                    memory_usage: proc.memory(),
                    exe_path: proc
                        .exe()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    command_line: proc
                        .cmd()
                        .into_iter()
                        .map(|s| s.to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_string(),

                    net_usage,
                    disk_read: disk.read_bytes,
                    disk_write: disk.written_bytes,
                    parent_pid: proc.parent().map(|p| p.as_u32()).unwrap_or(0),
                    user_name,
                }
            })
            .collect();

        let disk_p = self.pdh.get_disk_percent();
        self.net_provider.cleanup(&active_pids);

        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();

        ScanResult {
            processes,
            stats: ModelMachineStats {
                cpu_percent: self.sys.global_cpu_usage(),
                ram_percent: (used_mem as f32 / total_mem as f32) * 100.0,
                disk_percent: disk_p,
                net_percent: net_percent.clamp(0.0, 100.0),
                total_memory: total_mem,
                net_total_bandwidth: max_bandwidth_bytes,
            },
        }
    }
}
