use app_contracts::features::agents::WindowsReportMessage;
use app_contracts::features::metrics::{MetricsMsg, MetricsPort};
use app_contracts::features::processes::MachineSummary;
use guinea::widgets::chart::RingSeries;
use guinea_core::actor::ManagedActor;
use guinea_macros::{actor_manifest, handler};

#[derive(derive_more::Debug)]
pub struct MetricsActor<P: MetricsPort> {
    #[debug(skip)]
    ui_port: P,
    #[debug("{}", cpu_history.len())]
    cpu_history: RingSeries,
    #[debug("{}", memory_history.len())]
    memory_history: RingSeries,
    #[debug(skip)]
    machine: MachineSummary,
}

impl<P: MetricsPort> MetricsActor<P> {
    pub fn new(ui_port: P) -> Self {
        Self {
            ui_port,
            cpu_history: RingSeries::new(120),
            memory_history: RingSeries::new(120),
            machine: MachineSummary::default(),
        }
    }

    fn publish(&self) {
        self.ui_port.send(MetricsMsg::SetHistory {
            cpu: self.cpu_history.as_points(),
            memory: self.memory_history.as_points(),
            machine: self.machine.clone(),
        });
    }
}

#[actor_manifest]
impl<P: MetricsPort> ManagedActor for MetricsActor<P> {
    type Handlers = handlers!(@WindowsReportMessage);
}

#[handler]
fn on_windows_report<P: MetricsPort>(this: &mut MetricsActor<P>, msg: WindowsReportMessage) {
    let WindowsReportMessage::Report(report) = msg else {
        return;
    };
    let machine = &report.machine;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let memory_percent = if machine.total_physical_kb > 0 {
        (machine.used_physical_kb as f32 / machine.total_physical_kb as f32) * 100.0
    } else {
        0.0
    };
    this.cpu_history.push((timestamp, machine.cpu_percent));
    this.memory_history.push((timestamp, memory_percent));
    this.machine = MachineSummary {
        cpu_percent: machine.cpu_percent,
        cpu_current_mhz: machine.cpu_current_mhz,
        cpu_max_mhz: machine.cpu_max_mhz,
        memory_used_bytes: machine.used_physical_kb * 1024,
        memory_total_bytes: machine.total_physical_kb * 1024,
    };
    this.publish();
}
