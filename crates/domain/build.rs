use std::fs;
use std::path::Path;
use toml::{Table, Value};

fn main() {
    let en_toml = Path::new("./locales/en.toml");
    println!("cargo:rerun-if-changed=./locales/");

    let content = fs::read_to_string(en_toml).expect("./locales/en.toml not found");
    let table: Table = content.parse().expect("Failed to parse en.toml");

    let mut flat_keys = Vec::new();
    collect_keys("", &table, &mut flat_keys);
    flat_keys.sort();

    let apply_calls = flat_keys
        .iter()
        .map(|key| {
            let method_name = key.replace(['.', '-'], "_");
            format!("    port.set_{method_name}(t!(\"{key}\").to_string());")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let generated = format!(
        r#"// Based on locales/en.toml
// AUTO-GENERATED — do not edit manually
use app_contracts::features::l10n::L10nPort;
use rust_i18n::t;

pub fn apply<P: L10nPort>(port: &P) {{
{apply_calls}
}}
"#
    );

    write_if_changed(Path::new("src/features/l10n/apply.rs"), &generated);
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
