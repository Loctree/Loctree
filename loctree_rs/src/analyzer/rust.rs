use crate::types::{
    CommandRef, EventRef, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ReexportEntry,
    ReexportKind,
};

use super::offset_to_line;
use super::regexes::{
    regex_event_const_rust, regex_event_emit_rust, regex_event_listen_rust,
    regex_rust_async_main_attr, regex_rust_fn_main, regex_rust_pub_use, regex_rust_use,
    regex_tauri_command_fn, regex_tauri_generate_handler, rust_pub_const_regexes,
    rust_pub_decl_regexes,
};

fn split_words_lower(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_lower = false;

    for ch in name.chars() {
        if ch == '_' || ch == '-' {
            if !current.is_empty() {
                words.push(current.to_lowercase());
                current.clear();
            }
            prev_lower = false;
            continue;
        }

        if ch.is_ascii_uppercase() && prev_lower && !current.is_empty() {
            words.push(current.to_lowercase());
            current.clear();
        }

        current.push(ch);
        prev_lower = ch.is_ascii_lowercase();
    }

    if !current.is_empty() {
        words.push(current.to_lowercase());
    }

    words.retain(|w| !w.is_empty());
    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn apply_rename_all(fn_name: &str, style: &str) -> String {
    let words = split_words_lower(fn_name);
    if words.is_empty() {
        return fn_name.to_string();
    }

    match style {
        "snake_case" => words.join("_"),
        "kebab-case" => words.join("-"),
        "camelCase" => {
            let mut out = words[0].clone();
            for w in words.iter().skip(1) {
                out.push_str(&capitalize(w));
            }
            out
        }
        "PascalCase" | "UpperCamelCase" => {
            let mut out = String::new();
            for w in &words {
                out.push_str(&capitalize(w));
            }
            out
        }
        "lowercase" => words.join("").to_lowercase(),
        "UPPERCASE" => words.join("").to_uppercase(),
        "SCREAMING_SNAKE_CASE" => words.join("_").to_uppercase(),
        _ => fn_name.to_string(),
    }
}

fn exposed_command_name(attr_raw: &str, fn_name: &str) -> String {
    let inner = attr_raw
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();

    if inner.is_empty() {
        return fn_name.to_string();
    }

    let mut rename: Option<String> = None;
    let mut rename_all: Option<String> = None;

    for part in inner.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((key, raw_val)) = trimmed.split_once('=') {
            let key = key.trim();
            let val = raw_val.trim().trim_matches(['"', '\'']).to_string();
            if val.is_empty() {
                continue;
            }
            if key == "rename" {
                rename = Some(val);
            } else if key == "rename_all" {
                rename_all = Some(val);
            }
        }
    }

    if let Some(explicit) = rename {
        return explicit;
    }
    if let Some(style) = rename_all {
        return apply_rename_all(fn_name, &style);
    }

    fn_name.to_string()
}

