use std::collections::HashSet;
use std::path::Path;

use crate::types::{
    CommandRef, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ImportSymbol, ReexportEntry,
    ReexportKind,
};

use super::regexes::{
    regex_dynamic_import, regex_export_brace, regex_export_default, regex_export_named_decl,
    regex_import, regex_invoke_audio, regex_invoke_snake, regex_reexport_named,
    regex_reexport_star, regex_safe_invoke, regex_side_effect_import, regex_tauri_invoke,
};
use super::resolvers::resolve_reexport_target;
use super::{brace_list_to_names, offset_to_line};

fn parse_import_symbols(raw: &str) -> Vec<ImportSymbol> {
    let mut symbols = Vec::new();
    let trimmed = raw.trim().trim_start_matches("type ").trim();

    // namespace import: * as Foo
    if trimmed.starts_with('*') {
        if let Some((_, alias)) = trimmed.split_once(" as ") {
            symbols.push(ImportSymbol {
                name: "*".to_string(),
                alias: Some(alias.trim().to_string()),
            });
        }
        return symbols;
    }

    // default import before comma or brace
    let mut default_done = false;
    if let Some((head, _)) = trimmed.split_once(['{', ',']) {
        let default_name = head.trim();
        if !default_name.is_empty() {
            symbols.push(ImportSymbol {
                name: default_name.to_string(),
                alias: None,
            });
            default_done = true;
        }
    } else if !trimmed.contains('{') && !trimmed.is_empty() {
        symbols.push(ImportSymbol {
            name: trimmed.to_string(),
            alias: None,
        });
        default_done = true;
    }

    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed[start..].find('}') {
            let inner = &trimmed[start + 1..start + end];
            for part in inner.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                if let Some((name, alias)) = part.split_once(" as ") {
                    symbols.push(ImportSymbol {
                        name: name.trim().to_string(),
                        alias: Some(alias.trim().to_string()),
                    });
                } else {
                    symbols.push(ImportSymbol {
                        name: part.to_string(),
                        alias: None,
                    });
                }
            }
        }
    }

    // If there was no brace/import list and no default parsed, try to treat the whole trimmed token as default
    if symbols.is_empty() && !default_done && !trimmed.is_empty() {
        symbols.push(ImportSymbol {
            name: trimmed.to_string(),
            alias: None,
        });
    }

    symbols
}

pub(crate) fn analyze_js_file(
    content: &str,
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
    relative: String,
) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative);
    let mut command_calls = Vec::new();
    let mut add_call = |name: &str, line: usize, generic: Option<String>| {
        command_calls.push(CommandRef {
            name: name.to_string(),
            exposed_name: None,
            line,
            generic_type: generic,
        });
    };
    for caps in regex_import().captures_iter(content) {
        let clause = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let source = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
        let mut entry = ImportEntry::new(source.clone(), ImportKind::Static);
        entry.symbols = parse_import_symbols(clause);
        entry.resolved_path = resolve_reexport_target(path, root, &source, extensions)
            .or_else(|| super::resolvers::resolve_js_relative(path, root, &source, extensions));
        entry.is_bare = !source.starts_with('.') && !source.starts_with('/');
        analysis.imports.push(entry);
    }
    for caps in regex_side_effect_import().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let mut entry = ImportEntry::new(source.clone(), ImportKind::SideEffect);
        entry.resolved_path = resolve_reexport_target(path, root, &source, extensions)
            .or_else(|| super::resolvers::resolve_js_relative(path, root, &source, extensions));
        entry.is_bare = !source.starts_with('.') && !source.starts_with('/');
        analysis.imports.push(entry);
    }

    for caps in regex_safe_invoke().captures_iter(content) {
        if let Some(cmd) = caps.name("cmd") {
            let line = offset_to_line(content, cmd.start());
            let generic = caps
                .name("generic")
                .map(|g| g.as_str().trim().to_string())
                .filter(|s| !s.is_empty());
            add_call(cmd.as_str(), line, generic);
        }
    }
    for caps in regex_tauri_invoke().captures_iter(content) {
        if let Some(cmd) = caps.get(1) {
            let line = offset_to_line(content, cmd.start());
            add_call(cmd.as_str(), line, None);
        }
    }
    for caps in regex_invoke_audio().captures_iter(content) {
        if let Some(cmd) = caps.name("cmd") {
            let line = offset_to_line(content, cmd.start());
            let generic = caps
                .name("generic")
                .map(|g| g.as_str().trim().to_string())
                .filter(|s| !s.is_empty());
            add_call(cmd.as_str(), line, generic);
        }
    }
    for caps in regex_invoke_snake().captures_iter(content) {
        if let Some(cmd) = caps.name("cmd") {
            let line = offset_to_line(content, cmd.start());
            let generic = caps
                .name("generic")
                .map(|g| g.as_str().trim().to_string())
                .filter(|s| !s.is_empty());
            add_call(cmd.as_str(), line, generic);
        }
    }

    for caps in regex_reexport_star().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let resolved = resolve_reexport_target(path, root, &source, extensions);
        analysis.reexports.push(ReexportEntry {
            source,
            kind: ReexportKind::Star,
            resolved,
        });
    }
    for caps in regex_reexport_named().captures_iter(content) {
        let raw_names = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let source = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
        let names = brace_list_to_names(raw_names);
        let resolved = resolve_reexport_target(path, root, &source, extensions);
        analysis.reexports.push(ReexportEntry {
            source,
            kind: ReexportKind::Named(names.clone()),
            resolved,
        });
    }

    for caps in regex_dynamic_import().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        analysis.dynamic_imports.push(source);
    }

    for caps in regex_export_named_decl().captures_iter(content) {
        let name = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        if !name.is_empty() {
            let line = caps.get(1).map(|m| offset_to_line(content, m.start()));
            analysis
                .exports
                .push(ExportSymbol::new(name, "decl", "named", line));
        }
    }
    for caps in regex_export_default().captures_iter(content) {
        let name = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "default".to_string());
        let line = caps.get(0).map(|m| offset_to_line(content, m.start()));
        analysis
            .exports
            .push(ExportSymbol::new(name, "default", "default", line));
    }
    for caps in regex_export_brace().captures_iter(content) {
        let raw = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        for name in brace_list_to_names(raw) {
            let line = caps.get(1).map(|m| offset_to_line(content, m.start()));
            analysis
                .exports
                .push(ExportSymbol::new(name, "named", "named", line));
        }
    }
    for re in &analysis.reexports {
        if let ReexportKind::Named(names) = &re.kind {
            for name in names {
                analysis
                    .exports
                    .push(ExportSymbol::new(name.clone(), "reexport", "named", None));
            }
        }
    }

    analysis.command_calls = command_calls;
    analysis
}

