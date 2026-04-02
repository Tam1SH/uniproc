// Based on ../locales/en.toml
// AUTO-GENERATED — do not edit manually
use rust_i18n::t;

pub trait L10nPort {
    fn set_environments(&self, value: String);
    fn set_error_connection_lost(&self, value: String);
    fn set_perfomance_tab(&self, value: String);
    fn set_search_placeholder(&self, value: String);
    fn set_services_open_services(&self, value: String);
    fn set_services_restart(&self, value: String);
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
        l10n.set_services_open_services(t!("services.open_services").to_string());
        l10n.set_services_restart(t!("services.restart").to_string());
        l10n.set_services_start(t!("services.start").to_string());
        l10n.set_services_stop(t!("services.stop").to_string());
        l10n.set_settings_save_btn(t!("settings_save_btn").to_string());
    }
}
