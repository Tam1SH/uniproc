use app_core::actor::traits::Message;
use context::native_windows::slint_factory::SlintWindowRegistry;
use slint::SharedString;
use std::fmt::Debug;

use macros::{slint_bindings, slint_port};

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

#[slint_port(global = "ServicesFeatureGlobal")]
pub trait UiServiceDetailsPort {
    fn set_selected_service_details(&self, entry: ServiceEntryVm);
    fn set_active_buttons(
        &self,
        start_button_active: bool,
        stop_button_active: bool,
        restart_button_active: bool,
    );
}

#[slint_port(global = "ServicesFeatureGlobal")]
pub trait UiServicesPort: Debug + UiServiceDetailsPort + 'static {
    #[manual]
    fn set_column_widths(&self, widths: Vec<(SharedString, u64)>);
    #[manual]
    fn set_service_rows_window(&self, total_rows: usize, start: usize, rows: &[ServiceEntryVm]);
    fn set_current_sort(&self, field: SharedString);
    fn set_current_sort_descending(&self, descending: bool);
    fn set_total_services_count(&self, total_services_count: usize);
}

#[slint_bindings(global = "ServicesFeatureGlobal")]
pub trait UiServicesBindings: 'static {
    #[manual]
    #[tracing(target = "name,kind")]
    fn on_service_action<F>(&self, handler: F)
    where
        F: Fn(SharedString, ServiceActionKind) + 'static;
    fn on_sort_by<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static;
    fn on_select_service<F>(&self, handler: F)
    where
        F: Fn(SharedString, i32) + 'static;
    fn on_rows_viewport_changed<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;
    fn on_column_resized<F>(&self, handler: F)
    where
        F: Fn(SharedString, f32) + 'static;
    fn on_open_system_services<F>(&self, handler: F)
    where
        F: Fn() + 'static;
    fn on_open_properties_window<F>(&self, handler: F)
    where
        F: Fn(ServiceEntryVm) + 'static;
}
