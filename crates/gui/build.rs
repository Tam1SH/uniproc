use std::fs;
use std::path::Path;
use toml::Table;

fn main() {
    download_missing_assets();
    generate_icons_slint();
    generate_icons_rs();
    generate_l10n_files();

    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_include_paths(vec![
            std::path::PathBuf::from("ui"),
            std::path::PathBuf::from("ui/shared"),
            std::path::PathBuf::from("ui/components"),
        ]);

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        compile_windows_resources();
    }

    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}

fn generate_l10n_files() {
    let locales_dir = Path::new("locales");
    let en_toml = locales_dir.join("en.toml");
    let slint_out = Path::new("ui/shared/localization.slint");
    let rust_out = Path::new("src/shared/l10n.rs");

    println!("cargo:rerun-if-changed=locales/");

    let content = fs::read_to_string(&en_toml).expect("locales/en.toml not found");
    let table: Table = content.parse().expect("Failed to parse en.toml");

    let mut keys: Vec<String> = table.keys().cloned().collect();
    keys.sort();

    generate_slint_l10n(&keys, slint_out);

    generate_rust_l10n(&keys, rust_out);
}

fn generate_slint_l10n(keys: &[String], out_file: &Path) {
    let properties = keys
        .iter()
        .map(|key| {
            let slint_name = key.replace('_', "-");
            format!("    in property <string> {slint_name};")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        "// AUTO-GENERATED — do not edit manually\n\
         // Based on locales/en.toml\n\n\
         export global L10n {{\n{properties}\n}}\n"
    );

    write_if_changed(out_file, &content);
}

fn generate_rust_l10n(keys: &[String], out_file: &Path) {
    let apply_body = keys
        .iter()
        .map(|key| format!("    l10n.set_{key}(t!(\"{key}\").to_string().into());"))
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        r#"// AUTO-GENERATED — do not edit manually
// Based on locales/en.toml

use crate::L10n;
use rust_i18n::t;

pub struct L10nManager;

impl L10nManager {{
    pub fn apply_to_global(l10n: &L10n) {{
{apply_body}
    }}
}}
"#
    );

    write_if_changed(out_file, &content);
}

fn write_if_changed(path: &Path, content: &str) {
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing != content {
        fs::write(path, content).expect(&format!("Failed to write {:?}", path));
    }
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
        let Some((name, url)) = line
            .split_once(':')
            .map(|(n, rest)| {
                (
                    n.trim(),
                    format!("{}:{}", "", rest)
                        .trim_start_matches(':')
                        .to_string(),
                )
            })
            .or_else(|| {
                eprintln!("cargo:warning=Invalid line (expected <name>:<url>): {line}");
                None
            })
        else {
            continue;
        };

        let Some(colon_pos) = line.find(':') else {
            continue;
        };
        let name = line[..colon_pos].trim();
        let url = line[colon_pos + 1..].trim();

        if name.is_empty() || url.is_empty() {
            eprintln!("cargo:warning=Invalid line: {line}");
            continue;
        }

        let dest = assets_dir.join(format!("{name}.svg"));
        if dest.exists() {
            continue;
        }

        println!("cargo:warning=Downloading {name}.svg...");

        let output = std::process::Command::new("curl")
            .args(["-fsSL", "-o", dest.to_str().unwrap(), url])
            .output();

        match output {
            Ok(o) if o.status.success() => println!("cargo:warning=Downloaded {name}.svg"),
            Ok(o) => eprintln!(
                "cargo:warning=Failed to download {name}.svg: {}",
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => eprintln!("cargo:warning=curl failed: {e}"),
        }
    }
}
fn generate_icons_slint() {
    let assets_dir = Path::new("ui/assets");
    let out_file = Path::new("ui/shared/icons.slint");

    println!("cargo:rerun-if-changed=ui/assets");

    let mut entries: Vec<String> = fs::read_dir(assets_dir)
        .expect("ui/assets not found")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".svg") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    entries.sort();

    let properties = entries
        .iter()
        .map(|filename| {
            let prop_name = filename.trim_end_matches(".svg");
            format!("    out property <image> {prop_name}: @image-url(\"assets/{filename}\");")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        "// AUTO-GENERATED — do not edit manually\n\nexport global Icons {{\n{properties}\n}}\n"
    );

    let existing = fs::read_to_string(out_file).unwrap_or_default();
    if existing != content {
        fs::write(out_file, content).expect("Failed to write icons.slint");
    }
}

fn generate_icons_rs() {
    let assets_dir = Path::new("ui/assets");
    let out_file = Path::new("src/shared/icons.rs");

    let mut entries: Vec<String> = fs::read_dir(assets_dir)
        .expect("ui/assets not found")
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".svg") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    entries.sort();

    let arms = entries
        .iter()
        .map(|filename| {
            let name = filename.trim_end_matches(".svg");
            format!("            \"{name}\" => include_bytes!(\"../../ui/assets/{filename}\"),")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!(
        r#"// AUTO-GENERATED — do not edit manually

use slint::Image;

pub struct Icons;

impl Icons {{
    pub fn get(name: &str) -> Image {{
        let bytes: &[u8] = match name {{
{arms}
            _ => {{
                tracing::warn!(target: "internal", "Unknown icon: {{name}}");
                return Image::default();
            }}
        }};

        Image::load_from_svg_data(bytes).unwrap_or_else(|e| {{
            tracing::error!(target: "internal", "Failed to decode icon '{{name}}': {{e}}");
            Image::default()
        }})
    }}
}}
"#
    );

    let existing = fs::read_to_string(out_file).unwrap_or_default();
    if existing != content {
        fs::write(out_file, content).expect("Failed to write icons.rs");
    }
}

fn compile_windows_resources() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("ui/assets/icon.ico");
    res.compile().unwrap();
}
