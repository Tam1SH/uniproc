// AUTO-GENERATED — do not edit manually
// Based on locales/en.toml

use crate::L10n;
use rust_i18n::t;

pub struct L10nManager;

impl L10nManager {
    pub fn apply_to_global(l10n: &L10n) {
        l10n.set_environments(t!("environments").to_string().into());
        l10n.set_error_connection_lost(t!("error_connection_lost").to_string().into());
        l10n.set_perfomance_tab(t!("perfomance_tab").to_string().into());
        l10n.set_search_placeholder(t!("search_placeholder").to_string().into());
        l10n.set_settings_save_btn(t!("settings_save_btn").to_string().into());
    }
}
