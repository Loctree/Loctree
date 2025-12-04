use crate::types::{
    CommandRef, EventRef, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ReexportEntry,
    ReexportKind,
};

use super::offset_to_line;
use super::regexes::{
    regex_custom_command_fn, regex_event_const_rust, regex_event_emit_rust,
    regex_event_listen_rust, regex_rust_async_main_attr, regex_rust_fn_main, regex_rust_pub_use,
    regex_rust_use, regex_tauri_command_fn, regex_tauri_generate_handler, rust_pub_const_regexes,
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
                // Extract the last segment for nested paths like `models::Visit`
                let last_segment = trimmed.rsplit("::").next().unwrap_or(trimmed).trim();
                if last_segment.is_empty() {
                    None
                } else {
                    Some(last_segment.to_string())
                }
            }
        })
        .collect()
}

/// Find the position of the closing `]` that balances the opening one.
/// Returns the index of that `]` in the input, or 0 if not found.
fn find_balanced_bracket(s: &str) -> usize {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                if depth == 0 {
                    return i;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    0
}

/// Strip `#[cfg(test)]` annotated modules from content to avoid false positive cycles.
/// This removes test-only imports from dependency analysis.
fn strip_cfg_test_modules(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut in_cfg_test_attr = false;

    while let Some(ch) = chars.next() {
        // Look for #[cfg(test)]
        if ch == '#' && chars.peek() == Some(&'[') {
            let pos = result.len();
            result.push(ch);

            // Collect the attribute
            let mut attr = String::from("#");
            for next in chars.by_ref() {
                attr.push(next);
                if next == ']' {
                    break;
                }
            }
            result.push_str(&attr[1..]); // Skip the '#' we already added

            // Check if it's #[cfg(test)] or #[cfg(all(..., test, ...))]
            let attr_inner = attr.trim();
            if attr_inner.starts_with("#[cfg(test)")
                || attr_inner.starts_with("#[cfg(all(") && attr_inner.contains("test")
            {
                in_cfg_test_attr = true;
                // Remove the attribute we just added
                result.truncate(pos);
            }
            continue;
        }

        // If we're after #[cfg(test)], look for `mod` keyword and skip the block
        if in_cfg_test_attr {
            result.push(ch);

            // Skip whitespace and look for `mod`
            if ch.is_whitespace() {
                continue;
            }

            // Check for 'mod' keyword
            if ch == 'm' {
                let mut keyword = String::from("m");
                while let Some(&next) = chars.peek() {
                    if next.is_alphabetic() || next == '_' {
                        keyword.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if keyword == "mod" {
                    // Skip until we find the opening brace
                    let mut found_brace = false;
                    for next in chars.by_ref() {
                        if next == '{' {
                            found_brace = true;
                            break;
                        }
                    }

                    if found_brace {
                        // Skip the entire block (handle nested braces)
                        let mut depth = 1;
                        for next in chars.by_ref() {
                            match next {
                                '{' => depth += 1,
                                '}' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    // Remove 'mod' we just added to result
                    result.truncate(result.len() - 1); // Remove the 'm'
                    in_cfg_test_attr = false;
                    continue;
                } else {
                    // Not a mod, push the keyword
                    result.push_str(&keyword[1..]); // Skip 'm' we already added
                    in_cfg_test_attr = false;
                }
            } else {
                in_cfg_test_attr = false;
            }
            continue;
        }

        result.push(ch);
    }
    result
}

/// Strip `#[...]` attributes from a string (handles nested brackets).
fn strip_cfg_attributes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '#' {
            // Check if next char is '['
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                let mut depth = 1;
                // Skip until we find the matching ']'
                for inner in chars.by_ref() {
                    match inner {
                        '[' => depth += 1,
                        ']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                continue;
            }
        }
        result.push(ch);
    }
    result
}

pub(crate) fn analyze_rust_file(
    content: &str,
    relative: String,
    custom_command_macros: &[String],
) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative);
    let mut event_emits = Vec::new();
    let mut event_listens = Vec::new();

    // Strip #[cfg(test)] modules to avoid false positive cycles from test-only imports
    let production_content = strip_cfg_test_modules(content);

    for caps in regex_rust_use().captures_iter(&production_content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        if source.is_empty() {
            continue;
        }

        let mut imp = ImportEntry::new(source.to_string(), ImportKind::Static);

        // Parse symbols from use statements like `use foo::{Bar, Baz}`
        if source.contains('{') && source.contains('}') {
            let mut parts = source.splitn(2, '{');
            let prefix = parts.next().unwrap_or("").trim().trim_end_matches("::");
            let braces = parts.next().unwrap_or("").trim_end_matches('}').trim();
            let names = parse_rust_brace_names(braces);
            for name in names {
                imp.symbols.push(crate::types::ImportSymbol {
                    name,
                    alias: None,
                    is_default: false,
                });
            }
            // Set source to the prefix for better matching
            imp.source = prefix.to_string();
        } else {
            // Single import like `use foo::Bar` or `use foo::*`
            if let Some(last_segment) = source.rsplit("::").next() {
                let last = last_segment.trim();
                if last == "*" {
                    // Star import - add "*" as symbol to trigger star_used check
                    imp.symbols.push(crate::types::ImportSymbol {
                        name: "*".to_string(),
                        alias: None,
                        is_default: false,
                    });
                    // Also set source to the prefix path
                    if let Some(prefix) = source.rsplit_once("::") {
                        imp.source = prefix.0.to_string();
                    }
                } else if !last.is_empty() && last != "self" {
                    imp.symbols.push(crate::types::ImportSymbol {
                        name: last.to_string(),
                        alias: None,
                        is_default: false,
                    });
                }
            }
        }

        analysis.imports.push(imp);
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

    // Custom command macros (from .loctree/config.toml)
    if let Some(custom_regex) = regex_custom_command_fn(custom_command_macros) {
        for caps in custom_regex.captures_iter(content) {
            let attr_raw = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            let name_match = caps.get(2);
            let params = caps
                .name("params")
                .map(|p| p.as_str().trim().to_string())
                .filter(|s| !s.is_empty());

            if let Some(name) = name_match {
                let fn_name = name.as_str().to_string();
                // Avoid duplicates if both #[tauri::command] and custom macro are used
                if analysis.command_handlers.iter().any(|c| c.name == fn_name) {
                    continue;
                }
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
    }

    // Tauri generate_handler! registrations
    // The generate_handler! macro may span multiple lines and contain #[cfg(...)] attributes.
    // We need to handle nested brackets by finding balanced pairs.
    for caps in regex_tauri_generate_handler().captures_iter(content) {
        if let Some(list_match) = caps.get(1) {
            let start_pos = list_match.start();
            // Find the actual end by matching balanced brackets from the start
            let remaining = &content[start_pos..];
            let balanced_end = find_balanced_bracket(remaining);
            let raw = if balanced_end > 0 {
                &remaining[..balanced_end]
            } else {
                list_match.as_str()
            };
            // Strip #[...] attributes from the handler list
            let cleaned = strip_cfg_attributes(raw);
            for part in cleaned.split(',') {
                let ident = part.trim();
                if ident.is_empty() {
                    continue;
                }
                // Strip potential trailing generics or module qualifiers (foo::<T>, module::foo)
                // Use .last() to get the function name from paths like commands::foo::bar
                let base = ident
                    .split(|c: char| c == ':' || c.is_whitespace() || c == '<')
                    .filter(|s| !s.is_empty())
                    .next_back()
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
    fn parses_generate_handler_multiline() {
        let content = r#"
tauri::generate_handler![
    // Comment
    alpha_status_command,
    commands::foo::bar,
    simple_cmd,
]
"#;
        let analysis = analyze_rust_file(content, "lib.rs".to_string(), &[]);
        assert!(
            analysis
                .tauri_registered_handlers
                .contains(&"alpha_status_command".to_string()),
            "Should contain alpha_status_command, got: {:?}",
            analysis.tauri_registered_handlers
        );
        assert!(
            analysis
                .tauri_registered_handlers
                .contains(&"bar".to_string()),
            "Should contain bar from module path"
        );
        assert!(
            analysis
                .tauri_registered_handlers
                .contains(&"simple_cmd".to_string()),
            "Should contain simple_cmd"
        );
    }

    #[test]
    fn parses_generate_handler_with_cfg_attributes() {
        let content = r#"
tauri::generate_handler![
    normal_command,
    #[cfg(target_os = "macos")]
    macos_only_command,
    #[cfg(target_os = "windows")]
    windows_only_command,
    another_normal_command,
]
"#;
        let analysis = analyze_rust_file(content, "lib.rs".to_string(), &[]);
        let handlers = &analysis.tauri_registered_handlers;
        assert!(
            handlers.contains(&"normal_command".to_string()),
            "Should contain normal_command, got: {:?}",
            handlers
        );
        assert!(
            handlers.contains(&"macos_only_command".to_string()),
            "Should contain macos_only_command even with #[cfg]"
        );
        assert!(
            handlers.contains(&"windows_only_command".to_string()),
            "Should contain windows_only_command even with #[cfg]"
        );
        assert!(
            handlers.contains(&"another_normal_command".to_string()),
            "Should contain another_normal_command after #[cfg] items"
        );
    }

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

        let analysis = analyze_rust_file(content, "src/lib.rs".to_string(), &[]);

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

    #[test]
    fn detects_fn_main_entry_point() {
        let content = r#"
fn main() {
    vista_lib::run()
}
"#;
        let analysis = analyze_rust_file(content, "main.rs".to_string(), &[]);
        assert!(
            analysis.entry_points.contains(&"main".to_string()),
            "Should detect fn main() as entry point, got: {:?}",
            analysis.entry_points
        );
    }

    #[test]
    fn detects_async_main_entry_point() {
        let content = r#"
#[tokio::main]
async fn main() {
    app::run().await
}
"#;
        let analysis = analyze_rust_file(content, "main.rs".to_string(), &[]);
        assert!(
            analysis.entry_points.contains(&"main".to_string()),
            "Should detect async fn main()"
        );
        assert!(
            analysis.entry_points.contains(&"async_main".to_string()),
            "Should detect #[tokio::main]"
        );
    }

    #[test]
    fn detects_custom_command_macros() {
        let content = r#"
#[api_cmd_tauri]
pub async fn custom_handler(state: State) -> Result<(), Error> {}

#[gitbutler_command]
pub fn another_custom() {}

#[tauri::command]
pub fn standard_command() {}
"#;
        let custom_macros = vec!["api_cmd_tauri".to_string(), "gitbutler_command".to_string()];
        let analysis = analyze_rust_file(content, "commands.rs".to_string(), &custom_macros);

        let handler_names: Vec<_> = analysis.command_handlers.iter().map(|c| &c.name).collect();
        assert!(
            handler_names.contains(&&"custom_handler".to_string()),
            "Should detect #[api_cmd_tauri] command, got: {:?}",
            handler_names
        );
        assert!(
            handler_names.contains(&&"another_custom".to_string()),
            "Should detect #[gitbutler_command] command"
        );
        assert!(
            handler_names.contains(&&"standard_command".to_string()),
            "Should still detect #[tauri::command]"
        );
        assert_eq!(
            analysis.command_handlers.len(),
            3,
            "Should have exactly 3 handlers"
        );
    }

    #[test]
    fn custom_macros_avoid_duplicates() {
        let content = r#"
#[api_cmd_tauri]
#[tauri::command]
pub fn double_annotated() {}
"#;
        let custom_macros = vec!["api_cmd_tauri".to_string()];
        let analysis = analyze_rust_file(content, "commands.rs".to_string(), &custom_macros);

        assert_eq!(
            analysis.command_handlers.len(),
            1,
            "Should not duplicate when both macros present"
        );
        assert_eq!(
            analysis.command_handlers[0].name, "double_annotated",
            "Handler name should match"
        );
    }

    #[test]
    fn strip_cfg_test_excludes_test_imports() {
        // This is the exact pattern that caused false positive cycles
        let content = r#"
use serde::Serialize;

pub struct MyType {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::CommandBridge;

    #[test]
    fn my_test() {
        assert!(true);
    }
}
"#;
        let analysis = analyze_rust_file(content, "report.rs".to_string(), &[]);

        // Should only have the serde import, not the test-only imports
        assert_eq!(
            analysis.imports.len(),
            1,
            "Should have 1 import, got {:?}",
            analysis.imports
        );
        assert!(
            analysis.imports[0].source.contains("serde"),
            "Should import serde, got: {}",
            analysis.imports[0].source
        );
        // Should NOT contain the test-only import
        assert!(
            !analysis
                .imports
                .iter()
                .any(|i| i.source.contains("snapshot")),
            "Should NOT contain test-only snapshot import"
        );
    }

    #[test]
    fn strip_cfg_test_handles_nested_blocks() {
        let content = r#"
use crate::types::FileAnalysis;

pub fn production_fn() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::helper;

    fn nested_helper() {
        let block = { "nested braces" };
    }

    #[test]
    fn test_with_nested_braces() {
        if true {
            let x = { 1 };
        }
    }
}

pub fn another_production_fn() {}
"#;
        let analysis = analyze_rust_file(content, "module.rs".to_string(), &[]);

        // Should only have production imports
        assert_eq!(analysis.imports.len(), 1, "Should have 1 production import");
        assert!(
            analysis.imports[0].source.contains("types::FileAnalysis"),
            "Should have FileAnalysis import"
        );
    }

    #[test]
    fn strip_cfg_test_preserves_non_test_cfg() {
        let content = r#"
use crate::production::Type;

#[cfg(target_os = "macos")]
mod platform {
    use crate::platform_specific::MacType;
}

#[cfg(test)]
mod tests {
    use crate::test_only::TestHelper;
}
"#;
        let analysis = analyze_rust_file(content, "platform.rs".to_string(), &[]);

        // Should have production import and platform-specific import
        // but NOT the test-only import
        let sources: Vec<&str> = analysis.imports.iter().map(|i| i.source.as_str()).collect();
        assert!(
            sources.iter().any(|s| s.contains("production::Type")),
            "Should have production import"
        );
        assert!(
            sources
                .iter()
                .any(|s| s.contains("platform_specific::MacType")),
            "Should have platform-specific import (not #[cfg(test)])"
        );
        assert!(
            !sources.iter().any(|s| s.contains("test_only")),
            "Should NOT have test-only import"
        );
    }
}