fn parse_rust_brace_names(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed == "self" {
                return None;
            }
            if let Some((_, alias)) = trimmed.split_once(" as ") {
                Some(alias.trim().to_string())
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

pub(crate) fn analyze_rust_file(content: &str, relative: String) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative);
    let mut event_emits = Vec::new();
    let mut event_listens = Vec::new();
    for caps in regex_rust_use().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        if !source.is_empty() {
            analysis
                .imports
                .push(ImportEntry::new(source.to_string(), ImportKind::Static));
        }
    }

    for caps in regex_rust_pub_use().captures_iter(content) {
        let raw = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        if raw.is_empty() {
            continue;
        }

        if raw.contains('{') && raw.contains('}') {
            let mut parts = raw.splitn(2, '{');
            let _prefix = parts.next().unwrap_or("").trim().trim_end_matches("::");
            let braces = parts.next().unwrap_or("").trim_end_matches('}').trim();
            let names = parse_rust_brace_names(braces);
            analysis.reexports.push(ReexportEntry {
                source: raw.to_string(),
                kind: ReexportKind::Named(names.clone()),
                resolved: None,
            });
            for name in names {
                analysis
                    .exports
                    .push(ExportSymbol::new(name, "reexport", "named", None));
            }
        } else if raw.ends_with("::*") {
            analysis.reexports.push(ReexportEntry {
                source: raw.to_string(),
                kind: ReexportKind::Star,
                resolved: None,
            });
        } else {
            // pub use foo::bar as Baz;
            let (path_part, export_name) = if let Some((path, alias)) = raw.split_once(" as ") {
                (path.trim(), alias.trim())
            } else {
                let mut segments = raw.rsplitn(2, "::");
                let name = segments.next().unwrap_or(raw).trim();
                let _ = segments.next();
                (raw, name)
            };

            analysis.reexports.push(ReexportEntry {
                source: path_part.to_string(),
                kind: ReexportKind::Named(vec![export_name.to_string()]),
                resolved: None,
            });
            analysis.exports.push(ExportSymbol::new(
                export_name.to_string(),
                "reexport",
                "named",
                None,
            ));
        }
    }

    // public items
    for regex in rust_pub_decl_regexes() {
        for caps in regex.captures_iter(content) {
            if let Some(name) = caps.get(1) {
                let line = offset_to_line(content, name.start());
                analysis.exports.push(ExportSymbol::new(
                    name.as_str().to_string(),
                    "decl",
                    "named",
                    Some(line),
                ));
            }
        }
    }

    for regex in rust_pub_const_regexes() {
        for caps in regex.captures_iter(content) {
            if let Some(name) = caps.get(1) {
                let line = offset_to_line(content, name.start());
                analysis.exports.push(ExportSymbol::new(
                    name.as_str().to_string(),
                    "decl",
                    "named",
                    Some(line),
                ));
            }
        }
    }

    for caps in regex_event_const_rust().captures_iter(content) {
        if let (Some(name), Some(val)) = (caps.get(1), caps.get(2)) {
            analysis
                .event_consts
                .insert(name.as_str().to_string(), val.as_str().to_string());
        }
    }
    let resolve_event = |token: &str| -> (String, Option<String>, String) {
        let trimmed = token.trim();
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let name = trimmed
                .trim_start_matches(['"', '\''])
                .trim_end_matches(['"', '\''])
                .to_string();
            return (name, Some(trimmed.to_string()), "literal".to_string());
        }
        if let Some(val) = analysis.event_consts.get(trimmed) {
            return (val.clone(), Some(trimmed.to_string()), "const".to_string());
        }
        (
            trimmed.to_string(),
            Some(trimmed.to_string()),
            "ident".to_string(),
        )
    };

    for caps in regex_event_emit_rust().captures_iter(content) {
        if let Some(target) = caps.name("target") {
            let (name, raw_name, source_kind) = resolve_event(target.as_str());
            let payload = caps
                .name("payload")
                .map(|p| p.as_str().trim().trim_end_matches(')').trim().to_string())
                .filter(|s| !s.is_empty());
            let line = offset_to_line(content, caps.get(0).map(|m| m.start()).unwrap_or(0));
            event_emits.push(EventRef {
                raw_name,
                name,
                line,
                kind: format!("emit_{}", source_kind),
                awaited: false,
                payload,
            });
        }
    }
    for caps in regex_event_listen_rust().captures_iter(content) {
        if let Some(target) = caps.name("target") {
            let (name, raw_name, source_kind) = resolve_event(target.as_str());
            let line = offset_to_line(content, caps.get(0).map(|m| m.start()).unwrap_or(0));
            event_listens.push(EventRef {
                raw_name,
                name,
                line,
                kind: format!("listen_{}", source_kind),
                awaited: false,
                payload: None,
            });
        }
    }

    for caps in regex_tauri_command_fn().captures_iter(content) {
        let attr_raw = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        let name_match = caps.get(2);
        let params = caps
            .name("params")
            .map(|p| p.as_str().trim().to_string())
            .filter(|s| !s.is_empty());

        if let Some(name) = name_match {
            let fn_name = name.as_str().to_string();
            let exposed_name = exposed_command_name(attr_raw, &fn_name);

            let line = offset_to_line(content, name.start());
            analysis.command_handlers.push(CommandRef {
                name: fn_name,
                exposed_name: Some(exposed_name),
                line,
                generic_type: None,
                payload: params,
            });
        }
    }

    // Tauri generate_handler! registrations
    for caps in regex_tauri_generate_handler().captures_iter(content) {
        if let Some(list) = caps.get(1) {
            let raw = list.as_str();
            for part in raw.split(',') {
                let ident = part.trim();
                if ident.is_empty() {
                    continue;
                }
                // Strip potential trailing generics or module qualifiers (foo::<T>, module::foo)
                let base = ident
                    .split(|c: char| c == ':' || c.is_whitespace() || c == '<')
                    .next()
                    .unwrap_or("")
                    .trim();
                if base.is_empty() {
                    continue;
                }
                // Basic Rust identifier check: starts with letter or '_', rest alphanumeric or '_'
                let mut chars = base.chars();
                if let Some(first) = chars.next() {
                    if !(first.is_ascii_alphabetic() || first == '_') {
                        continue;
                    }
                    if chars.any(|ch| !(ch.is_ascii_alphanumeric() || ch == '_')) {
                        continue;
                    }
                    if !analysis
                        .tauri_registered_handlers
                        .contains(&base.to_string())
                    {
                        analysis.tauri_registered_handlers.push(base.to_string());
                    }
                }
            }
        }
    }

    analysis.event_emits = event_emits;
    analysis.event_listens = event_listens;

    // Detect Rust entry points using proper regex (not contains - avoids false positives in comments/strings)
    if regex_rust_fn_main().is_match(content) {
        analysis.entry_points.push("main".to_string());
    }
    if regex_rust_async_main_attr().is_match(content)
        && !analysis.entry_points.contains(&"async_main".to_string())
    {
        analysis.entry_points.push("async_main".to_string());
    }

    analysis
}

#[cfg(test)]
mod tests {
    use super::analyze_rust_file;

    #[test]
    fn parses_exports_and_tauri_commands() {
        let content = r#"
use crate::something::Type;
pub use crate::foo::{Bar as Baz, Quux};
pub use crate::module::*;
pub struct MyStruct;
pub enum MyEnum { A }
pub const ANSWER: u32 = 42;
pub type Alias = u64;

#[tauri::command(rename = "exposed_cmd")]
pub async fn internal_name() {}

#[tauri::command(rename_all = "camelCase")]
pub fn snake_case_func() {}
        "#;

        let analysis = analyze_rust_file(content, "src/lib.rs".to_string());

        // check reexports and public items
        assert!(
            analysis
                .reexports
                .iter()
                .any(|r| r.source.contains("foo::{Bar as Baz"))
        );
        let export_names: Vec<_> = analysis.exports.iter().map(|e| e.name.clone()).collect();
        assert!(export_names.contains(&"MyStruct".to_string()));
        assert!(export_names.contains(&"MyEnum".to_string()));
        assert!(export_names.contains(&"ANSWER".to_string()));
        assert!(export_names.contains(&"Alias".to_string()));

        // Tauri commands with rename/rename_all
        let handlers: Vec<_> = analysis
            .command_handlers
            .iter()
            .map(|c| c.exposed_name.clone().unwrap_or_else(|| c.name.clone()))
            .collect();
        assert!(handlers.contains(&"exposed_cmd".to_string()));
        assert!(handlers.contains(&"snakeCaseFunc".to_string()));
    }
}
