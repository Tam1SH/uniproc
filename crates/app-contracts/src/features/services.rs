use app_core::actor::traits::Message;
use slint::SharedString;
use std::fmt::Debug;

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

pub trait ServicesUiPort: Debug + 'static {
    fn set_column_widths(&self, widths: Vec<(SharedString, u64)>);
    fn set_service_rows_window(&self, total_rows: usize, start: usize, rows: &[ServiceEntryVm]);
    fn set_loading(&self, loading: bool);
    fn set_selected_name(&self, name: SharedString);
    fn set_selected_service_details(
        &self,
        display_name: SharedString,
        pid: i32,
        status: SharedString,
        group: SharedString,
        description: SharedString,
    );
    fn set_sort_state(&self, field: SharedString, descending: bool);
    fn set_total_services_count(&self, count: usize);
    fn set_active_start_button(&self, flag: bool);
    fn set_active_stop_button(&self, flag: bool);
    fn set_active_restart_button(&self, flag: bool);
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
}
