#![allow(unsafe_op_in_unsafe_fn)]

use rust_i18n::i18n;

i18n!("../../context/locales", fallback = "en");

pub mod features;
