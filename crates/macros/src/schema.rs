// Re-export all codegen schema types from build-utils — the single source of truth.
// This crate only adds `load_schema()` as a thin alias so that existing callers
// inside proc-macro code don't need to change their import paths.
pub use build_utils::collector::{
    ArgDef, BindingDef, BindingMethodDef, MethodDef, PortDef, Schema,
};

pub use build_utils::load_schema;
