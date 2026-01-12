// Rust analyzer module structure
mod imports;
mod naming;
mod preprocess;
mod tauri;
mod usages;

// Re-export public items
pub use imports::CrateModuleMap;

// Imports from submodules
use imports::parse_rust_brace_names;
use naming::exposed_command_name;
use preprocess::{
    find_balanced_bracket, strip_cfg_attributes, strip_cfg_test_modules, strip_function_body_uses,
};
use tauri::{extract_plugin_identifier, extract_plugin_name};
use usages::{
    collect_identifier_mentions, collect_rust_signature_uses, extract_bare_function_calls,
    extract_function_arguments, extract_identifier_usages, extract_path_qualified_calls,
    extract_struct_field_types, extract_type_alias_qualified_paths,
};

// External imports
use super::offset_to_line;
use super::regexes::{
    regex_custom_command_fn, regex_event_const_rust, regex_event_emit_rust,
    regex_event_listen_rust, regex_rust_async_main_attr, regex_rust_fn_main, regex_rust_mod_decl,
    regex_rust_pub_use, regex_rust_use, regex_tauri_command_fn, regex_tauri_generate_handler,
    rust_pub_const_regexes, rust_pub_decl_regexes,
};
use crate::types::{
    CommandRef, EventRef, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ParamInfo,
    ReexportEntry, ReexportKind,
};

