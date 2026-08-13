use std::rc::Rc;
use std::time::Instant;

use app_contracts::features::agents::RemoteScanResult;
use guinea_core::messages;
use app_contracts::features::wsl::{
    AgentPresence, DistroRow, LinuxMachineSummary, WslMsg, WslPort,
};
use guinea_core::actor::{AsyncContext, ManagedActor, Message};
use guinea_macros::{actor_manifest, handler};

use super::scanner;

#[derive(Clone, Copy)]
struct CpuSample {
    busy_ns: u64,
    at: Instant,
}

fn cpu_percent(previous: Option<CpuSample>, current: CpuSample, cpu_count: u32) -> Option<f32> {
    let previous = previous?;
    let cores = cpu_count.max(1) as f64;

    let elapsed_ns = current.at.duration_since(previous.at).as_nanos() as f64;
    if elapsed_ns <= 0.0 {
        return None;
    }

    let busy_ns = current.busy_ns.checked_sub(previous.busy_ns)? as f64;
    Some(((busy_ns / (elapsed_ns * cores)) * 100.0).clamp(0.0, 100.0) as f32)
}

#[derive(derive_more::Debug)]
pub struct WslActor<P: WslPort> {
    #[debug(skip)]
    ui_port: P,
    #[debug("{}", distros.len())]
    distros: Rc<[DistroRow]>,
    configured: String,
    #[debug("{}", machine.is_some())]
    machine: Option<LinuxMachineSummary>,
    #[debug(skip)]
    previous_cpu: Option<CpuSample>,
}

impl<P: WslPort> WslActor<P> {
    pub fn new(ui_port: P, configured: String) -> Self {
        Self {
            ui_port,
            distros: Rc::from(Vec::new()),
            configured,
            machine: None,
            previous_cpu: None,
        }
    }

    fn publish(&self) {
        self.ui_port.send(WslMsg::SetDistros(self.distros.clone()));
        self.ui_port.send(WslMsg::SetMachine(self.machine.clone()));
    }

    fn apply_presence(&mut self) {
        let answering = self.machine.is_some();
        let configured = self.configured.clone();

        let mut rows = self.distros.to_vec();
        for row in &mut rows {
            if row.name == configured {
                row.agent = if answering {
                    AgentPresence::Answering
                } else {
                    AgentPresence::Silent
                };
                row.metrics = self.machine.clone();
            }
        }
        self.distros = Rc::from(rows);
    }
}

messages! { RefreshDistros }

enum ScanResult {
    Distros(Vec<DistroRow>),
    Failed,
}
impl Message for ScanResult {}

#[actor_manifest]
impl<P: WslPort> ManagedActor for WslActor<P> {
    type Handlers = handlers!(@RefreshDistros, @ScanResult, @RemoteScanResult);
}

#[handler]
async fn handle_refresh<P: WslPort>(ctx: AsyncContext<WslActor<P>>, _: RefreshDistros) {
    match scanner::scan_distros() {
        Ok(distros) => ctx.send(ScanResult::Distros(distros)),
        Err(err) => {
            tracing::warn!(%err, "wsl distribution scan failed");
            ctx.send(ScanResult::Failed);
        }
    }
}

#[handler]
fn on_scan_result<P: WslPort>(this: &mut WslActor<P>, msg: ScanResult) {
    let ScanResult::Distros(distros) = msg else {
        return;
    };
    this.distros = Rc::from(distros);
    this.apply_presence();
    this.publish();
}

