use rust_i18n::i18n;

pub mod caches;
pub mod l10n;
pub mod page_status;
pub mod settings;
pub mod trace;

i18n!("locales", fallback = "en");