#[cfg(test)]
mod tests {
    use super::analyze_js_file;
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn detects_commands_reexports_and_exports() {
        let content = r#"
import defaultThing from "./dep";
import type { Foo } from "./types";
import "./side.css";
export { bar } from "./reexports";
export * from "./star";
export const localValue = 1;
export default function MyComp() {}
export { namedA, namedB as aliasB };
const dyn = import("./lazy");
safeInvoke("cmd_safe");
invokeSnake("cmd_snake");
invoke("cmd_invoke");
safeInvoke<Foo.Bar>("cmd_generic_safe");
invokeSnake<MyType>("cmd_generic_snake");
invoke<Inline<Ok>>("cmd_generic_invoke");
invokeAudioCamel<Baz>("cmd_audio_generic");
        "#;

        let analysis = analyze_js_file(
            content,
            Path::new("src/app.tsx"),
            Path::new("src"),
            Some(&HashSet::from(["ts".to_string(), "tsx".to_string()])),
            "app.tsx".to_string(),
        );

        assert!(analysis
            .imports
            .iter()
            .any(|i| i.source == "./dep" && matches!(i.kind, crate::types::ImportKind::Static)));
        assert!(analysis
            .imports
            .iter()
            .any(|i| i.source == "./side.css"
                && matches!(i.kind, crate::types::ImportKind::SideEffect)));
        assert!(analysis.reexports.iter().any(|r| r.source == "./reexports"));
        assert!(analysis.reexports.iter().any(|r| r.source == "./star"));
        assert!(analysis.dynamic_imports.iter().any(|s| s == "./lazy"));

        let commands: Vec<_> = analysis
            .command_calls
            .iter()
            .map(|c| c.name.clone())
            .collect();
        assert!(commands.contains(&"cmd_safe".to_string()));
        assert!(commands.contains(&"cmd_snake".to_string()));
        assert!(commands.contains(&"cmd_invoke".to_string()));
        assert!(commands.contains(&"cmd_generic_safe".to_string()));
        assert!(commands.contains(&"cmd_generic_snake".to_string()));
        assert!(commands.contains(&"cmd_generic_invoke".to_string()));
        assert!(commands.contains(&"cmd_audio_generic".to_string()));

        let generics: Vec<_> = analysis
            .command_calls
            .iter()
            .filter_map(|c| c.generic_type.clone())
            .collect();
        assert!(generics.iter().any(|g| g.contains("Foo.Bar")));

        // exports should include defaults and named
        let export_names: Vec<_> = analysis.exports.iter().map(|e| e.name.clone()).collect();
        assert!(export_names.contains(&"localValue".to_string()));
        assert!(export_names.contains(&"MyComp".to_string()));
        assert!(export_names.contains(&"namedA".to_string()));
    }
}
