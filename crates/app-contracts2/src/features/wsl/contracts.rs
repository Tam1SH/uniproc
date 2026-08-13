use guinea_core::Load;
use guinea_macros::{actions, port, reducer};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AgentPresence {
    Answering,
    Silent,
    NotChecked,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DistroRow {
    pub name: String,
    pub running: bool,
    pub agent: AgentPresence,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct LinuxMachineSummary {
    pub cpu_percent: Option<f32>,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub process_count: usize,
    pub container_count: usize,
}

#[derive(Clone)]
pub enum WslMsg {
    SetDistros(Rc<[DistroRow]>),
    SetMachine(Option<LinuxMachineSummary>),
}

#[port]
pub trait WslPort: 'static {
    fn send(&self, msg: WslMsg);
}

#[actions]
pub trait WslActions {}

#[derive(Clone, PartialEq, Debug)]
pub struct WslState {
    pub distros: Load<Rc<[DistroRow]>>,
    pub machine: Option<LinuxMachineSummary>,
}

impl Default for WslState {
    fn default() -> Self {
        Self {
            distros: Load::Loading,
            machine: None,
        }
    }
}

impl WslState {
    pub fn distros(&self) -> &[DistroRow] {
        self.distros.ready().map(|d| d.as_ref()).unwrap_or(&[])
    }

    pub fn total(&self) -> usize {
        self.distros().len()
    }

    pub fn running(&self) -> usize {
        self.distros().iter().filter(|d| d.running).count()
    }
}

#[reducer]
#[dispatch(WslActions)]
pub fn wsl_reducer(state: &mut WslState, msg: WslMsg) {
    match msg {
        WslMsg::SetDistros(distros) => {
            state.distros = Load::Ready(distros);
        }
        WslMsg::SetMachine(machine) => {
            state.machine = machine;
        }
    }
}
