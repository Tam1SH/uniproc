#[macro_use]
extern crate rust_i18n;

pub mod actor;
pub mod app;
pub mod icons;
pub mod l10n;
pub mod reactor;
pub mod shared_state;

i18n!("../domain/locales", fallback = "en");

pub use shared_state::SharedState;
