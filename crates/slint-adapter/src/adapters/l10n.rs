use crate::{AppWindow, L10n};
use app_core::l10n::L10nPort;
use slint::ComponentHandle;

#[derive(Clone)]
pub struct SlintL10nPort {
    ui: slint::Weak<AppWindow>,
}

impl SlintL10nPort {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }

    fn with_l10n<F>(&self, f: F)
    where
        F: FnOnce(&L10n),
    {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui.global::<L10n>());
        }
    }
}

impl L10nPort for SlintL10nPort {
    fn set_environments(&self, value: String) {
        self.with_l10n(|l10n| l10n.set_environments(value.into()));
    }

    fn set_error_connection_lost(&self, value: String) {
        self.with_l10n(|l10n| l10n.set_error_connection_lost(value.into()));
    }

    fn set_perfomance_tab(&self, value: String) {
        self.with_l10n(|l10n| l10n.set_perfomance_tab(value.into()));
    }

    fn set_search_placeholder(&self, value: String) {
        self.with_l10n(|l10n| l10n.set_search_placeholder(value.into()));
    }

    fn set_settings_save_btn(&self, value: String) {
        self.with_l10n(|l10n| l10n.set_settings_save_btn(value.into()));
    }
}
