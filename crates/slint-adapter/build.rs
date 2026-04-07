mod slint_parser;

use std::fs;
use std::path::Path;
use toml::{Table, Value};

fn main() {
    download_missing_assets();
    generate_icons_slint();
    generate_slint_l10n();

    slint_parser::generate_globals_export(Path::new("ui"));

    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_include_paths(vec![
            std::path::PathBuf::from("ui"),
            std::path::PathBuf::from("ui/shared"),
            std::path::PathBuf::from("ui/components"),
        ]);

    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}

fn collect_string_entries(prefix: &str, table: &Table, acc: &mut Vec<(String, String)>) {
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", prefix, key)
        };

        match value {
            Value::Table(sub_table) => collect_string_entries(&full_key, sub_table, acc),
            Value::String(text) => acc.push((full_key, text.clone())),
            other => acc.push((full_key, other.to_string())),
        }
    }
}

fn escape_slint_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn generate_slint_l10n_adapter(flat_keys: &[String]) {
    let out_file = Path::new("src/adapters/l10n.rs");

    let methods = flat_keys
        .iter()
        .map(|key| {
            let name = key.replace(['.', '-'], "_");
            format!(
                "    fn set_{name}(&self, ui: &AppWindow, value: String) {{\n\
                 \t    ui.global::<L10n>().set_{name}(value.into());\n\
                 \t}}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let content = format!(
        r#"// AUTO-GENERATED — do not edit manually
use crate::{{AppWindow, L10n}};
use context::l10n::L10nPort;
use macros::ui_adapter;
use slint::ComponentHandle;

#[derive(Clone)]
pub struct SlintL10nPort {{
    ui: slint::Weak<AppWindow>,
}}

impl SlintL10nPort {{
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {{
        Self {{ ui }}
    }}
}}

#[ui_adapter]
impl L10nPort for SlintL10nPort {{
{methods}
}}
"#
    );

    write_if_changed(out_file, &content);
}

fn generate_slint_l10n() {
    let en_toml = Path::new("../context/locales/en.toml");
    let out_file = Path::new("ui/shared/localization.slint");

    println!("cargo:rerun-if-changed=../context/locales/");

    let content = fs::read_to_string(en_toml).expect("../context/locales/en.toml not found");
    let table: Table = content.parse().expect("Failed to parse en.toml");

    let mut flat_entries = Vec::new();
    collect_string_entries("", &table, &mut flat_entries);
    flat_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let flat_keys = flat_entries
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();

    let properties = flat_entries
        .iter()
        .map(|(key, value)| {
            let slint_name = key.replace(['.', '_'], "-");
            let escaped = escape_slint_string(value);
            format!("    in property <string> {slint_name}: \"{escaped}\";")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let generated = format!(
        "// AUTO-GENERATED — do not edit manually\nexport global L10n {{\n{properties}\n}}\n"
    );

    write_if_changed(out_file, &generated);

    generate_slint_l10n_adapter(&flat_keys);
}

fn download_missing_assets() {
    let urls_file = Path::new("ui/assets/download.txt");
    let assets_dir = Path::new("ui/assets");
    println!("cargo:rerun-if-changed=ui/assets/download.txt");

    let Ok(content) = fs::read_to_string(urls_file) else {
        return;
    };

    for line in content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let Some(colon_pos) = line.find(':') else {
            continue;
        };
        let name = line[..colon_pos].trim();
        let url = line[colon_pos + 1..].trim();
        let dest = assets_dir.join(format!("{name}.svg"));

        if dest.exists() {
            continue;
        }

        let _ = std::process::Command::new("curl")
            .args(["-fsSL", "-o", dest.to_str().unwrap(), url])
            .output();
    }
}

fn generate_icons_slint() {
    let assets_dir = Path::new("ui/assets");
    let out_file = Path::new("ui/shared/icons.slint");
    println!("cargo:rerun-if-changed=ui/assets");

    let mut entries: Vec<String> = fs::read_dir(assets_dir)
        .expect("ui/assets not found")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".svg"))
        .collect();
    entries.sort();

    let properties = entries
        .iter()
        .map(|filename| {
            format!(
                "    out property <image> {}: @image-url(\"../assets/{filename}\");",
                filename.trim_end_matches(".svg").replace(['.', '_'], "-")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let generated = format!(
        "// AUTO-GENERATED — do not edit manually\nexport global Icons {{\n{properties}\n}}\n"
    );
    write_if_changed(out_file, &generated);
}

fn write_if_changed(path: &Path, content: &str) {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing != content {
        if let Some(p) = path.parent() {
            let _ = fs::create_dir_all(p);
        }
        fs::write(path, content).ok();
    }
}