#[handler]
fn on_remote_scan<P: WslPort>(this: &mut WslActor<P>, msg: RemoteScanResult) {
    match msg {
        RemoteScanResult::Scan(scan) => {
            let sample = CpuSample {
                busy_ns: scan.machine.busy_ns,
                at: Instant::now(),
            };
            let percent = cpu_percent(this.previous_cpu, sample, scan.machine.cpu_count);
            this.previous_cpu = Some(sample);

            let m = &scan.machine;
            this.machine = Some(LinuxMachineSummary {
                cpu_percent: percent,
                memory_used_bytes: m.used_kb * 1024,
                memory_total_bytes: m.total_kb * 1024,
                disk_bytes: m.disk_read_bytes + m.disk_write_bytes,
                net_bytes: m.tcp_rx_remote_bytes
                    + m.tcp_tx_remote_bytes
                    + m.udp_rx_remote_bytes
                    + m.udp_tx_remote_bytes,
                process_count: scan.processes.len(),
                container_count: scan.docker_containers.len(),
            });
        }
        RemoteScanResult::Unavailable(_) => {
            this.machine = None;
            this.previous_cpu = None;
        }
    }
    this.apply_presence();
    this.publish();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sample(busy_ns: u64, at: Instant) -> CpuSample {
        CpuSample { busy_ns, at }
    }

    #[test]
    fn the_first_report_has_nothing_to_compare_against() {
        let now = Instant::now();
        assert_eq!(cpu_percent(None, sample(1_000, now), 4), None);
    }

    #[test]
    fn busy_time_is_divided_by_cores() {
        let start = Instant::now();
        let later = start + Duration::from_secs(1);

        let all_four_cores = cpu_percent(Some(sample(0, start)), sample(4_000_000_000, later), 4);
        assert!((all_four_cores.unwrap() - 100.0).abs() < 0.5);

        let one_of_four = cpu_percent(Some(sample(0, start)), sample(1_000_000_000, later), 4);
        assert!((one_of_four.unwrap() - 25.0).abs() < 0.5);
    }

    #[test]
    fn a_restarted_agent_counting_from_zero_yields_nothing() {
        let start = Instant::now();
        let later = start + Duration::from_secs(1);

        assert_eq!(
            cpu_percent(Some(sample(9_000_000_000, start)), sample(10, later), 4),
            None
        );
    }

    #[test]
    fn a_missing_core_count_is_treated_as_one() {
        let start = Instant::now();
        let later = start + Duration::from_secs(1);

        let percent = cpu_percent(Some(sample(0, start)), sample(1_000_000_000, later), 0);
        assert!((percent.unwrap() - 100.0).abs() < 0.5);
    }

    fn distro(name: &str, running: bool) -> DistroRow {
        DistroRow {
            name: name.to_string(),
            running,
            agent: AgentPresence::NotChecked,
            metrics: None,
        }
    }

    struct Recorder;
    impl WslPort for Recorder {
        fn send(&self, _msg: WslMsg) {}
    }

    fn actor_with(distros: Vec<DistroRow>) -> WslActor<Recorder> {
        let mut actor = WslActor::new(Recorder, "Ubuntu".to_string());
        actor.distros = Rc::from(distros);
        actor
    }

    #[test]
    fn only_the_configured_distribution_is_judged() {
        let mut actor = actor_with(vec![distro("Ubuntu", true), distro("Debian", true)]);
        actor.apply_presence();

        assert_eq!(actor.distros[0].agent, AgentPresence::Silent);
        assert_eq!(
            actor.distros[1].agent,
            AgentPresence::NotChecked,
            "a distribution nothing was attempted against is not agentless"
        );
    }

    #[test]
    fn a_report_makes_the_configured_distribution_answering() {
        let mut actor = actor_with(vec![distro("Ubuntu", true)]);
        actor.machine = Some(LinuxMachineSummary::default());
        actor.apply_presence();

        assert_eq!(actor.distros[0].agent, AgentPresence::Answering);
    }

    #[test]
    fn losing_the_agent_takes_the_figures_with_it() {
        let mut actor = actor_with(vec![distro("Ubuntu", true)]);
        actor.machine = Some(LinuxMachineSummary::default());
        actor.apply_presence();

        actor.machine = None;
        actor.apply_presence();

        assert_eq!(actor.distros[0].agent, AgentPresence::Silent);
    }
}
