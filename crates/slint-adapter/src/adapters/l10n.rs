// AUTO-GENERATED — do not edit manually
use crate::{AppWindow, L10n};
use context::l10n::L10nPort;
use macros::ui_adapter;
use slint::ComponentHandle;

#[derive(Clone)]
pub struct SlintL10nPort {
    ui: slint::Weak<AppWindow>,
}

impl SlintL10nPort {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}

#[ui_adapter]
impl L10nPort for SlintL10nPort {
    fn set_environments(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_environments(value.into());
	}

    fn set_error_connection_lost(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_error_connection_lost(value.into());
	}

    fn set_perfomance_tab(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_perfomance_tab(value.into());
	}

    fn set_search_placeholder(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_search_placeholder(value.into());
	}

    fn set_services_description(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_description(value.into());
	}

    fn set_services_display_name(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_display_name(value.into());
	}

    fn set_services_group(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_group(value.into());
	}

    fn set_services_not_available(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_not_available(value.into());
	}

    fn set_services_not_running(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_not_running(value.into());
	}

    fn set_services_open_services(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_open_services(value.into());
	}

    fn set_services_path_to_executable(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_path_to_executable(value.into());
	}

    fn set_services_pid(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_pid(value.into());
	}

    fn set_services_properties(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_properties(value.into());
	}

    fn set_services_restart(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_restart(value.into());
	}

    fn set_services_service_name(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_service_name(value.into());
	}

    fn set_services_start(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_start(value.into());
	}

    fn set_services_stop(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_services_stop(value.into());
	}

    fn set_settings_save_btn(&self, ui: &AppWindow, value: String) {
	    ui.global::<L10n>().set_settings_save_btn(value.into());
	}
}
