use std::fs;
use std::path::Path;
use toml::Table;

fn main() {
    generate_l10n_port();
}

fn generate_l10n_port() {
    let en_toml = Path::new("../domain/locales/en.toml");
    let out = Path::new("src/l10n.rs");

    println!("cargo:rerun-if-changed=../domain/locales/");

    let content = fs::read_to_string(en_toml).expect("../domain/locales/en.toml not found");
    let table: Table = content.parse().expect("Failed to parse en.toml");

    let mut keys: Vec<String> = table.keys().cloned().collect();
    keys.sort();

    let trait_methods = keys
        .iter()
        .map(|key| format!("    fn set_{key}(&self, value: String);"))
        .collect::<Vec<_>>()
        .join("\n");

    let apply_body = keys
        .iter()
        .map(|key| format!("        l10n.set_{key}(t!(\"{key}\").to_string());"))
        .collect::<Vec<_>>()
        .join("\n");

    let generated = format!(
        r#"// AUTO-GENERATED — do not edit manually
// Based on ../domain/locales/en.toml

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

fn write_if_changed(path: &Path, content: &str) {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing != content {
        fs::write(path, content).expect(&format!("Failed to write {:?}", path));
    }
}
