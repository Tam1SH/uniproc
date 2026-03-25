pub mod bandwidth_scanner;
pub mod net_provider;
mod nt_scanner;
pub mod phd_scanner;
mod spotlight;
pub mod types;
pub mod visitor;

use crate::features::processes::scanner::windows::bandwidth_scanner::BandwidthScanner;
use crate::features::processes::scanner::windows::net_provider::ProcessNetProvider;
use crate::features::processes::scanner::windows::phd_scanner::PdhScanner;
use crate::features::processes::scanner::windows::types::{
    WindowsProcessStat, WindowsScanResult, WindowsStats,
};
use app_contracts::features::agents::ScanTick;
use app_core::actor::event_bus::EVENT_BUS;
use app_core::actor::traits::{Context, Handler, Message};
use slint::ComponentHandle;
use smallvec::SmallVec;
use sysinfo::{Networks, ProcessesToUpdate, System, Users};

#[derive(Clone)]
pub struct WindowsReport(pub WindowsScanResult);
impl Message for WindowsReport {}

pub struct WindowsScannerActor {
    sys: System,
    users: Users,
    networks: Networks,
    pdh: PdhScanner,
    bandwidth: BandwidthScanner,
    net_provider: ProcessNetProvider,
    cpu_count: usize,
}

impl WindowsScannerActor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        let cpu_count = sys.cpus().len();
        Self {
            sys,
            users: Users::new_with_refreshed_list(),
            cpu_count: if cpu_count > 0 { cpu_count } else { 1 },
            pdh: PdhScanner,
            bandwidth: BandwidthScanner::new(),
            networks: Networks::new_with_refreshed_list(),
            net_provider: ProcessNetProvider::new(),
        }
    }
}

impl<TWindow: ComponentHandle + 'static> Handler<ScanTick, TWindow> for WindowsScannerActor {
    fn handle(&mut self, _: ScanTick, ctx: &Context<Self, TWindow>) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.users.refresh();
        self.networks.refresh(true);

        let mut total_rx = 0u64;
        let mut total_tx = 0u64;
        for (_, data) in &self.networks {
            total_rx += data.received();
            total_tx += data.transmitted();
        }

        let max_bandwidth_bytes = self.bandwidth.get_total_bandwidth();
        let net_percent = if max_bandwidth_bytes > 0 {
            (((total_rx + total_tx) as f64 / max_bandwidth_bytes as f64) * 100.0) as f32
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
                    .map(|u| u.name().to_string());
                WindowsProcessStat {
                    pid: pid_u32,
                    name: proc.name().to_string_lossy().to_string(),
                    cpu_usage: proc.cpu_usage() / self.cpu_count as f32,
                    memory_usage: proc.memory(),
                    exe_path: proc.exe().map(|p| p.to_string_lossy().to_string()),
                    command_line: Some(
                        proc.cmd()
                            .iter()
                            .map(|s| s.to_string_lossy().to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                    net_usage: Some(net_usage),
                    disk_read: Some(disk.read_bytes),
                    disk_write: Some(disk.written_bytes),
                    parent_pid: proc.parent().map(|p| p.as_u32()).unwrap_or(0),
                    user_name,
                }
            })
            .collect();

        let disk_percent = self.pdh.get_disk_percent();
        self.net_provider.cleanup(&active_pids);

        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();

        let result = WindowsScanResult {
            processes,
            stats: WindowsStats {
                cpu_percent: self.sys.global_cpu_usage(),
                ram_percent: (used_mem as f32 / total_mem as f32) * 100.0,
                disk_percent,
                net_percent: net_percent.clamp(0.0, 100.0),
                net_total_bandwidth: max_bandwidth_bytes,
                total_memory: total_mem,
            },
        };

        EVENT_BUS.with(|bus| bus.publish(WindowsReport(result)));
    }
}