/// Extract params from content starting at a given position after function name.
/// Looks for `(...)` and parses the params inside.
fn extract_rust_fn_params(content: &str, after_name_pos: usize) -> Vec<ParamInfo> {
    // Find opening paren
    let rest = &content[after_name_pos..];
    let Some(paren_start) = rest.find('(') else {
        return Vec::new();
    };

    // Find matching closing paren (handle nested generics)
    let params_start = after_name_pos + paren_start + 1;
    let mut depth = 1;
    let mut end_pos = params_start;
    for (i, ch) in content[params_start..].char_indices() {
        match ch {
            '(' | '<' | '[' | '{' => depth += 1,
            ')' | '>' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    end_pos = params_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Vec::new();
    }

    let params_text = &content[params_start..end_pos];
    parse_rust_params(params_text)
}

/// Parse Rust function params like `x: i32, y: &str, z: Option<T>`.
/// Skips `self`, `&self`, `&mut self`.
fn parse_rust_params(params_text: &str) -> Vec<ParamInfo> {
    let mut params = Vec::new();
    let mut current = String::new();
    let mut depth: usize = 0;

    for ch in params_text.chars() {
        match ch {
            '<' | '(' | '[' | '{' => {
                depth += 1;
                current.push(ch);
            }
            '>' | ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                if let Some(p) = parse_single_rust_param(current.trim()) {
                    params.push(p);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    // Last param
    if !current.trim().is_empty()
        && let Some(p) = parse_single_rust_param(current.trim())
    {
        params.push(p);
    }

    params
}

/// Parse a single Rust param like `name: Type`.
fn parse_single_rust_param(param: &str) -> Option<ParamInfo> {
    let param = param.trim();
    if param.is_empty() {
        return None;
    }

    // Skip self variants
    if param == "self"
        || param == "&self"
        || param == "&mut self"
        || param == "mut self"
        || param.starts_with("self:")
    {
        return None;
    }

    // Parse `name: Type` or `mut name: Type`
    let param = param.strip_prefix("mut ").unwrap_or(param);

    if let Some((name, type_ann)) = param.split_once(':') {
        Some(ParamInfo {
            name: name.trim().to_string(),
            type_annotation: Some(type_ann.trim().to_string()),
            has_default: false, // Rust doesn't have default params
        })
    } else {
        // Just a name without type annotation (rare in Rust)
        Some(ParamInfo {
            name: param.to_string(),
            type_annotation: None,
            has_default: false,
        })
    }
}

pub(crate) fn analyze_rust_file(
    content: &str,
    relative: String,
    custom_command_macros: &[String],
) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative.clone());
    let mut event_emits = Vec::new();
    let mut event_listens = Vec::new();

    // Extract plugin identifier for Tauri plugins
    // Tries: 1) #![plugin(identifier = "...")] attribute
    //        2) tauri-plugin-XXX in path
    //        3) plugins/XXX/ in path
    let plugin_identifier = extract_plugin_identifier(content, &relative);

    // Strip #[cfg(test)] modules and inline function-body imports to avoid false positive cycles
    let production_content = strip_function_body_uses(&strip_cfg_test_modules(content));

    for caps in regex_rust_use().captures_iter(&production_content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        if source.is_empty() {
            continue;
        }

        let mut imp = ImportEntry::new(source.to_string(), ImportKind::Static);

        // Track crate-internal import patterns for dead code detection
        imp.raw_path = source.to_string();
        imp.is_crate_relative = source.starts_with("crate::");
        imp.is_super_relative = source.starts_with("super::");
        imp.is_self_relative = source.starts_with("self::");

        // Parse symbols from use statements like `use foo::{Bar, Baz}`
        if source.contains('{') && source.contains('}') {
            let mut parts = source.splitn(2, '{');
            let prefix = parts.next().unwrap_or("").trim().trim_end_matches("::");
            let braces = parts.next().unwrap_or("").trim_end_matches('}').trim();
            let names = parse_rust_brace_names(braces);
            for (original, exported) in names {
                imp.symbols.push(crate::types::ImportSymbol {
                    name: original.clone(),
                    alias: if original != exported {
                        Some(exported)
                    } else {
                        None
                    },
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
            for (_, exported) in names {
                analysis
                    .exports
                    .push(ExportSymbol::new(exported, "reexport", "named", None));
            }
        } else if raw.ends_with("::*") {
            analysis.reexports.push(ReexportEntry {
                source: raw.to_string(),
                kind: ReexportKind::Star,
                resolved: None,
            });
        } else {
            // pub use foo::bar as Baz;
            let (path_part, original_name, export_name) =
                if let Some((path, alias)) = raw.split_once(" as ") {
                    // Extract original name from path (last segment)
                    let orig = path.trim().rsplit("::").next().unwrap_or(path.trim());
                    (path.trim(), orig, alias.trim())
                } else {
                    let mut segments = raw.rsplitn(2, "::");
                    let name = segments.next().unwrap_or(raw).trim();
                    let _ = segments.next();
                    (raw, name, name) // No alias - same name
                };

            analysis.reexports.push(ReexportEntry {
                source: path_part.to_string(),
                kind: ReexportKind::Named(vec![(
                    original_name.to_string(),
                    export_name.to_string(),
                )]),
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

    // Parse `mod foo;` declarations as imports
    // This creates a dependency edge from the declaring file to the module file
    for caps in regex_rust_mod_decl().captures_iter(&production_content) {
        if let Some(mod_name) = caps.get(2) {
            let mod_name = mod_name.as_str();

            // Check for #[path = "..."] attribute (group 1)
            let custom_path = caps.get(1).map(|m| m.as_str().to_string());

            // Create import source in format: mod::name or mod::path::name for #[path]
            let source = if let Some(path) = &custom_path {
                // #[path = "foo.rs"] mod bar; -> mod::path:foo.rs
                format!("mod::path:{}", path)
            } else {
                // Regular mod foo; -> mod::foo
                format!("mod::{}", mod_name)
            };

            let mut imp = ImportEntry::new(source.clone(), ImportKind::Static);
            imp.raw_path = source;
            imp.is_crate_relative = false;
            imp.is_super_relative = false;
            imp.is_self_relative = false;
            // Mark as mod declaration - this is NOT an import edge for cycle detection
            imp.is_mod_declaration = true;

            // Add the module name as an imported symbol
            imp.symbols.push(crate::types::ImportSymbol {
                name: mod_name.to_string(),
                alias: None,
                is_default: false,
            });

            analysis.imports.push(imp);
        }
    }

    // public items - process with proper kind detection
    // rust_pub_decl_regexes() returns [fn, struct, enum, trait, type, union] in order
    let kinds = ["function", "struct", "enum", "trait", "type", "union"];
    for (regex, kind) in rust_pub_decl_regexes().iter().zip(kinds.iter()) {
        for caps in regex.captures_iter(content) {
            if let Some(name) = caps.get(1) {
                let line = offset_to_line(content, name.start());
                let name_str = name.as_str().to_string();

                // Extract params only for functions
                let params = if *kind == "function" {
                    extract_rust_fn_params(content, name.end())
                } else {
                    Vec::new()
                };

                analysis.exports.push(ExportSymbol::with_params(
                    name_str,
                    kind,
                    "named",
                    Some(line),
                    params,
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

    collect_rust_signature_uses(&production_content, &mut analysis);

    for caps in regex_event_const_rust().captures_iter(content) {
        if let (Some(name), Some(val)) = (caps.get(1), caps.get(2)) {
            analysis
                .event_consts
                .insert(name.as_str().to_string(), val.as_str().to_string());
        }
    }

    // Check if a token looks like a valid Tauri event name (not a Rust literal/keyword).
    // Returns true if the token should be filtered out (is NOT a valid event name).
    let is_invalid_event_identifier = |token: &str| -> bool {
        // Filter out Rust keywords and common literals
        const RUST_KEYWORDS: &[&str] = &[
            "true", "false", "None", "Some", "Ok", "Err", "self", "Self", "super", "crate",
        ];

        if RUST_KEYWORDS.contains(&token) {
            return true;
        }

        // Filter out tokens that look like module paths (contain ::)
        // These are likely enum variants or associated items, not event names
        if token.contains("::") {
            return true;
        }

        // Filter out single lowercase words that look like crate/module names
        // Valid event names typically use kebab-case, snake_case with underscores,
        // or have mixed case. Single lowercase words without separators are
        // more likely to be crate names (e.g., "gix", "tokio", "serde")
        if token.chars().all(|c| c.is_ascii_lowercase()) && token.len() <= 8 {
            return true;
        }

        // Filter out PascalCase identifiers without underscores or hyphens
        // These are likely type names (Mode, AppState, etc.) not event names.
        // Event names typically use kebab-case, snake_case, or SCREAMING_SNAKE_CASE.
        // A single PascalCase word is almost never an event name.
        if let Some(first) = token.chars().next()
            && first.is_ascii_uppercase()
        {
            // Check if it's a simple PascalCase identifier (no underscores/hyphens)
            let has_separator = token.contains('_') || token.contains('-');
            let is_all_caps = token.chars().all(|c| !c.is_ascii_lowercase());

            // Filter out if it's PascalCase without separators and not all caps
            if !has_separator && !is_all_caps {
                return true;
            }
        }

        false
    };

    let resolve_event = |token: &str| -> Option<(String, Option<String>, String, bool)> {
        let trimmed = token.trim();

        // Detect format! pattern - e.g., format!("event:{}", var) or &format!(...)
        if trimmed.contains("format!") {
            // Extract the format string pattern
            if let Some(start) = trimmed.find("format!(\"") {
                let after_paren = &trimmed[start + 9..]; // Skip 'format!("'
                if let Some(end) = after_paren.find('"') {
                    let pattern = &after_paren[..end];
                    // Replace {} placeholders with * for pattern matching
                    let normalized = pattern.replace("{}", "*").replace("{:?}", "*");
                    return Some((
                        normalized.clone(),
                        Some(format!("format!(\"{}\")", pattern)),
                        "dynamic".to_string(),
                        true, // is_dynamic
                    ));
                }
            }
            // Fallback for complex format patterns
            return Some((
                "dynamic-event:*".to_string(),
                Some(trimmed.to_string()),
                "dynamic".to_string(),
                true,
            ));
        }

        // String literals are always valid event names
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let name = trimmed
                .trim_start_matches(['"', '\''])
                .trim_end_matches(['"', '\''])
                .to_string();
            return Some((
                name,
                Some(trimmed.to_string()),
                "literal".to_string(),
                false,
            ));
        }

        // Check if it's a known const
        if let Some(val) = analysis.event_consts.get(trimmed) {
            return Some((
                val.clone(),
                Some(trimmed.to_string()),
                "const".to_string(),
                false,
            ));
        }

        // For identifiers, apply filtering
        if is_invalid_event_identifier(trimmed) {
            return None;
        }

        Some((
            trimmed.to_string(),
            Some(trimmed.to_string()),
            "ident".to_string(),
            false,
        ))
    };

    for caps in regex_event_emit_rust().captures_iter(content) {
        if let Some(target) = caps.name("target") {
            // Skip if resolve_event filters out this identifier
            if let Some((name, raw_name, source_kind, is_dynamic)) = resolve_event(target.as_str())
            {
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
                    is_dynamic,
                });
            }
        }
    }
    for caps in regex_event_listen_rust().captures_iter(content) {
        if let Some(target) = caps.name("target") {
            // Skip if resolve_event filters out this identifier
            if let Some((name, raw_name, source_kind, is_dynamic)) = resolve_event(target.as_str())
            {
                let line = offset_to_line(content, caps.get(0).map(|m| m.start()).unwrap_or(0));
                event_listens.push(EventRef {
                    raw_name,
                    name,
                    line,
                    kind: format!("listen_{}", source_kind),
                    awaited: false,
                    payload: None,
                    is_dynamic,
                });
            }
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
            let base_exposed_name = exposed_command_name(attr_raw, &fn_name);
            // Check if this is a plugin command (has root = "crate" attribute)
            let is_plugin_command = extract_plugin_name(attr_raw).is_some();

            // exposed_name is just the command name (without plugin prefix)
            // The plugin namespace is stored separately in plugin_name field
            // This matches frontend behavior: invoke('plugin:window|cmd') parses to name="cmd", plugin_name="window"
            let exposed_name = base_exposed_name;

            let line = offset_to_line(content, name.start());
            analysis.command_handlers.push(CommandRef {
                name: fn_name,
                exposed_name: Some(exposed_name),
                line,
                generic_type: None,
                payload: params,
                plugin_name: if is_plugin_command {
                    plugin_identifier.clone()
                } else {
                    None
                },
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
                let base_exposed_name = exposed_command_name(attr_raw, &fn_name);
                let is_plugin_command = extract_plugin_name(attr_raw).is_some();

                // exposed_name is just the command name (without plugin prefix)
                // The plugin namespace is stored separately in plugin_name field
                let exposed_name = base_exposed_name;

                let line = offset_to_line(content, name.start());
                analysis.command_handlers.push(CommandRef {
                    name: fn_name,
                    exposed_name: Some(exposed_name),
                    line,
                    generic_type: None,
                    payload: params,
                    plugin_name: if is_plugin_command {
                        plugin_identifier.clone()
                    } else {
                        None
                    },
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
                    .rfind(|s| !s.is_empty())
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

    // Detect path-qualified calls like `module::function()` or `Type::method()`
    // These are function calls via module path without explicit `use` import.
    // Pattern: `::<identifier>(` or `::<Identifier>{` or `::<Identifier><`
    // This catches: command::branch::handle(), OutputChannel::new(), etc.
    extract_path_qualified_calls(&production_content, &mut analysis.local_uses);

    // Detect type alias qualified paths like `io::Result`, `fs::File`, etc.
    // This handles cases where a module is imported but types from that module
    // are used via qualified paths (e.g., `use std::io; fn foo() -> io::Result<()>`)
    // This reduces false positives by ~15% for Rust codebases
    extract_type_alias_qualified_paths(content, &analysis.imports, &mut analysis.local_uses);

    // Detect bare function calls like `func_name(...)` in the same file
    // This catches local function calls without path qualification
    extract_bare_function_calls(&production_content, &mut analysis.local_uses);

    // Detect type names used in struct/enum field definitions
    // This catches types like Vec<DiffEdge>, Option<HubFile>, etc. that are used
    // as field types within the same file - they count as "local uses" of those types
    extract_struct_field_types(content, &mut analysis.local_uses);

    // Detect identifiers used in expressions and variable declarations
    // This catches const/static usage like `create_buffer::<BUFFER_SIZE>()`
    // and type usage in let bindings like `let x: SomeType = ...`
    // NOTE: Use full `content` here, not `production_content`, because we need to
    // scan function bodies for usages of exported symbols (constants, types, etc.)
    extract_identifier_usages(content, &mut analysis.local_uses);

    // Detect identifiers used as function arguments like `func(CONST_NAME)`
    // This catches const/static usage passed as arguments to functions
    extract_function_arguments(content, &mut analysis.local_uses);

    // Fallback: treat any identifier mention (excluding keywords) as a local use.
    // This plugs gaps where complex patterns (const tables, enum variants, nested types)
    // might not be caught by the structured extractors above.
    collect_identifier_mentions(content, &mut analysis.local_uses);

    // Remove standard library/common types from local uses to avoid false positives
    // in same-file usage checks.
    const SKIP_STD_TYPES: &[&str] = &[
        "Vec", "Option", "Result", "String", "HashMap", "Box", "Arc", "Rc",
    ];
    analysis
        .local_uses
        .retain(|u| !SKIP_STD_TYPES.contains(&u.as_str()));

    analysis
}
