use std::fs;
use std::path::Path;
use toml::{Table, Value};

fn main() {
    generate_l10n_port();
}

fn generate_l10n_port() {
    let en_toml = Path::new("./locales/en.toml");
    let out = Path::new("src/l10n.rs");

    println!("cargo:rerun-if-changed=./locales/");

    let content = fs::read_to_string(en_toml).expect("./locales/en.toml not found");
    let table: Table = content.parse().expect("Failed to parse en.toml");

    let mut flat_keys = Vec::new();
    collect_keys("", &table, &mut flat_keys);
    flat_keys.sort();

    let trait_methods = flat_keys
        .iter()
        .map(|key| {
            let method_name = key.replace('.', "_").replace('-', "_");
            format!("    fn set_{method_name}(&self, value: String);")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let apply_body = flat_keys
        .iter()
        .map(|key| {
            let method_name = key.replace('.', "_").replace('-', "_");
            format!("        l10n.set_{method_name}(t!(\"{key}\").to_string());")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let generated = format!(
        r#"// Based on ../locales/en.toml
// AUTO-GENERATED — do not edit manually
use rust_i18n::t;

pub trait L10nPort {{
{trait_methods}
}}

pub struct L10nManager;

impl L10nManager {{
    pub fn apply_to_port<P: L10nPort>(l10n: &P) {{
{apply_body}
    }}
}}
"#
    );

    write_if_changed(out, &generated);
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

fn write_if_changed(path: &Path, content: &str) {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing != content {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, content).ok();
    }
}
