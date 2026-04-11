use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize)]
pub struct Schema {
    pub ports: Vec<PortDef>,
    pub bindings: Vec<BindingDef>,
}

#[derive(Deserialize)]
pub struct PortDef {
    pub name: String,
    pub global: String,
    pub methods: Vec<MethodDef>,
}

#[derive(Deserialize)]
pub struct BindingDef {
    pub name: String,
    pub global: String,
    pub methods: Vec<BindingMethodDef>,
}

#[derive(Deserialize)]
pub struct MethodDef {
    pub name: String,
    pub is_manual: bool,
    pub global_override: Option<String>,
    pub slint_name: Option<String>,
    pub args: Vec<ArgDef>,
}

#[derive(Deserialize)]
pub struct BindingMethodDef {
    pub name: String,
    pub is_manual: bool,
    pub tracing_skip: bool,
    pub tracing_target: Option<String>,
    pub slint_name: Option<String>,
    pub handler_args: Vec<ArgDef>,
}

#[derive(Deserialize)]
pub struct ArgDef {
    pub name: String,
    pub ty: String,
}

pub fn load_schema() -> Schema {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let schema_path = PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .join("app-contracts")
        .join("contracts-schema.json");
    let json = fs::read_to_string(&schema_path)
        .expect("Failed to read contracts-schema.json. Did you build app-contracts first?");
    serde_json::from_str(&json).expect("Failed to parse schema JSON")
}
