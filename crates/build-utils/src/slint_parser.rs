use i_slint_compiler::diagnostics::BuildDiagnostics;
use i_slint_compiler::parser::{SyntaxKind, identifier_text, parse_file};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn generate_globals_export(ui_dir: &Path) {
    let out_file = ui_dir.join("globals-export.slint");
    let mut imports_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut all_entities: Vec<String> = Vec::new();

    for entry in WalkDir::new(ui_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap_or_default() != "slint" {
            continue;
        }

        let file_name = path.file_name().unwrap().to_string_lossy();
        if file_name == "globals-export.slint" || file_name == "app-window.slint" {
            continue;
        }

        let mut diag = BuildDiagnostics::default();
        let root_node = match parse_file(path, &mut diag) {
            Some(node) => node,
            None => continue,
        };

        let mut file_entities = Vec::new();

        for node in root_node.descendants() {
            if node.kind() == SyntaxKind::ExportsList {
                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::StructDeclaration | SyntaxKind::EnumDeclaration => {
                            if let Some(id_node) = child
                                .children()
                                .find(|n| n.kind() == SyntaxKind::DeclaredIdentifier)
                            {
                                if let Some(name) = identifier_text(&id_node) {
                                    file_entities.push(name.to_string());
                                }
                            }
                        }

                        SyntaxKind::Component => {
                            let is_global = child
                                .first_token()
                                .map(|t| t.text() == "global")
                                .unwrap_or(false);

                            let name = child
                                .children()
                                .find(|n| n.kind() == SyntaxKind::DeclaredIdentifier)
                                .and_then(|n| identifier_text(&n));

                            if let Some(name) = name {
                                if is_global {
                                    file_entities.push(name.to_string());
                                } else {
                                    if let Some(element) =
                                        child.children().find(|n| n.kind() == SyntaxKind::Element)
                                    {
                                        let inherits_window = element
                                            .children()
                                            .find(|n| n.kind() == SyntaxKind::QualifiedName)
                                            .map(|n| {
                                                let base = n.text().to_string();
                                                let base = base.trim();
                                                base == "Window" || base == "Dialog"
                                            })
                                            .unwrap_or(false);

                                        if inherits_window {
                                            file_entities.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if !file_entities.is_empty() {
            let relative_path = path
                .strip_prefix(ui_dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");

            file_entities.sort();
            file_entities.dedup();
            all_entities.extend(file_entities.clone());
            imports_map.insert(relative_path, file_entities);
        }
    }

    if imports_map.is_empty() {
        return;
    }

    let mut generated = String::from("// AUTO-GENERATED — do not edit manually\n\n");

    for (slint_path, entities) in &imports_map {
        let entities_str = entities.join(", ");
        generated.push_str(&format!(
            "import {{ {} }} from \"{}\";\n",
            entities_str, slint_path
        ));
    }

    all_entities.sort();
    all_entities.dedup();
    generated.push_str("\nexport {\n    ");
    generated.push_str(&all_entities.join(",\n    "));
    generated.push_str("\n}\n");

    let existing = fs::read_to_string(&out_file).unwrap_or_default();
    if existing != generated {
        fs::write(&out_file, generated).ok();
    }
}
