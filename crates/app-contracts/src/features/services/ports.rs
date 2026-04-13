use context::native_windows::slint_factory::SlintWindowRegistry;
use macros::slint_port;
use slint::SharedString;
use std::fmt::Debug;

use super::model::ServiceEntryVm;

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
