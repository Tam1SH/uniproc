use app_core::actor::traits::Message;
use context::native_windows::slint_factory::SlintWindowRegistry;
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

pub trait ServicesWindowRegister {
    fn register(&self, registry: &SlintWindowRegistry);
}

pub trait ServiceDetailsPort {
    fn set_selected_service_details(&self, entry: ServiceEntryVm);
    fn set_active_buttons(&self, start_flag: bool, stop_flag: bool, restart_flag: bool);
}

pub trait ServicesUiPort: Debug + ServiceDetailsPort + 'static {
    fn set_column_widths(&self, widths: Vec<(SharedString, u64)>);
    fn set_service_rows_window(&self, total_rows: usize, start: usize, rows: &[ServiceEntryVm]);
    fn set_sort_state(&self, field: SharedString, descending: bool);
    fn set_total_services_count(&self, count: usize);
}

pub trait ServicesUiBindings: 'static {
    fn on_sort_by<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static;
    fn on_select_service<F>(&self, handler: F)
    where
        F: Fn(SharedString, usize) + 'static;
    fn on_viewport_changed<F>(&self, handler: F)
    where
        F: Fn(usize, usize) + 'static;
    fn on_column_resized<F>(&self, handler: F)
    where
        F: Fn(SharedString, f32) + 'static;
    fn on_service_action<F>(&self, handler: F)
    where
        F: Fn(SharedString, ServiceActionKind) + 'static;
    fn on_open_system_services<F>(&self, handler: F)
    where
        F: Fn() + 'static;
    fn on_open_properties_window<F>(&self, handler: F)
    where
        F: Fn(ServiceEntryVm) + 'static;
}
