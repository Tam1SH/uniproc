use guinea_macros::port;
use std::rc::Rc;

use super::model::{DistroRow, LinuxMachineSummary};

#[derive(Clone)]
pub enum WslMsg {
    SetDistros(Rc<[DistroRow]>),
    SetMachine(Option<LinuxMachineSummary>),
}

#[port]
pub trait WslPort: 'static {
    fn send(&self, msg: WslMsg);
}

