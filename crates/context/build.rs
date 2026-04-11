use std::fs;
use std::path::{Path, PathBuf};
use toml::{Table, Value};

fn main() {
    generate_l10n_port();
    generate_icons_registry();
    generate_trace_scopes();
}

fn generate_l10n_port() {
    let en_toml = Path::new("./locales/en.toml");
    println!("cargo:rerun-if-changed=./locales/");

    let content = fs::read_to_string(en_toml).expect("./locales/en.toml not found");
    let table: Table = content.parse().expect("Failed to parse en.toml");

    let mut flat_keys = Vec::new();
    collect_keys("", &table, &mut flat_keys);
    flat_keys.sort();

    let trait_methods = flat_keys
        .iter()
        .map(|key| {
            let method_name = key.replace(['.', '-'], "_");
            format!("    fn set_{method_name}(&self, value: String);")
        })
        .collect::<Vec<_>>()
        .join("\n");

    write_if_changed(
        Path::new("../app-contracts/src/features/l10n.rs"),
        &format!(
            r#"// Based on context/locales/en.toml
// AUTO-GENERATED — do not edit manually
use macros::slint_port;

#[slint_port(global = "L10n")]
pub trait L10nPort: Clone + 'static {{
{trait_methods}
}}
"#
        ),
    );
}

fn generate_trace_scopes() {
    let scopes_toml = Path::new("./trace-scopes.toml");
    let out = out_dir_file("trace_scopes.rs");

    println!("cargo:rerun-if-changed=./trace-scopes.toml");

    let content = fs::read_to_string(scopes_toml).expect("./trace-scopes.toml not found");
    let table: Table = content.parse().expect("Failed to parse trace-scopes.toml");

    let mut scopes = Vec::new();
    for root in ["ui", "context", "core"] {
        if let Some(Value::Table(root_table)) = table.get(root) {
            collect_scope_entries(vec![root.to_string()], root_table, &mut scopes);
        }
    }
    scopes.sort_by(|a, b| a.name.cmp(&b.name));

    let builtin_enable_scopes = policy_strings(&table, "enable_scopes");
    let builtin_disable_messages = policy_strings(&table, "disable_messages");
    let builtin_disable_targets = policy_strings(&table, "disable_targets");

    let consts = scopes
        .iter()
        .map(|entry| {
            let ctor = if entry.enabled_by_default { "new" } else { "disabled" };
            format!(
                "pub const {}: ScopeSpec = ScopeSpec::{}(\"{}\", ScopeKind::{});",
                entry.const_name, ctor, entry.name, entry.kind
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let all_scopes = scopes
        .iter()
        .map(|entry| entry.const_name.as_str())
        .collect::<Vec<_>>()
        .join(",\n    ");

    let builtin_enable_scopes = string_slice_literal(&builtin_enable_scopes);
    let builtin_disable_messages = string_slice_literal(&builtin_disable_messages);
    let builtin_disable_targets = string_slice_literal(&builtin_disable_targets);

    let generated = format!(
        r#"// AUTO-GENERATED from ./trace-scopes.toml
use app_core::trace::{{ScopeKind, ScopeSpec}};

{consts}

pub const ALL_SCOPES: &[ScopeSpec] = &[
    {all_scopes}
];

pub const BUILTIN_ENABLE_SCOPES: &[&str] = &{builtin_enable_scopes};
pub const BUILTIN_DISABLE_MESSAGES: &[&str] = &{builtin_disable_messages};
pub const BUILTIN_DISABLE_TARGETS: &[&str] = &{builtin_disable_targets};
"#
    );

    write_if_changed(&out, &generated);
}

fn generate_icons_registry() {
    let assets_dir = Path::new("../slint-adapter/ui/assets");
    let out = out_dir_file("icons.rs");

    println!("cargo:rerun-if-changed=../slint-adapter/ui/assets");

    let mut entries: Vec<String> = fs::read_dir(assets_dir)
        .expect("../slint-adapter/ui/assets not found")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name.ends_with(".svg"))
        .collect();
    entries.sort();

    let arms = entries
        .iter()
        .map(|filename| {
            let name = filename.trim_end_matches(".svg");
            let asset_path = assets_dir
                .join(filename)
                .canonicalize()
                .expect("icon asset should be canonicalizable");
            let asset_path = asset_path.to_string_lossy().replace('\\', "\\\\");
            format!("            \"{name}\" => include_bytes!(\"{asset_path}\"),")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let generated = format!(
        r#"// AUTO-GENERATED from ../slint-adapter/ui/assets
use slint::Image;

pub struct Icons;

impl Icons {{
    pub fn get(name: &str) -> Image {{
        let bytes: &[u8] = match name {{
{arms}
            _ => return Image::default(),
        }};
        Image::load_from_svg_data(bytes).unwrap_or_default()
    }}
}}
"#
    );

    write_if_changed(&out, &generated);
}

#[derive(Clone)]
struct ScopeEntry {
    name: String,
    const_name: String,
    kind: &'static str,
    enabled_by_default: bool,
}

fn collect_scope_entries(path: Vec<String>, table: &Table, acc: &mut Vec<ScopeEntry>) {
    for (key, value) in table {
        let mut next_path = path.clone();
        next_path.push(key.replace('-', "_"));

        match value {
            Value::Table(sub_table) => collect_scope_entries(next_path, sub_table, acc),
            Value::Boolean(enabled_by_default) => {
                let name = next_path.join(".");
                let kind = next_path
                    .first()
                    .map(|segment| scope_kind(segment))
                    .unwrap_or("Core");
                let const_name = next_path
                    .iter()
                    .map(|segment| segment.to_ascii_uppercase())
                    .collect::<Vec<_>>()
                    .join("_");
                acc.push(ScopeEntry { name, const_name, kind, enabled_by_default: *enabled_by_default });
            }
            other => panic!("Unexpected trace scope entry for {:?}: {other:?}", next_path),
        }
    }
}

fn policy_strings(table: &Table, key: &str) -> Vec<String> {
    table
        .get("policy")
        .and_then(Value::as_table)
        .and_then(|policy| policy.get(key))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn string_slice_literal(values: &[String]) -> String {
    let items = values.iter().map(|v| format!("{v:?}")).collect::<Vec<_>>().join(", ");
    format!("[{items}]")
}

fn scope_kind(root: &str) -> &'static str {
    match root {
        "ui" => "Ui",
        "context" => "Context",
        _ => "Core",
    }
}

fn collect_keys(prefix: &str, table: &Table, acc: &mut Vec<String>) {
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", prefix, key)
        };
        match value {
            Value::Table(sub_table) => collect_keys(&full_key, sub_table, acc),
            _ => acc.push(full_key),
        }
    }
}

fn out_dir_file(name: &str) -> PathBuf {
    PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR should be set")).join(name)
}

fn write_if_changed(path: &Path, content: &str) {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing != content {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, content).ok();
    }
}
