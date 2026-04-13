pub mod collector;
pub mod slint_parser;

// Re-export the codegen schema types at the crate root for convenience.
pub use collector::{
    ArgDef, BindingDef, BindingMethodDef, DtoDef, DtoField, MethodDef, PortDef, Schema,
    load_schema,
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
