use crate::AppWindow;
use app_contracts::features::l10n::L10nPort;
use macros::slint_port_adapter;

#[derive(Clone)]
pub struct SlintL10nPort {
    ui: slint::Weak<AppWindow>,
}

impl SlintL10nPort {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}

#[slint_port_adapter(window = AppWindow)]
impl L10nPort for SlintL10nPort {}
