use app_core::actor::traits::Message;
use slint::SharedString;
use std::fmt::Debug;

pub const PROPERTIES_DIALOG_KEY: &str = "services-properties";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServiceEntryVm {
    pub name: SharedString,
    pub display_name: SharedString,
    pub pid: i32,
    pub status: SharedString,
    pub group: SharedString,
    pub description: SharedString,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServiceEntryDto {
    pub name: String,
    pub display_name: String,
    pub pid: i32,
    pub status: String,
    pub group: String,
    pub description: String,
}

impl From<ServiceEntryDto> for ServiceEntryVm {
    fn from(entry: ServiceEntryDto) -> Self {
        Self {
            status: entry.status.clone().into(),
            name: entry.name.clone().into(),
            pid: entry.pid,
            description: entry.description.clone().into(),
            group: entry.group.clone().into(),
            display_name: entry.display_name.clone().into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServiceSnapshot {
    pub services: Vec<ServiceEntryDto>,
}

impl Message for ServiceSnapshot {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ServiceActionKind {
    Start,
    Stop,
    Restart,
    Pause,
    Resume,
}
