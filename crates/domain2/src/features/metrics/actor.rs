use app_contracts2::features::agents::WindowsReportMessage;
use app_contracts2::features::metrics::{MetricsMsg, MetricsPort};
use guinea::widgets::chart::RingSeries;
use guinea_core::actor::ManagedActor;
use guinea_macros::{actor_manifest, handler};

pub struct MetricsActor<P: MetricsPort> {
    ui_port: P,
    cpu_history: RingSeries,
    memory_history: RingSeries,
}

impl<P: MetricsPort> std::fmt::Debug for MetricsActor<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsActor")
            .field("cpu_points", &self.cpu_history.len())
            .field("memory_points", &self.memory_history.len())
            .finish()
    }
}

impl<P: MetricsPort> MetricsActor<P> {
    pub fn new(ui_port: P) -> Self {
        Self {
            ui_port,
            cpu_history: RingSeries::new(120),
            memory_history: RingSeries::new(120),
        }
    }

    fn publish(&self) {
        self.ui_port.send(MetricsMsg::SetHistory {
            cpu: self.cpu_history.as_points(),
            memory: self.memory_history.as_points(),
        });
    }
}

#[actor_manifest]
impl<P: MetricsPort> ManagedActor for MetricsActor<P> {
    type Handlers = handlers!(@WindowsReportMessage);
}

#[handler]
fn on_windows_report<P: MetricsPort>(this: &mut MetricsActor<P>, msg: WindowsReportMessage) {
    let machine = &msg.0.machine;
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
    this.publish();
}
