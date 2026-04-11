// Based on context/locales/en.toml
// AUTO-GENERATED — do not edit manually
use app_contracts::features::l10n::L10nPort;
use rust_i18n::t;

pub fn apply<P: L10nPort>(port: &P) {
    port.set_environments(t!("environments").to_string());
    port.set_error_connection_lost(t!("error_connection_lost").to_string());
    port.set_perfomance_tab(t!("perfomance_tab").to_string());
    port.set_search_placeholder(t!("search_placeholder").to_string());
    port.set_services_description(t!("services.description").to_string());
    port.set_services_display_name(t!("services.display_name").to_string());
    port.set_services_group(t!("services.group").to_string());
    port.set_services_not_available(t!("services.not_available").to_string());
    port.set_services_not_running(t!("services.not_running").to_string());
    port.set_services_open_services(t!("services.open_services").to_string());
    port.set_services_path_to_executable(t!("services.path_to_executable").to_string());
    port.set_services_pid(t!("services.pid").to_string());
    port.set_services_properties(t!("services.properties").to_string());
    port.set_services_restart(t!("services.restart").to_string());
    port.set_services_service_name(t!("services.service_name").to_string());
    port.set_services_start(t!("services.start").to_string());
    port.set_services_stop(t!("services.stop").to_string());
    port.set_settings_save_btn(t!("settings_save_btn").to_string());
}
