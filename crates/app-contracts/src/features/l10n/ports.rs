// Based on context/locales/en.toml
// AUTO-GENERATED — do not edit manually
use macros::slint_port;

#[slint_port(global = "L10n")]
pub trait L10nPort: Clone + 'static {
    fn set_environments(&self, value: String);
    fn set_error_connection_lost(&self, value: String);
    fn set_perfomance_tab(&self, value: String);
    fn set_search_placeholder(&self, value: String);
    fn set_services_description(&self, value: String);
    fn set_services_display_name(&self, value: String);
    fn set_services_group(&self, value: String);
    fn set_services_not_available(&self, value: String);
    fn set_services_not_running(&self, value: String);
    fn set_services_open_services(&self, value: String);
    fn set_services_path_to_executable(&self, value: String);
    fn set_services_pid(&self, value: String);
    fn set_services_properties(&self, value: String);
    fn set_services_restart(&self, value: String);
    fn set_services_service_name(&self, value: String);
    fn set_services_start(&self, value: String);
    fn set_services_stop(&self, value: String);
    fn set_settings_save_btn(&self, value: String);
}
