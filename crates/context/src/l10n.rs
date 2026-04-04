// Based on ../locales/en.toml
// AUTO-GENERATED — do not edit manually
use rust_i18n::t;

pub trait L10nPort {
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

pub struct L10nManager;

impl L10nManager {
    pub fn apply_to_port<P: L10nPort>(l10n: &P) {
        l10n.set_environments(t!("environments").to_string());
        l10n.set_error_connection_lost(t!("error_connection_lost").to_string());
        l10n.set_perfomance_tab(t!("perfomance_tab").to_string());
        l10n.set_search_placeholder(t!("search_placeholder").to_string());
        l10n.set_services_description(t!("services.description").to_string());
        l10n.set_services_display_name(t!("services.display_name").to_string());
        l10n.set_services_group(t!("services.group").to_string());
        l10n.set_services_not_available(t!("services.not_available").to_string());
        l10n.set_services_not_running(t!("services.not_running").to_string());
        l10n.set_services_open_services(t!("services.open_services").to_string());
        l10n.set_services_path_to_executable(t!("services.path_to_executable").to_string());
        l10n.set_services_pid(t!("services.pid").to_string());
        l10n.set_services_properties(t!("services.properties").to_string());
        l10n.set_services_restart(t!("services.restart").to_string());
        l10n.set_services_service_name(t!("services.service_name").to_string());
        l10n.set_services_start(t!("services.start").to_string());
        l10n.set_services_stop(t!("services.stop").to_string());
        l10n.set_settings_save_btn(t!("settings_save_btn").to_string());
    }
}
