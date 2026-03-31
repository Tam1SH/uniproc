use crate::{AppWindow, L10n};
use app_core::l10n::L10nPort;
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

    fn set_settings_save_btn(&self, ui: &AppWindow, value: String) {
        ui.global::<L10n>().set_settings_save_btn(value.into());
    }
}
