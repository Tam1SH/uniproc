use guinea_core::Load;
use guinea_macros::reducer;
use std::rc::Rc;

use super::messages::{WslDispatch, WslMsg};
use super::model::{DistroRow, LinuxMachineSummary};

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
