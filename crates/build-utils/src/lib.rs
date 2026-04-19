#![cfg_attr(coverage, feature(coverage_attribute))]
#![cfg_attr(coverage, coverage(off))]
pub mod collector;

pub mod slint_parser;

pub use collector::{
    load_schema, ArgDef, BindingDef, BindingMethodDef, DtoDef, DtoField, MethodDef, PortDef, Schema,
};

use std::{fs, path::Path};

pub fn write_if_changed(path: &Path, content: &str) {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing != content {
        if let Some(p) = path.parent() {
            let _ = fs::create_dir_all(p);
        }
        fs::write(path, content).ok();
    }
}
