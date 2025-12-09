use crate::types::{
    CommandRef, EventRef, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ReexportEntry,
    ReexportKind, SignatureUse, SignatureUseKind,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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

fn regex_rust_pub_fn_signature() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^\s*pub\s*(?:\([^)]*\)\s*)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\((?P<params>[^)]*)\)\s*(?:->\s*(?P<ret>[^{;]+))?"#,
        )
        .expect("valid pub fn regex")
    })
}

fn extract_rust_type_tokens(segment: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut types = Vec::new();
    for token in segment.split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != ':') {
        if token.is_empty() {
            continue;
        }
        let first = token.chars().next().unwrap_or('_');
        let looks_like_type = first.is_ascii_uppercase() || token.contains("::");
        if !looks_like_type {
            continue;
        }
        const SKIP: &[&str] = &["Self", "String", "Vec", "Option", "Result"];
        if SKIP.contains(&token) {
            continue;
        }
        if seen.insert(token.to_string()) {
            types.push(token.to_string());
        }
    }
    types
}

fn collect_rust_signature_uses(content: &str, analysis: &mut FileAnalysis) {
    for caps in regex_rust_pub_fn_signature().captures_iter(content) {
        let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if fn_name.is_empty() {
            continue;
        }
        let params = caps.name("params").map(|m| m.as_str()).unwrap_or("");
        let ret = caps.name("ret").map(|m| m.as_str()).unwrap_or("");
        let line = offset_to_line(content, caps.get(0).map(|m| m.start()).unwrap_or(0));

        for ty in extract_rust_type_tokens(params) {
            analysis.signature_uses.push(SignatureUse {
                function: fn_name.to_string(),
                usage: SignatureUseKind::Parameter,
                type_name: ty.clone(),
                line: Some(line),
            });
            if !analysis.local_uses.contains(&ty) {
                analysis.local_uses.push(ty);
            }
        }
        for ty in extract_rust_type_tokens(ret) {
            analysis.signature_uses.push(SignatureUse {
                function: fn_name.to_string(),
                usage: SignatureUseKind::Return,
                type_name: ty.clone(),
                line: Some(line),
            });
            if !analysis.local_uses.contains(&ty) {
                analysis.local_uses.push(ty);
            }
        }
    }
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

/// Strip `use` statements from inside function bodies to avoid false positive cycles.
/// Inline imports (inside `fn`) are lazy-resolved and shouldn't contribute to module cycles.
fn strip_function_body_uses(content: &str) -> String {
    let mut result = String::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut fn_brace_depth: i32 = 0; // Track depth only when inside a function body

    while i < len {
        // Look for 'fn ' keyword (must be preceded by whitespace or start of file)
        if i + 3 <= len
            && &bytes[i..i + 3] == b"fn "
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
        {
            // Found 'fn ', now skip to the opening brace '{'
            result.push_str("fn ");
            i += 3;

            // Skip until we find '{' (the function body start)
            while i < len {
                let ch = bytes[i] as char;
                result.push(ch);
                i += 1;
                if ch == '{' {
                    fn_brace_depth = 1;
                    break;
                }
            }
            continue;
        }

        // If we're inside a function body
        if fn_brace_depth > 0 {
            let ch = bytes[i] as char;

            // Track brace depth
            match ch {
                '{' => {
                    fn_brace_depth += 1;
                    result.push(ch);
                    i += 1;
                }
                '}' => {
                    fn_brace_depth -= 1;
                    result.push(ch);
                    i += 1;
                }
                _ => {
                    // Check for 'use ' at start of statement (after whitespace/newline)
                    if i + 4 <= len
                        && &bytes[i..i + 4] == b"use "
                        && (i == 0
                            || bytes[i - 1] == b'\n'
                            || bytes[i - 1] == b' '
                            || bytes[i - 1] == b'\t'
                            || bytes[i - 1] == b'{')
                    {
                        // Skip the entire use statement until ';'
                        while i < len && bytes[i] != b';' {
                            i += 1;
                        }
                        // Skip the ';' too
                        if i < len {
                            i += 1;
                        }
                        // Skip trailing whitespace/newline
                        while i < len && (bytes[i] == b' ' || bytes[i] == b'\n') {
                            i += 1;
                        }
                    } else {
                        result.push(ch);
                        i += 1;
                    }
                }
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
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

/// Maps Rust module paths (like `crate::foo::bar`) to their corresponding file paths.
/// This is needed to resolve crate-internal imports for dead code detection.
#[derive(Debug, Clone)]
pub struct CrateModuleMap {
    /// Map from module path (e.g., "foo::bar") to file path relative to crate root
    modules: HashMap<String, PathBuf>,
    /// Crate root directory
    crate_root: PathBuf,
}

impl CrateModuleMap {
    /// Build a module map by scanning the crate starting from lib.rs or main.rs
    pub fn build(crate_root: &Path) -> std::io::Result<Self> {
        let mut map = CrateModuleMap {
            modules: HashMap::new(),
            crate_root: crate_root.to_path_buf(),
        };

        // Find the crate entry point (lib.rs or main.rs)
        let lib_rs = crate_root.join("src").join("lib.rs");
        let main_rs = crate_root.join("src").join("main.rs");

        let entry_point = if lib_rs.exists() {
            lib_rs
        } else if main_rs.exists() {
            main_rs
        } else {
            return Ok(map); // No entry point found, return empty map
        };

        // Parse the entry point to build the module tree
        map.scan_module(&entry_point, "")?;

        Ok(map)
    }

    /// Recursively scan a module file and register its submodules
    fn scan_module(&mut self, file_path: &Path, module_prefix: &str) -> std::io::Result<()> {
        // nosemgrep:rust.actix.path-traversal.tainted-path.tainted-path - file_path from internal module scan, not user input
        let content = std::fs::read_to_string(file_path)?;

        // Find all `mod foo;` declarations
        // Regex pattern: `pub mod name;` or `mod name;`
        let mod_regex = regex::Regex::new(r"(?m)^\s*(?:pub\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;")
            .expect("valid mod regex");

        for caps in mod_regex.captures_iter(&content) {
            if let Some(mod_name) = caps.get(1) {
                let mod_name = mod_name.as_str();
                let module_path = if module_prefix.is_empty() {
                    mod_name.to_string()
                } else {
                    format!("{}::{}", module_prefix, mod_name)
                };

                // Determine where to look for the module file based on the current file's structure:
                // 1. If current file is foo.rs -> look in foo/ directory
                // 2. If current file is foo/mod.rs -> look in foo/ directory
                // 3. Otherwise (lib.rs, main.rs) -> look in same directory

                let parent = file_path.parent().unwrap_or(file_path);
                let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                let search_dirs: Vec<PathBuf> = if file_name.ends_with(".rs")
                    && file_name != "mod.rs"
                    && file_name != "lib.rs"
                    && file_name != "main.rs"
                {
                    // For foo.rs, submodules can be in foo/ directory
                    let module_dir = parent.join(file_name.strip_suffix(".rs").unwrap());
                    vec![module_dir, parent.to_path_buf()]
                } else {
                    // For mod.rs, lib.rs, main.rs, submodules are in the same directory
                    vec![parent.to_path_buf()]
                };

                let mut found = false;
                for search_dir in search_dirs {
                    // Try to find the module file - Rust supports two conventions:
                    // 1. foo.rs (in search directory)
                    // 2. foo/mod.rs (subdirectory with mod.rs)
                    let mod_file = search_dir.join(format!("{}.rs", mod_name));
                    let mod_dir_file = search_dir.join(mod_name).join("mod.rs");

                    if mod_file.exists() {
                        // Register the module and scan it recursively
                        if let Ok(relative) = mod_file.strip_prefix(&self.crate_root) {
                            self.modules
                                .insert(module_path.clone(), relative.to_path_buf());
                        }
                        // Recursively scan the module file
                        let _ = self.scan_module(&mod_file, &module_path);
                        found = true;
                        break;
                    } else if mod_dir_file.exists() {
                        // Register the module directory and scan it recursively
                        if let Ok(relative) = mod_dir_file.strip_prefix(&self.crate_root) {
                            self.modules
                                .insert(module_path.clone(), relative.to_path_buf());
                        }
                        // Recursively scan the module file
                        let _ = self.scan_module(&mod_dir_file, &module_path);
                        found = true;
                        break;
                    }
                }

                if !found {
                    // Module file not found - this is okay, might be in a different workspace or conditional
                    // Just skip it
                }
            }
        }

        Ok(())
    }

    /// Resolve a module path to a file path.
    /// Handles:
    /// - `crate::foo::bar` - absolute from crate root
    /// - `super::bar` - relative to parent module
    /// - `self::bar` - relative to current module
    /// - `foo::bar` (no prefix) - relative to current module
    pub fn resolve_module_path(&self, from_file: &Path, import_path: &str) -> Option<PathBuf> {
        // Handle `crate::` prefix - absolute from crate root
        if let Some(rest) = import_path.strip_prefix("crate::") {
            return self.resolve_absolute(rest);
        }

        // Get the current module path from the file path
        let current_module = self.file_to_module_path(from_file)?;

        // Handle `super::` prefix - go up one level
        if let Some(rest) = import_path.strip_prefix("super::") {
            let parent_module = self.parent_module(&current_module)?;
            let target_path = if rest.is_empty() {
                parent_module
            } else {
                format!("{}::{}", parent_module, rest)
            };
            return self.resolve_absolute(&target_path);
        }

        // Handle `self::` prefix - same module
        if let Some(rest) = import_path.strip_prefix("self::") {
            let target_path = format!("{}::{}", current_module, rest);
            return self.resolve_absolute(&target_path);
        }

        // No prefix - try current module first, then parent modules (Rust 2015 style)
        // In Rust 2018+, bare imports must use crate:: prefix, but we're lenient for analysis

        // Build list of paths to try, from most specific to least specific
        let mut paths_to_try = Vec::new();

        if !current_module.is_empty() {
            paths_to_try.push(format!("{}::{}", current_module, import_path));

            // Walk up parent modules
            let mut current = current_module.to_string();
            while !current.is_empty() {
                if let Some(parent) = self.parent_module(&current) {
                    if parent.is_empty() {
                        paths_to_try.push(import_path.to_string());
                    } else {
                        paths_to_try.push(format!("{}::{}", parent, import_path));
                    }
                    current = parent;
                } else {
                    paths_to_try.push(import_path.to_string());
                    break;
                }
            }
        } else {
            // Already at root
            paths_to_try.push(import_path.to_string());
        }

        // Try each path in order
        for path in paths_to_try {
            if let Some(resolved) = self.resolve_absolute_exact(&path) {
                return Some(resolved);
            }
        }

        // If still not found, try with segment stripping (for type/function resolution)
        self.resolve_absolute(import_path)
    }

    /// Resolve an absolute module path with exact match only (no segment stripping)
    fn resolve_absolute_exact(&self, module_path: &str) -> Option<PathBuf> {
        self.modules.get(module_path).cloned()
    }

    /// Resolve an absolute module path (without crate:: prefix)
    /// This version strips segments to find containing modules (for type/function resolution)
    fn resolve_absolute(&self, module_path: &str) -> Option<PathBuf> {
        self.modules.get(module_path).cloned().or_else(|| {
            // If exact match not found, try to find by stripping last segment
            // (e.g., `foo::Bar` -> `foo.rs` where Bar is a type/fn in foo)
            let mut parts: Vec<&str> = module_path.split("::").collect();
            while !parts.is_empty() {
                parts.pop();
                let partial = parts.join("::");
                if let Some(path) = self.modules.get(&partial) {
                    return Some(path.clone());
                }
            }
            None
        })
    }

    /// Convert a file path to its module path
    fn file_to_module_path(&self, file_path: &Path) -> Option<String> {
        let relative = file_path.strip_prefix(&self.crate_root).ok()?;

        // Convert path to module path: src/foo/bar.rs -> foo::bar
        let mut parts = Vec::new();
        let path_components: Vec<_> = relative.components().collect();

        for (i, component) in path_components.iter().enumerate() {
            let component_str = component.as_os_str().to_str()?;
            if component_str == "src" {
                continue;
            }

            // Check if this is the last component (the file itself)
            let is_last = i == path_components.len() - 1;

            if is_last {
                // For lib.rs or main.rs, this is the root module
                if component_str == "lib.rs" || component_str == "main.rs" {
                    break; // Root module
                }
                // For mod.rs, don't add it (parent dir is the module)
                if component_str == "mod.rs" {
                    break;
                }
                // For foo.rs, add "foo"
                if component_str.ends_with(".rs") {
                    let name = component_str.strip_suffix(".rs").unwrap_or(component_str);
                    parts.push(name);
                }
            } else {
                // Directory component - add it
                parts.push(component_str);
            }
        }

        if parts.is_empty() {
            Some(String::new()) // Root module
        } else {
            Some(parts.join("::"))
        }
    }

    /// Get parent module path (e.g., "foo::bar::baz" -> "foo::bar")
    fn parent_module(&self, module_path: &str) -> Option<String> {
        if module_path.is_empty() {
            return None; // Root module has no parent
        }

        let mut parts: Vec<&str> = module_path.split("::").collect();
        if parts.is_empty() {
            return None;
        }

        parts.pop();
        if parts.is_empty() {
            Some(String::new()) // Parent is root
        } else {
            Some(parts.join("::"))
        }
    }
}

pub(crate) fn analyze_rust_file(
    content: &str,
    relative: String,
    custom_command_macros: &[String],
) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative);
    let mut event_emits = Vec::new();
    let mut event_listens = Vec::new();

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
                    "const",
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

    collect_rust_signature_uses(&production_content, &mut analysis);

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

    // Detect path-qualified calls like `module::function()` or `Type::method()`
    // These are function calls via module path without explicit `use` import.
    // Pattern: `::<identifier>(` or `::<Identifier>{` or `::<Identifier><`
    // This catches: command::branch::handle(), OutputChannel::new(), etc.
    extract_path_qualified_calls(&production_content, &mut analysis.local_uses);

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

/// Extract identifiers used in expressions and variable declarations.
/// This catches:
/// - Constants in generic parameters: `foo::<BUFFER_SIZE, _>`
/// - Constants in array sizes: `[0; BUFFER_SIZE]`
/// - Types in let bindings: `let x: Config = ...`
/// - Types in struct literals: `Config { ... }`
fn extract_identifier_usages(content: &str, local_uses: &mut Vec<String>) {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Skip Rust keywords that look like identifiers
    const KEYWORDS: &[&str] = &[
        "if", "else", "while", "for", "loop", "match", "return", "break", "continue", "fn", "let",
        "const", "static", "pub", "use", "mod", "struct", "enum", "impl", "trait", "type", "where",
        "unsafe", "async", "await", "move", "ref", "mut", "self", "super", "crate", "dyn", "as",
        "in", "true", "false", "Some", "None", "Ok", "Err", "bool", "char", "str", "u8", "u16",
        "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64",
    ];

    fn add_if_uppercase(ident: &str, uses: &mut Vec<String>) {
        if ident.is_empty() {
            return;
        }
        let first_char = ident.chars().next().unwrap_or('_');
        if first_char.is_ascii_uppercase() && !uses.contains(&ident.to_string()) {
            uses.push(ident.to_string());
        }
    }

    while i < len {
        // Look for `<` which could be start of generic parameters
        if bytes[i] == b'<' {
            i += 1;
            // Skip whitespace after `<`
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            // Now look for identifier inside the generic
            if i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = &content[start..i];
                if !KEYWORDS.contains(&ident) {
                    add_if_uppercase(ident, local_uses);
                }
            }
            continue;
        }

        // Look for `: Type` patterns (type annotations)
        if bytes[i] == b':' {
            i += 1;
            // Skip whitespace and possible second `:` (for `::`)
            while i < len && (bytes[i].is_ascii_whitespace() || bytes[i] == b':') {
                i += 1;
            }

            // Now look for identifier after `:`
            if i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = &content[start..i];
                if !KEYWORDS.contains(&ident) {
                    add_if_uppercase(ident, local_uses);
                }
            }
            continue;
        }

        // Look for struct literals or other identifiers: `TypeName {` or `CONST.method()`
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &content[start..i];

            // Skip whitespace
            let saved_i = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            // Check what follows the identifier:
            // - `{` = struct literal (e.g., `Config { ... }`)
            // - `.` = method call on const/static (e.g., `CONST.as_bytes()`)
            // - `(` = handled by bare_function_calls
            if i < len && !KEYWORDS.contains(&ident) {
                if bytes[i] == b'{' {
                    // Struct literal
                    add_if_uppercase(ident, local_uses);
                    i += 1; // Move past `{`
                } else if bytes[i] == b'.' {
                    // Method call on identifier (likely a const/static)
                    add_if_uppercase(ident, local_uses);
                    i += 1; // Move past `.`
                } else {
                    // Not a special pattern, restore position
                    i = saved_i;
                    i += 1;
                }
            } else {
                // Keyword or end of content
                i = saved_i;
                if i < len {
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Extract identifiers that are followed by `(` indicating a function call.
/// This catches bare function calls like `my_func(arg)` within the same file.
fn extract_bare_function_calls(content: &str, local_uses: &mut Vec<String>) {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for identifier followed by `(`
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &content[start..i];

            // Skip whitespace
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            // Check if followed by `(` (function call) or `!` (macro call)
            if i < len && (bytes[i] == b'(' || bytes[i] == b'!') {
                // Skip Rust keywords that aren't function calls
                const KEYWORDS: &[&str] = &[
                    "if", "else", "while", "for", "loop", "match", "return", "break", "continue",
                    "fn", "let", "const", "static", "pub", "use", "mod", "struct", "enum", "impl",
                    "trait", "type", "where", "unsafe", "async", "await", "move", "ref", "mut",
                    "self", "super", "crate", "dyn", "as", "in", "true", "false",
                ];
                if !KEYWORDS.contains(&ident) && !local_uses.contains(&ident.to_string()) {
                    local_uses.push(ident.to_string());
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Extract uppercase identifiers used as function arguments.
/// This catches constants passed to functions like `.timer(COPILOT_DEBOUNCE_TIMEOUT)`
/// or `advance_clock(BUFFER_SIZE)`.
fn extract_function_arguments(content: &str, local_uses: &mut Vec<String>) {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Rust keywords that look like identifiers but aren't values
    const KEYWORDS: &[&str] = &[
        "if", "else", "while", "for", "loop", "match", "return", "break", "continue", "fn", "let",
        "const", "static", "pub", "use", "mod", "struct", "enum", "impl", "trait", "type", "where",
        "unsafe", "async", "await", "move", "ref", "mut", "self", "super", "crate", "dyn", "as",
        "in", "true", "false", "Some", "None", "Ok", "Err", "bool", "char", "str", "u8", "u16",
        "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64",
    ];

    fn add_if_uppercase(ident: &str, uses: &mut Vec<String>, keywords: &[&str]) {
        if ident.is_empty() || keywords.contains(&ident) {
            return;
        }
        // Only add if ALL characters are uppercase/underscore/digits (like CONST_NAME)
        // This avoids false positives from regular identifiers
        let is_const_style = ident
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit());
        if is_const_style
            && ident.chars().any(|c| c.is_ascii_uppercase())
            && !uses.contains(&ident.to_string())
        {
            uses.push(ident.to_string());
        }
    }

    while i < len {
        // Look for `(` or `,` which could precede a function argument
        if bytes[i] == b'(' || bytes[i] == b',' {
            i += 1;
            // Skip whitespace after `(` or `,`
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            // Check if we have an identifier starting with uppercase
            if i < len && bytes[i].is_ascii_uppercase() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = &content[start..i];
                add_if_uppercase(ident, local_uses, KEYWORDS);
            }
        } else {
            i += 1;
        }
    }
}

fn identifier_finder() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").expect("valid identifier regex"))
}

fn collect_identifier_mentions(content: &str, local_uses: &mut Vec<String>) {
    const SKIP: &[&str] = &[
        "if", "else", "while", "for", "loop", "match", "return", "break", "continue", "fn", "let",
        "const", "static", "pub", "use", "mod", "struct", "enum", "impl", "trait", "type", "where",
        "unsafe", "async", "await", "move", "ref", "mut", "self", "super", "crate", "dyn", "as",
        "in", "true", "false", "Some", "None", "Ok", "Err", "bool", "char", "str", "u8", "u16",
        "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64",
        "String", "Vec", "Option", "Result", "Self",
    ];

    for cap in identifier_finder().find_iter(content) {
        let ident = cap.as_str();
        if SKIP.contains(&ident) {
            continue;
        }
        if !local_uses.contains(&ident.to_string()) {
            local_uses.push(ident.to_string());
        }
    }
}

/// Extract identifiers from path-qualified calls like `foo::bar::func()` or `Type::new()`
/// These are usages that don't require a `use` import.
/// For `Foo::bar::baz()`, we record ALL segments: Foo, bar, baz (each might be a pub export)
fn extract_path_qualified_calls(content: &str, local_uses: &mut Vec<String>) {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Helper to add identifier if not already present
    fn add_ident(ident: &str, uses: &mut Vec<String>) {
        if !ident.is_empty() && !uses.contains(&ident.to_string()) {
            uses.push(ident.to_string());
        }
    }

    while i < len {
        // Look for identifier followed by `::`
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &content[start..i];

            // Skip whitespace
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            // Check if followed by `::`
            if i + 1 < len && bytes[i] == b':' && bytes[i + 1] == b':' {
                // This is a path-qualified usage (Type::method or module::func)
                // Record the first identifier (it's a type or module being used)
                add_ident(ident, local_uses);

                // Now scan the rest of the path, recording all segments
                while i + 1 < len && bytes[i] == b':' && bytes[i + 1] == b':' {
                    i += 2;
                    // Skip whitespace
                    while i < len && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    // Read next identifier
                    let seg_start = i;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    if i > seg_start {
                        let seg = &content[seg_start..i];
                        add_ident(seg, local_uses);
                    }
                    // Skip whitespace
                    while i < len && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Strip both line comments (//) and block comments (/* */) from Rust source code.
/// This prevents false positives where type names are mentioned in comments.
fn strip_comments(content: &str) -> String {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        // Check for line comment //
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Skip to end of line
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            if i < len {
                result.push('\n'); // Preserve line breaks
                i += 1;
            }
            continue;
        }

        // Check for block comment /* */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Skip to closing */
            i += 2;
            while i + 1 < len {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                // Preserve newlines within comments to maintain line structure
                if bytes[i] == b'\n' {
                    result.push('\n');
                }
                i += 1;
            }
            continue;
        }

        // Regular character
        if let Some(ch) = content[i..].chars().next() {
            result.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    result
}

/// Extract type names used in struct/enum field definitions.
/// This catches types like `Vec<DiffEdge>`, `Option<HubFile>`, `HashMap<K, V>` etc.
/// that are used as field types within the same file.
///
/// Patterns handled:
/// - `pub struct Foo { field: SomeType, ... }`
/// - `pub struct Foo { field: Vec<SomeType>, ... }`
/// - `pub enum Foo { Variant { field: SomeType }, ... }`
/// - Tuple structs: `pub struct Foo(SomeType, AnotherType);`
fn extract_struct_field_types(content: &str, local_uses: &mut Vec<String>) {
    // Strip comments first to avoid false positives from type names in comments
    let content_no_comments = strip_comments(content);

    // Helper to add identifier if not already present and looks like a type name
    fn add_type_if_valid(name: &str, uses: &mut Vec<String>) {
        if name.is_empty() {
            return;
        }
        // Type names typically start with uppercase
        let first_char = name.chars().next().unwrap_or('_');
        if !first_char.is_ascii_uppercase() {
            return;
        }
        // Skip common standard library types (they're not local exports)
        const STD_TYPES: &[&str] = &[
            "Vec",
            "Option",
            "Result",
            "String",
            "Box",
            "Rc",
            "Arc",
            "Cell",
            "RefCell",
            "HashMap",
            "HashSet",
            "BTreeMap",
            "BTreeSet",
            "VecDeque",
            "LinkedList",
            "Mutex",
            "RwLock",
            "Cow",
            "PathBuf",
            "OsString",
            "CString",
            "Duration",
            "Instant",
            "SystemTime",
            "NonZeroU8",
            "NonZeroU16",
            "NonZeroU32",
            "NonZeroU64",
            "NonZeroUsize",
            "NonZeroI8",
            "NonZeroI16",
            "NonZeroI32",
            "NonZeroI64",
            "NonZeroIsize",
            "PhantomData",
            "Pin",
            "ManuallyDrop",
            "MaybeUninit",
            "Self",
        ];
        if STD_TYPES.contains(&name) {
            return;
        }
        if !uses.contains(&name.to_string()) {
            uses.push(name.to_string());
        }
    }

    // Extract type tokens from a type annotation string
    fn extract_types_from_annotation(annotation: &str, uses: &mut Vec<String>) {
        let bytes = annotation.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        while i < len {
            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = &annotation[start..i];
                add_type_if_valid(ident, uses);
            } else {
                i += 1;
            }
        }
    }

    // Inner function to parse struct block content for type annotations
    fn parse_struct_block_for_types(block: &str, uses: &mut Vec<String>) {
        let bytes = block.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        while i < len {
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= len {
                break;
            }
            if bytes[i] == b':' {
                i += 1;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                let type_start = i;
                let mut depth = 0;
                while i < len {
                    match bytes[i] {
                        b'<' | b'(' | b'[' | b'{' => depth += 1,
                        b'>' | b')' | b']' | b'}' => {
                            if depth > 0 {
                                depth -= 1;
                            } else if bytes[i] == b'}' {
                                break;
                            }
                        }
                        b',' if depth == 0 => break,
                        _ => {}
                    }
                    i += 1;
                }
                if type_start < i {
                    let type_annotation = &block[type_start..i];
                    extract_types_from_annotation(type_annotation, uses);
                }
            } else {
                i += 1;
            }
        }
    }

    // Find struct/enum blocks and extract field types from comment-stripped content
    let bytes = content_no_comments.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip multi-byte UTF-8 characters (non-ASCII) since keywords are ASCII
        if bytes[i] >= 0x80 {
            i += 1;
            continue;
        }

        // Check for 'struct' or 'enum' keyword by looking at bytes directly
        let is_struct = i + 7 <= len
            && bytes[i] == b's'
            && bytes[i + 1] == b't'
            && bytes[i + 2] == b'r'
            && bytes[i + 3] == b'u'
            && bytes[i + 4] == b'c'
            && bytes[i + 5] == b't'
            && (bytes[i + 6] == b' ' || bytes[i + 6] == b'\t' || bytes[i + 6] == b'\n');
        let is_enum = !is_struct
            && i + 5 <= len
            && bytes[i] == b'e'
            && bytes[i + 1] == b'n'
            && bytes[i + 2] == b'u'
            && bytes[i + 3] == b'm'
            && (bytes[i + 4] == b' ' || bytes[i + 4] == b'\t' || bytes[i + 4] == b'\n');

        if is_struct || is_enum {
            let keyword_len = if is_struct { 6 } else { 4 };
            i += keyword_len;

            while i < len {
                let ch = bytes[i];
                if ch == b'{' {
                    i += 1;
                    let mut depth = 1;
                    let block_start = i;
                    while i < len && depth > 0 {
                        match bytes[i] {
                            b'{' => depth += 1,
                            b'}' => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            i += 1;
                        }
                    }
                    // Safe to slice since we're tracking ASCII braces
                    if let Some(block) = content_no_comments.get(block_start..i) {
                        parse_struct_block_for_types(block, local_uses);
                    }
                    break;
                } else if ch == b'(' {
                    i += 1;
                    let paren_start = i;
                    let mut depth = 1;
                    while i < len && depth > 0 {
                        match bytes[i] {
                            b'(' => depth += 1,
                            b')' => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            i += 1;
                        }
                    }
                    // Safe to slice since we're tracking ASCII parens
                    if let Some(tuple_content) = content_no_comments.get(paren_start..i) {
                        extract_types_from_annotation(tuple_content, local_uses);
                    }
                    break;
                } else if ch == b';' {
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
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

    #[test]
    fn strip_function_body_uses_excludes_inline_imports() {
        // This is the exact pattern that caused false positive cycles in snapshot.rs
        let content = r#"
use crate::types::FileAnalysis;

pub fn run_init() {
    use crate::analyzer::root_scan::{ScanConfig, scan_roots};
    use crate::analyzer::runner::default_analyzer_exts;

    let _x = 1;
}

pub fn another_fn() {}
"#;
        let analysis = analyze_rust_file(content, "snapshot.rs".to_string(), &[]);

        // Should only have the module-level import, NOT the inline imports
        assert_eq!(
            analysis.imports.len(),
            1,
            "Should have 1 module-level import, got {:?}",
            analysis.imports
        );
        assert!(
            analysis.imports[0].source.contains("types::FileAnalysis"),
            "Should have FileAnalysis import"
        );
        // Should NOT contain the inline imports
        assert!(
            !analysis
                .imports
                .iter()
                .any(|i| i.source.contains("root_scan")),
            "Should NOT contain inline root_scan import"
        );
        assert!(
            !analysis.imports.iter().any(|i| i.source.contains("runner")),
            "Should NOT contain inline runner import"
        );
    }

    #[test]
    fn strip_function_body_preserves_module_level_imports() {
        let content = r#"
use crate::one::A;
use crate::two::B;

fn helper() {
    use crate::inline::C;
}

use crate::three::D;
"#;
        let analysis = analyze_rust_file(content, "test.rs".to_string(), &[]);

        // Should have 3 module-level imports (A, B, D), NOT the inline one (C)
        let sources: Vec<&str> = analysis.imports.iter().map(|i| i.source.as_str()).collect();
        assert!(
            sources.iter().any(|s| s.contains("one::A")),
            "Should have A"
        );
        assert!(
            sources.iter().any(|s| s.contains("two::B")),
            "Should have B"
        );
        assert!(
            sources.iter().any(|s| s.contains("three::D")),
            "Should have D"
        );
        assert!(
            !sources.iter().any(|s| s.contains("inline::C")),
            "Should NOT have inline C"
        );
        assert_eq!(sources.len(), 3, "Should have exactly 3 imports");
    }

    #[test]
    fn detects_struct_field_types_as_local_uses() {
        // This is the exact pattern that caused false positives in dead export detection:
        // DiffEdge is defined and then used as Vec<DiffEdge> in GraphDiff's field
        let content = r#"
pub struct DiffEdge {
    pub from: PathBuf,
    pub to: PathBuf,
}

pub struct GraphDiff {
    pub edges_added: Vec<DiffEdge>,
    pub edges_removed: Vec<DiffEdge>,
}
"#;
        let analysis = analyze_rust_file(content, "diff.rs".to_string(), &[]);

        // DiffEdge should be detected as a local use because it's used in GraphDiff's fields
        assert!(
            analysis.local_uses.contains(&"DiffEdge".to_string()),
            "DiffEdge should be in local_uses when used as struct field type. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn detects_generic_wrapped_types_in_struct_fields() {
        let content = r#"
pub struct HubFile {
    pub path: String,
}

pub struct ForAiSummary {
    pub hubs: Vec<HubFile>,
    pub warnings: Option<WarningItem>,
    pub cache: HashMap<String, CacheEntry>,
}

pub struct WarningItem {
    pub msg: String,
}

pub struct CacheEntry {
    pub value: u32,
}
"#;
        let analysis = analyze_rust_file(content, "for_ai.rs".to_string(), &[]);

        // All custom types used in generic wrappers should be in local_uses
        assert!(
            analysis.local_uses.contains(&"HubFile".to_string()),
            "HubFile should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
        assert!(
            analysis.local_uses.contains(&"WarningItem".to_string()),
            "WarningItem should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
        assert!(
            analysis.local_uses.contains(&"CacheEntry".to_string()),
            "CacheEntry should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn detects_tuple_struct_types() {
        let content = r#"
pub struct Point(i32, i32);
pub struct Container(InnerType, Option<AnotherType>);

pub struct InnerType {
    pub value: u32,
}

pub struct AnotherType {
    pub name: String,
}
"#;
        let analysis = analyze_rust_file(content, "tuple.rs".to_string(), &[]);

        assert!(
            analysis.local_uses.contains(&"InnerType".to_string()),
            "InnerType from tuple struct should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
        assert!(
            analysis.local_uses.contains(&"AnotherType".to_string()),
            "AnotherType from tuple struct should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn detects_enum_variant_types() {
        let content = r#"
pub enum Message {
    Data { payload: CustomPayload },
    Error { code: ErrorCode, details: Vec<ErrorDetail> },
    Empty,
}

pub struct CustomPayload {
    pub data: Vec<u8>,
}

pub struct ErrorCode(u32);

pub struct ErrorDetail {
    pub msg: String,
}
"#;
        let analysis = analyze_rust_file(content, "message.rs".to_string(), &[]);

        assert!(
            analysis.local_uses.contains(&"CustomPayload".to_string()),
            "CustomPayload should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
        assert!(
            analysis.local_uses.contains(&"ErrorCode".to_string()),
            "ErrorCode should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
        assert!(
            analysis.local_uses.contains(&"ErrorDetail".to_string()),
            "ErrorDetail should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn detects_const_usage_in_generic_parameters() {
        // This is the exact pattern that caused false positives in Zed codebase:
        // pub const BUFFER_SIZE is used in the same file as `<BUFFER_SIZE, _>`
        let content = r#"
pub const BUFFER_SIZE: usize = 480;

pub fn process() {
    let source = source.inspect_buffer::<BUFFER_SIZE, _>(move |buffer| {
        // process buffer
    });
}
"#;
        let analysis = analyze_rust_file(content, "audio.rs".to_string(), &[]);

        assert!(
            analysis.local_uses.contains(&"BUFFER_SIZE".to_string()),
            "BUFFER_SIZE should be in local_uses when used in generic parameter. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn detects_type_usage_in_struct_literal() {
        let content = r#"
pub struct Config {
    pub name: String,
}

pub fn setup() {
    let config = Config {
        name: "test".to_string(),
    };
}
"#;
        let analysis = analyze_rust_file(content, "config.rs".to_string(), &[]);

        assert!(
            analysis.local_uses.contains(&"Config".to_string()),
            "Config should be in local_uses when used in struct literal. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn detects_type_usage_in_type_annotation() {
        let content = r#"
pub struct MyType {
    pub value: u32,
}

pub fn create() -> MyType {
    let result: MyType = MyType { value: 42 };
    result
}
"#;
        let analysis = analyze_rust_file(content, "types.rs".to_string(), &[]);

        assert!(
            analysis.local_uses.contains(&"MyType".to_string()),
            "MyType should be in local_uses when used in type annotation. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn detects_const_usage_with_method_call() {
        let content = r#"
pub const CLEAR_INPUT: &str = "\x15";

pub fn undo() {
    terminal.input(CLEAR_INPUT.as_bytes());
}
"#;
        let analysis = analyze_rust_file(content, "terminal.rs".to_string(), &[]);

        assert!(
            analysis.local_uses.contains(&"CLEAR_INPUT".to_string()),
            "CLEAR_INPUT should be in local_uses when used with method call. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn skips_lowercase_identifiers_in_generic_params() {
        // Should NOT track lowercase identifiers like `move` or `self`
        let content = r#"
pub const MY_CONST: usize = 10;

pub fn process() {
    let closure = move |x| x + 1;
}
"#;
        let analysis = analyze_rust_file(content, "test.rs".to_string(), &[]);

        assert!(
            !analysis.local_uses.contains(&"move".to_string()),
            "Keyword 'move' should NOT be in local_uses"
        );
    }

    #[test]
    fn skips_standard_library_types_in_struct_fields() {
        let content = r#"
pub struct MyStruct {
    pub items: Vec<String>,
    pub mapping: HashMap<String, u32>,
    pub optional: Option<bool>,
    pub custom: CustomType,
}

pub struct CustomType {
    pub value: u32,
}
"#;
        let analysis = analyze_rust_file(content, "types.rs".to_string(), &[]);

        // Should NOT include Vec, HashMap, Option, String (std types)
        assert!(
            !analysis.local_uses.contains(&"Vec".to_string()),
            "Vec should NOT be in local_uses"
        );
        assert!(
            !analysis.local_uses.contains(&"HashMap".to_string()),
            "HashMap should NOT be in local_uses"
        );
        assert!(
            !analysis.local_uses.contains(&"Option".to_string()),
            "Option should NOT be in local_uses"
        );
        assert!(
            !analysis.local_uses.contains(&"String".to_string()),
            "String should NOT be in local_uses"
        );

        // But SHOULD include CustomType
        assert!(
            analysis.local_uses.contains(&"CustomType".to_string()),
            "CustomType should be in local_uses. Got: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn crate_module_map_resolves_crate_prefix() {
        // Create a temporary directory structure for testing
        let temp_dir = std::env::temp_dir().join("loctree_test_crate");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("src/foo")).unwrap();

        // Create lib.rs with module declarations
        std::fs::write(temp_dir.join("src/lib.rs"), "pub mod foo;\npub mod bar;\n").unwrap();

        // Create foo.rs
        std::fs::write(
            temp_dir.join("src/foo.rs"),
            "pub struct FooStruct;\npub mod nested;\n",
        )
        .unwrap();

        // Create foo/nested.rs
        std::fs::write(
            temp_dir.join("src/foo/nested.rs"),
            "pub struct NestedStruct;\n",
        )
        .unwrap();

        // Create bar.rs
        std::fs::write(temp_dir.join("src/bar.rs"), "pub struct BarStruct;\n").unwrap();

        // Build the module map
        let map = super::CrateModuleMap::build(&temp_dir).unwrap();

        // Test resolving crate::foo from lib.rs
        let lib_path = temp_dir.join("src/lib.rs");
        let resolved = map.resolve_module_path(&lib_path, "crate::foo");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/foo.rs")
        );

        // Test resolving crate::bar
        let resolved = map.resolve_module_path(&lib_path, "crate::bar");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/bar.rs")
        );

        // Test resolving crate::foo::nested
        let resolved = map.resolve_module_path(&lib_path, "crate::foo::nested");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/foo/nested.rs")
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn crate_module_map_resolves_super() {
        let temp_dir = std::env::temp_dir().join("loctree_test_super");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("src/foo")).unwrap();

        // Create lib.rs
        std::fs::write(temp_dir.join("src/lib.rs"), "pub mod foo;\n").unwrap();

        // Create foo.rs with nested module
        std::fs::write(
            temp_dir.join("src/foo.rs"),
            "pub mod nested;\npub struct FooStruct;\n",
        )
        .unwrap();

        // Create foo/nested.rs
        std::fs::write(
            temp_dir.join("src/foo/nested.rs"),
            "use super::FooStruct;\n",
        )
        .unwrap();

        let map = super::CrateModuleMap::build(&temp_dir).unwrap();

        // From nested.rs, super:: should resolve to foo.rs
        let nested_path = temp_dir.join("src/foo/nested.rs");
        let resolved = map.resolve_module_path(&nested_path, "super::FooStruct");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/foo.rs")
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn crate_module_map_resolves_self() {
        let temp_dir = std::env::temp_dir().join("loctree_test_self");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("src/foo")).unwrap();

        // Create lib.rs
        std::fs::write(temp_dir.join("src/lib.rs"), "pub mod foo;\n").unwrap();

        // Create foo.rs with nested module
        std::fs::write(
            temp_dir.join("src/foo.rs"),
            "pub mod bar;\npub struct FooStruct;\n",
        )
        .unwrap();

        // Create foo/bar.rs
        std::fs::write(temp_dir.join("src/foo/bar.rs"), "pub struct BarStruct;\n").unwrap();

        let map = super::CrateModuleMap::build(&temp_dir).unwrap();

        // From foo.rs, self::bar should resolve to foo/bar.rs
        let foo_path = temp_dir.join("src/foo.rs");
        let resolved = map.resolve_module_path(&foo_path, "self::bar");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/foo/bar.rs")
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn crate_module_map_resolves_relative_imports() {
        let temp_dir = std::env::temp_dir().join("loctree_test_relative");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("src")).unwrap();

        // Create lib.rs
        std::fs::write(temp_dir.join("src/lib.rs"), "pub mod foo;\npub mod bar;\n").unwrap();

        // Create foo.rs
        std::fs::write(temp_dir.join("src/foo.rs"), "use bar::BarStruct;\n").unwrap();

        // Create bar.rs
        std::fs::write(temp_dir.join("src/bar.rs"), "pub struct BarStruct;\n").unwrap();

        let map = super::CrateModuleMap::build(&temp_dir).unwrap();

        // From foo.rs, bare `bar` should resolve relative to root
        let foo_path = temp_dir.join("src/foo.rs");
        let resolved = map.resolve_module_path(&foo_path, "bar");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/bar.rs")
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn crate_module_map_handles_mod_dir_structure() {
        // Test the foo/mod.rs convention
        let temp_dir = std::env::temp_dir().join("loctree_test_mod_dir");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("src/foo")).unwrap();

        // Create lib.rs
        std::fs::write(temp_dir.join("src/lib.rs"), "pub mod foo;\n").unwrap();

        // Create foo/mod.rs
        std::fs::write(temp_dir.join("src/foo/mod.rs"), "pub struct FooStruct;\n").unwrap();

        let map = super::CrateModuleMap::build(&temp_dir).unwrap();

        // Resolve crate::foo
        let lib_path = temp_dir.join("src/lib.rs");
        let resolved = map.resolve_module_path(&lib_path, "crate::foo");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/foo/mod.rs")
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn crate_module_map_handles_type_imports() {
        // Test that we can resolve imports with type names (last segment)
        let temp_dir = std::env::temp_dir().join("loctree_test_types");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(temp_dir.join("src")).unwrap();

        // Create lib.rs
        std::fs::write(temp_dir.join("src/lib.rs"), "pub mod types;\n").unwrap();

        // Create types.rs with struct
        std::fs::write(
            temp_dir.join("src/types.rs"),
            "pub struct MyType { pub value: u32 }\n",
        )
        .unwrap();

        let map = super::CrateModuleMap::build(&temp_dir).unwrap();

        // Resolve crate::types::MyType - should resolve to types.rs
        // (stripping the type name segment)
        let lib_path = temp_dir.join("src/lib.rs");
        let resolved = map.resolve_module_path(&lib_path, "crate::types::MyType");
        assert_eq!(
            resolved.as_ref().map(|p| p.to_str().unwrap()),
            Some("src/types.rs")
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn detects_crate_relative_imports() {
        let content = r#"
use crate::types::FileAnalysis;
use crate::analyzer::{rust, typescript};
use std::path::PathBuf;
"#;
        let analysis = analyze_rust_file(content, "module.rs".to_string(), &[]);

        // Check that crate:: imports are detected
        let crate_imports: Vec<_> = analysis
            .imports
            .iter()
            .filter(|i| i.is_crate_relative)
            .collect();
        assert_eq!(
            crate_imports.len(),
            2,
            "Should have 2 crate:: imports, got {:?}",
            analysis.imports
        );

        // Check raw_path is preserved
        let file_analysis_import = analysis
            .imports
            .iter()
            .find(|i| i.raw_path.contains("FileAnalysis"));
        assert!(
            file_analysis_import.is_some(),
            "Should find FileAnalysis import"
        );
        assert!(
            file_analysis_import.unwrap().is_crate_relative,
            "FileAnalysis import should be marked as crate_relative"
        );
        assert_eq!(
            file_analysis_import.unwrap().raw_path,
            "crate::types::FileAnalysis",
            "raw_path should preserve full import path"
        );
    }

    #[test]
    fn detects_super_relative_imports() {
        let content = r#"
use super::models::Patient;
use super::super::config::Config;
use super::*;
"#;
        let analysis = analyze_rust_file(content, "handlers.rs".to_string(), &[]);

        // All imports should be super:: relative
        assert_eq!(
            analysis.imports.len(),
            3,
            "Should have 3 imports, got {:?}",
            analysis.imports
        );
        for imp in &analysis.imports {
            assert!(
                imp.is_super_relative,
                "All imports should be super:: relative, got: {:?}",
                imp
            );
        }

        // Check star import is handled
        let star_import = analysis
            .imports
            .iter()
            .find(|i| i.symbols.iter().any(|s| s.name == "*"));
        assert!(star_import.is_some(), "Should have star import");
        assert!(
            star_import.unwrap().is_super_relative,
            "Star import should be super:: relative"
        );
    }

    #[test]
    fn detects_self_relative_imports() {
        let content = r#"
use self::utils::Helper;
use self::constants::MAX_SIZE;
"#;
        let analysis = analyze_rust_file(content, "lib.rs".to_string(), &[]);

        assert_eq!(analysis.imports.len(), 2, "Should have 2 imports");
        for imp in &analysis.imports {
            assert!(
                imp.is_self_relative,
                "All imports should be self:: relative, got: {:?}",
                imp
            );
            assert!(
                imp.raw_path.starts_with("self::"),
                "raw_path should start with self::"
            );
        }
    }

    #[test]
    fn handles_brace_imports_with_relative_paths() {
        let content = r#"
use crate::types::{FileAnalysis, ImportEntry, ExportSymbol};
use super::models::{Patient, Visit};
use self::helpers::{parse, validate};
"#;
        let analysis = analyze_rust_file(content, "module.rs".to_string(), &[]);

        assert_eq!(analysis.imports.len(), 3, "Should have 3 imports");

        // Check crate:: import
        let crate_import = analysis.imports.iter().find(|i| i.is_crate_relative);
        assert!(crate_import.is_some(), "Should have crate:: import");
        let crate_import = crate_import.unwrap();
        assert_eq!(crate_import.symbols.len(), 3, "Should have 3 symbols");
        assert!(
            crate_import
                .symbols
                .iter()
                .any(|s| s.name == "FileAnalysis")
        );
        assert!(crate_import.symbols.iter().any(|s| s.name == "ImportEntry"));
        assert!(
            crate_import
                .symbols
                .iter()
                .any(|s| s.name == "ExportSymbol")
        );
        assert_eq!(
            crate_import.raw_path, "crate::types::{FileAnalysis, ImportEntry, ExportSymbol}",
            "raw_path should preserve full import with braces"
        );

        // Check super:: import
        let super_import = analysis.imports.iter().find(|i| i.is_super_relative);
        assert!(super_import.is_some(), "Should have super:: import");
        assert_eq!(super_import.unwrap().symbols.len(), 2);

        // Check self:: import
        let self_import = analysis.imports.iter().find(|i| i.is_self_relative);
        assert!(self_import.is_some(), "Should have self:: import");
        assert_eq!(self_import.unwrap().symbols.len(), 2);
    }

    #[test]
    fn distinguishes_relative_from_external_imports() {
        let content = r#"
use crate::internal::Module;
use super::parent::Module2;
use self::current::Module3;
use std::collections::HashMap;
use serde::Serialize;
"#;
        let analysis = analyze_rust_file(content, "mixed.rs".to_string(), &[]);

        assert_eq!(analysis.imports.len(), 5, "Should have 5 imports");

        // Count relative vs external
        let relative_count = analysis
            .imports
            .iter()
            .filter(|i| i.is_crate_relative || i.is_super_relative || i.is_self_relative)
            .count();
        assert_eq!(relative_count, 3, "Should have 3 relative imports");

        let external_count = analysis
            .imports
            .iter()
            .filter(|i| !i.is_crate_relative && !i.is_super_relative && !i.is_self_relative)
            .count();
        assert_eq!(external_count, 2, "Should have 2 external imports");

        // External imports should not have relative flags set
        let std_import = analysis
            .imports
            .iter()
            .find(|i| i.raw_path.contains("std::"));
        assert!(std_import.is_some());
        assert!(!std_import.unwrap().is_crate_relative);
        assert!(!std_import.unwrap().is_super_relative);
        assert!(!std_import.unwrap().is_self_relative);
    }

    #[test]
    fn preserves_raw_path_for_all_imports() {
        let content = r#"
use crate::foo::Bar;
use super::baz::Qux;
use std::path::PathBuf;
"#;
        let analysis = analyze_rust_file(content, "test.rs".to_string(), &[]);

        for imp in &analysis.imports {
            assert!(
                !imp.raw_path.is_empty(),
                "raw_path should be set for all imports"
            );
        }

        // Verify raw_path matches original import source
        let paths: Vec<&str> = analysis
            .imports
            .iter()
            .map(|i| i.raw_path.as_str())
            .collect();
        assert!(paths.contains(&"crate::foo::Bar"));
        assert!(paths.contains(&"super::baz::Qux"));
        assert!(paths.contains(&"std::path::PathBuf"));
    }

    #[test]
    fn handles_nested_module_paths() {
        let content = r#"
use crate::foo::bar::baz::Qux;
use super::super::super::root::Type;
"#;
        let analysis = analyze_rust_file(content, "deep.rs".to_string(), &[]);

        assert_eq!(analysis.imports.len(), 2);

        let crate_import = analysis.imports.iter().find(|i| i.is_crate_relative);
        assert!(crate_import.is_some());
        assert_eq!(crate_import.unwrap().raw_path, "crate::foo::bar::baz::Qux");

        let super_import = analysis.imports.iter().find(|i| i.is_super_relative);
        assert!(super_import.is_some());
        assert_eq!(
            super_import.unwrap().raw_path,
            "super::super::super::root::Type"
        );
    }
}
