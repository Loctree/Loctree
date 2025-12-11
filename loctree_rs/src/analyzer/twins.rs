//! Twins Module - Semantic Duplicate Detection
//!
//! Finds two types of code issues:
//! 1. **Dead Parrots**: Exported symbols with zero imports
//! 2. **Exact Twins**: Symbols with the same name exported from different files
//!
//! These are candidates for removal or consolidation.
//!
//! # Philosophy
//!
//! Not all exports need imports to be useful:
//! - Library entry points (lib.rs, index.ts)
//! - CLI handlers (main.rs)
//! - Test fixtures
//! - Framework magic (Next.js pages, Tauri commands)
//!
//! This module focuses on **internal application code** where zero imports
//! or duplicate names usually indicate dead code or naming conflicts.

use serde::Serialize;
use std::collections::HashMap;

use crate::types::{FileAnalysis, OutputMode};

/// A single symbol entry in the registry
#[derive(Debug, Clone, Serialize)]
pub struct SymbolEntry {
    /// Symbol name
    pub name: String,
    /// Symbol kind (function, type, const, class, interface, re-export)
    pub kind: String,
    /// File path where symbol is exported
    pub file_path: String,
    /// Line number (if available)
    pub line: usize,
    /// Number of files that import this symbol
    pub import_count: usize,
}

/// Result of twins analysis
#[derive(Debug, Clone, Serialize)]
pub struct TwinsResult {
    /// All dead parrots (0 imports)
    pub dead_parrots: Vec<SymbolEntry>,
    /// Total symbols analyzed
    pub total_symbols: usize,
    /// Total files analyzed
    pub total_files: usize,
}

/// Build symbol registry from file analyses
///
/// Counts how many times each symbol is imported across the codebase.
pub fn build_symbol_registry(analyses: &[FileAnalysis]) -> HashMap<(String, String), SymbolEntry> {
    use crate::analyzer::classify::should_exclude_from_reports;
    let mut registry: HashMap<(String, String), SymbolEntry> = HashMap::new();

    // First pass: Register all exports
    for analysis in analyses {
        // Skip test files to avoid pytest/Jest fixtures being treated as dead parrots
        if analysis.is_test {
            continue;
        }
        // Skip test fixtures and mock files
        if should_exclude_from_reports(&analysis.path) {
            continue;
        }
        for export in &analysis.exports {
            let key = (analysis.path.clone(), export.name.clone());
            registry.insert(
                key,
                SymbolEntry {
                    name: export.name.clone(),
                    kind: export.kind.clone(),
                    file_path: analysis.path.clone(),
                    line: export.line.unwrap_or(0),
                    import_count: 0,
                },
            );
        }
    }

    // Second pass: Count imports
    for analysis in analyses {
        for import in &analysis.imports {
            // Get resolved path (or fall back to source)
            let target_path = import.resolved_path.as_ref().unwrap_or(&import.source);

            // Count each imported symbol
            for symbol in &import.symbols {
                let symbol_name = if symbol.is_default {
                    "default".to_string()
                } else {
                    symbol.name.clone()
                };

                let key = (target_path.clone(), symbol_name);
                if let Some(entry) = registry.get_mut(&key) {
                    entry.import_count += 1;
                }
            }
        }
    }

    registry
}

/// Check if a file is an entry point that shouldn't have dead parrot warnings
fn is_entry_point(path: &str) -> bool {
    // Rust crate roots
    path == "lib.rs"
        || path == "main.rs"
        || path.ends_with("/lib.rs")
        || path.ends_with("/main.rs")
        // TypeScript/JavaScript entry points
        || path.ends_with("/index.ts")
        || path.ends_with("/index.tsx")
        || path.ends_with("/index.js")
        || path.ends_with("/index.jsx")
        || path.ends_with("/index.mjs")
        // Python package roots
        || path.ends_with("/__init__.py")
        // Go package main
        || (path.ends_with(".go") && path.contains("/cmd/"))
}

/// Check if a file is a mod.rs (Rust module declaration file)
fn is_mod_rs(path: &str) -> bool {
    path == "mod.rs" || path.ends_with("/mod.rs")
}

/// Check if export is a common framework magic pattern
fn is_framework_magic(name: &str, kind: &str) -> bool {
    // Python dunder methods
    if name.starts_with("__") && name.ends_with("__") {
        return true;
    }
    // React/Svelte component conventions (PascalCase default exports)
    if kind == "default"
        && name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    {
        return true;
    }
    // Common framework hooks
    if name.starts_with("use")
        && name.len() > 3
        && name
            .chars()
            .nth(3)
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    {
        return true;
    }
    // Django/Python mixins
    if kind == "class" && name.ends_with("Mixin") {
        return true;
    }
    // Rust trait implementations often not directly imported
    if kind == "impl" || kind == "trait" {
        return true;
    }
    false
}

/// Check if symbol name is a common re-export pattern (barrel exports)
fn is_barrel_reexport(_name: &str, kind: &str) -> bool {
    // Re-exports are not dead - they're forwarding from another module
    kind == "re-export" || kind == "reexport"
}

/// Find dead parrots - symbols with 0 imports
pub fn find_dead_parrots(analyses: &[FileAnalysis], _dead_only: bool) -> TwinsResult {
    let registry = build_symbol_registry(analyses);

    // Build set of Tauri handlers (registered commands)
    let tauri_handlers: std::collections::HashSet<String> = analyses
        .iter()
        .flat_map(|a| a.tauri_registered_handlers.iter().cloned())
        .collect();

    // Build set of locally used symbols per file
    let all_local_uses: std::collections::HashSet<String> = analyses
        .iter()
        .flat_map(|a| a.local_uses.iter().cloned())
        .collect();

    // Build set of all imported symbol names (fallback for unresolved paths)
    let all_imported_names: std::collections::HashSet<String> = analyses
        .iter()
        .flat_map(|a| a.imports.iter())
        .flat_map(|imp| imp.symbols.iter())
        .map(|sym| {
            if sym.is_default {
                "default".to_string()
            } else {
                sym.name.clone()
            }
        })
        .collect();

    let mut dead_parrots: Vec<SymbolEntry> = registry
        .values()
        .filter(|entry| {
            // Skip if has imports
            if entry.import_count > 0 {
                return false;
            }
            // Skip Tauri commands
            if tauri_handlers.contains(&entry.name) {
                return false;
            }
            // Skip locally used symbols
            if all_local_uses.contains(&entry.name) {
                return false;
            }
            // Skip if imported by name anywhere (handles unresolved paths)
            if all_imported_names.contains(&entry.name) {
                return false;
            }
            // Skip entry points (lib.rs, index.ts, __init__.py, etc.)
            if is_entry_point(&entry.file_path) {
                return false;
            }
            // Skip mod.rs files (Rust module declarations)
            if is_mod_rs(&entry.file_path) {
                return false;
            }
            // Skip framework magic patterns
            if is_framework_magic(&entry.name, &entry.kind) {
                return false;
            }
            // Skip barrel re-exports
            if is_barrel_reexport(&entry.name, &entry.kind) {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    // Sort by file path, then symbol name for consistent output
    dead_parrots.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then_with(|| a.name.cmp(&b.name))
    });

    TwinsResult {
        dead_parrots,
        total_symbols: registry.len(),
        total_files: analyses.len(),
    }
}

/// Print twins results in human-readable format
pub fn print_twins_human(result: &TwinsResult) {
    if result.dead_parrots.is_empty() {
        println!("No dead parrots found - all exports are imported!");
        return;
    }

    println!("DEAD PARROTS ({} found)", result.dead_parrots.len());
    println!();

    // Group by file for cleaner output
    let mut by_file: HashMap<String, Vec<&SymbolEntry>> = HashMap::new();
    for entry in &result.dead_parrots {
        by_file
            .entry(entry.file_path.clone())
            .or_default()
            .push(entry);
    }

    let mut files: Vec<_> = by_file.keys().collect();
    files.sort();

    for file in files {
        let entries = &by_file[file];
        println!("  {}", file);
        for entry in entries {
            println!(
                "    ├─ {} ({}:{}) - {} imports",
                entry.name, entry.kind, entry.line, entry.import_count
            );
        }
        println!();
    }

    println!("Summary:");
    println!("  Total symbols: {}", result.total_symbols);
    println!("  Dead parrots: {}", result.dead_parrots.len());
    println!("  Files analyzed: {}", result.total_files);
}

/// Print twins results in JSON format
pub fn print_twins_json(result: &TwinsResult) {
    let output = serde_json::json!({
        "dead_parrots": result.dead_parrots.iter().map(|e| {
            serde_json::json!({
                "name": e.name,
                "file": e.file_path,
                "line": e.line,
                "kind": e.kind,
                "import_count": e.import_count,
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "symbols": result.total_symbols,
            "files": result.total_files,
            "dead_parrots": result.dead_parrots.len(),
        }
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

/// Print twins results based on output mode
pub fn print_twins_result(result: &TwinsResult, output: OutputMode) {
    match output {
        OutputMode::Json | OutputMode::Jsonl => print_twins_json(result),
        OutputMode::Human => print_twins_human(result),
    }
}

// ============================================================================
// EXACT TWIN DETECTION
// ============================================================================

/// A location where an exact twin symbol is found
#[derive(Clone, Debug, Serialize)]
pub struct TwinLocation {
    /// File path where the symbol is exported
    pub file_path: String,
    /// Line number (1-based)
    pub line: usize,
    /// Export kind: "export", "re-export", "type", "default", etc.
    pub kind: String,
    /// Number of imports of this specific export
    pub import_count: usize,
    /// True if this is the "source of truth" (canonical definition)
    pub is_canonical: bool,
}

/// An exact twin - a symbol exported from multiple files
#[derive(Clone, Debug, Serialize)]
pub struct ExactTwin {
    /// Symbol name
    pub name: String,
    /// All locations where this symbol is exported
    pub locations: Vec<TwinLocation>,
}

/// Generic method names that should be excluded from twin detection
const GENERIC_METHOD_NAMES: &[&str] = &[
    "new",
    "default",
    "from",
    "into",
    "clone",
    "process",
    "load",
    "save",
    "run",
    "init",
    "build",
    "execute",
    "create",
    "update",
    "delete",
    "get",
    "set",
    "handle",
    "render",
    "parse",
    "format",
    "serialize",
    "deserialize",
    "start",
    "stop",
    "reset",
    "clear",
    "close",
    "open",
    "read",
    "write",
    "send",
    "receive",
    "connect",
    "disconnect",
    "validate",
    "configure",
];

fn is_generic_method(name: &str) -> bool {
    GENERIC_METHOD_NAMES.contains(&name)
}

/// Detect exact twins: symbols with the same name exported from different files
pub fn detect_exact_twins(analyses: &[FileAnalysis]) -> Vec<ExactTwin> {
    let registry = build_symbol_registry(analyses);

    // Build map: symbol_name -> Vec<(file_path, line, kind, import_count)>
    let mut symbol_map: HashMap<String, Vec<(String, usize, String, usize)>> = HashMap::new();

    for ((file_path, symbol_name), entry) in &registry {
        // Skip re-exports - they're intentional API design, not duplicates
        if entry.kind == "reexport" || entry.kind == "re-export" {
            continue;
        }
        symbol_map.entry(symbol_name.clone()).or_default().push((
            file_path.clone(),
            entry.line,
            entry.kind.clone(),
            entry.import_count,
        ));
    }

    // Filter to only symbols exported from multiple files
    let mut twins: Vec<ExactTwin> = Vec::new();

    for (name, locations_raw) in symbol_map {
        // Skip generic method names (new, from, clone, etc.)
        if is_generic_method(&name) {
            continue;
        }

        // Skip if only one location (not a duplicate)
        if locations_raw.len() <= 1 {
            continue;
        }

        // Build locations with import counts
        let mut locations: Vec<TwinLocation> = locations_raw
            .iter()
            .map(|(file, line, kind, import_count)| TwinLocation {
                file_path: file.clone(),
                line: *line,
                kind: kind.clone(),
                import_count: *import_count,
                is_canonical: false, // Will determine below
            })
            .collect();

        // Determine canonical location:
        // 1. Most imports
        // 2. If tie, shortest path (likely more central)
        // 3. If still tie, first alphabetically (deterministic)
        if !locations.is_empty() {
            let max_imports = locations.iter().map(|l| l.import_count).max().unwrap_or(0);

            let mut canonicals: Vec<&mut TwinLocation> = locations
                .iter_mut()
                .filter(|l| l.import_count == max_imports)
                .collect();

            // If multiple have max imports, pick shortest path
            if canonicals.len() > 1 {
                canonicals.sort_by_key(|l| l.file_path.len());
            }

            // Mark first as canonical
            if let Some(canonical) = canonicals.first_mut() {
                canonical.is_canonical = true;
            }
        }

        twins.push(ExactTwin { name, locations });
    }

    // Sort by number of locations (most duplicated first)
    twins.sort_by(|a, b| b.locations.len().cmp(&a.locations.len()));

    twins
}

/// Print exact twins in human-readable format
pub fn print_exact_twins_human(twins: &[ExactTwin]) {
    if twins.is_empty() {
        println!("No exact twins found - all symbol names are unique!");
        return;
    }

    println!("👯 EXACT TWINS ({} found)", twins.len());
    println!();
    println!("  Multiple exports share the same name. CANONICAL = most imported version.");
    println!("  Consider: Consolidate duplicates, or rename if they serve different purposes.");
    println!();

    for twin in twins {
        println!("  Symbol: {}", twin.name);
        for loc in &twin.locations {
            let canonical_marker = if loc.is_canonical { " CANONICAL" } else { "" };
            println!(
                "    ├─ {}:{} ({}) - {} imports{}",
                loc.file_path, loc.line, loc.kind, loc.import_count, canonical_marker
            );
        }
        // Add suggestion based on import counts
        let zero_import_count = twin
            .locations
            .iter()
            .filter(|l| l.import_count == 0)
            .count();
        if zero_import_count > 0 && zero_import_count < twin.locations.len() {
            println!(
                "    └─ 💡 {} location(s) have 0 imports - candidates for removal or consolidation",
                zero_import_count
            );
        }
        println!();
    }

    println!("Summary:");
    println!("  Total symbol groups with twins: {}", twins.len());
    let total_dups: usize = twins.iter().map(|t| t.locations.len()).sum();
    println!("  Total duplicate definitions: {}", total_dups);
    println!();
    println!("  Note: Twins in FE↔BE projects (TS + Rust) are often intentional type mirrors.");
    println!("  Verify if duplicates serve the same purpose before consolidating.");
}

/// Print exact twins in JSON format
pub fn print_exact_twins_json(twins: &[ExactTwin]) {
    let output = serde_json::json!({
        "exact_twins": twins.iter().map(|twin| {
            serde_json::json!({
                "name": twin.name,
                "locations": twin.locations.iter().map(|loc| {
                    serde_json::json!({
                        "file": loc.file_path,
                        "line": loc.line,
                        "kind": loc.kind,
                        "imports": loc.import_count,
                        "canonical": loc.is_canonical,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "twins": twins.len(),
            "total_duplicates": twins.iter().map(|t| t.locations.len()).sum::<usize>(),
        }
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

/// Print exact twins based on output mode
pub fn print_exact_twins(twins: &[ExactTwin], output: OutputMode) {
    match output {
        OutputMode::Json | OutputMode::Jsonl => print_exact_twins_json(twins),
        OutputMode::Human => print_exact_twins_human(twins),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExportSymbol, ImportEntry, ImportKind, ImportSymbol};

    fn mock_file_with_exports(path: &str, exports: Vec<(&str, &str)>) -> FileAnalysis {
        FileAnalysis {
            path: path.to_string(),
            exports: exports
                .into_iter()
                .enumerate()
                .map(|(i, (name, kind))| ExportSymbol {
                    name: name.to_string(),
                    kind: kind.to_string(),
                    export_type: "named".to_string(),
                    line: Some(i + 1),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_build_symbol_registry_empty() {
        let analyses: Vec<FileAnalysis> = vec![];
        let registry = build_symbol_registry(&analyses);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_build_symbol_registry_no_imports() {
        let analyses = vec![
            mock_file_with_exports("a.ts", vec![("foo", "function")]),
            mock_file_with_exports("b.ts", vec![("bar", "function")]),
        ];

        let registry = build_symbol_registry(&analyses);
        assert_eq!(registry.len(), 2);

        let foo_entry = registry
            .get(&("a.ts".to_string(), "foo".to_string()))
            .unwrap();
        assert_eq!(foo_entry.import_count, 0);
    }

    #[test]
    fn test_build_symbol_registry_with_imports() {
        let exporter = mock_file_with_exports("utils.ts", vec![("helper", "function")]);
        let mut importer = FileAnalysis {
            path: "app.ts".to_string(),
            ..Default::default()
        };

        let mut import = ImportEntry::new("./utils".to_string(), ImportKind::Static);
        import.resolved_path = Some("utils.ts".to_string());
        import.symbols.push(ImportSymbol {
            name: "helper".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(import);

        let registry = build_symbol_registry(&[exporter, importer]);

        let helper_entry = registry
            .get(&("utils.ts".to_string(), "helper".to_string()))
            .unwrap();
        assert_eq!(helper_entry.import_count, 1);
    }

    #[test]
    fn test_build_symbol_registry_skips_tests() {
        let test_file = FileAnalysis {
            path: "tests/test_api_integration.py".to_string(),
            is_test: true,
            exports: vec![ExportSymbol {
                name: "TestHealthEndpoints".to_string(),
                kind: "class".to_string(),
                export_type: "named".to_string(),
                line: Some(10),
            }],
            ..Default::default()
        };
        let normal_file = mock_file_with_exports("app.py", vec![("App", "class")]);

        let registry = build_symbol_registry(&[test_file, normal_file]);

        // Only the non-test export should remain
        assert_eq!(registry.len(), 1);
        assert!(registry.contains_key(&("app.py".to_string(), "App".to_string())));
    }

    #[test]
    fn test_find_dead_parrots() {
        let used_file = mock_file_with_exports("used.ts", vec![("used", "function")]);
        let dead_file = mock_file_with_exports("dead.ts", vec![("unused", "function")]);

        let mut importer = FileAnalysis {
            path: "app.ts".to_string(),
            ..Default::default()
        };

        let mut import = ImportEntry::new("./used".to_string(), ImportKind::Static);
        import.resolved_path = Some("used.ts".to_string());
        import.symbols.push(ImportSymbol {
            name: "used".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(import);

        let result = find_dead_parrots(&[used_file, dead_file, importer], true);

        assert_eq!(result.dead_parrots.len(), 1);
        assert_eq!(result.dead_parrots[0].name, "unused");
        assert_eq!(result.total_symbols, 2);
    }

    // Exact twin detection tests
    #[test]
    fn test_detect_exact_twins_no_duplicates() {
        let analyses = vec![
            mock_file_with_exports("a.ts", vec![("foo", "function")]),
            mock_file_with_exports("b.ts", vec![("bar", "function")]),
        ];

        let twins = detect_exact_twins(&analyses);
        assert!(twins.is_empty());
    }

    #[test]
    fn test_detect_exact_twins_simple() {
        let analyses = vec![
            mock_file_with_exports("a.ts", vec![("Button", "class")]),
            mock_file_with_exports("b.ts", vec![("Button", "class")]),
        ];

        let twins = detect_exact_twins(&analyses);
        assert_eq!(twins.len(), 1);
        assert_eq!(twins[0].name, "Button");
        assert_eq!(twins[0].locations.len(), 2);
    }

    #[test]
    fn test_detect_exact_twins_canonical_by_path() {
        let analyses = vec![
            mock_file_with_exports("shared/types.ts", vec![("Message", "type")]),
            mock_file_with_exports("hooks/useChat.ts", vec![("Message", "type")]),
        ];

        let twins = detect_exact_twins(&analyses);
        assert_eq!(twins.len(), 1);

        // Canonical should be shortest path
        let canonical = twins[0].locations.iter().find(|l| l.is_canonical).unwrap();
        assert_eq!(canonical.file_path, "shared/types.ts");
    }

    #[test]
    fn test_detect_exact_twins_canonical_by_imports() {
        let a = mock_file_with_exports("a.ts", vec![("Foo", "type")]);
        let b = mock_file_with_exports("b.ts", vec![("Foo", "type")]);

        // Import from a.ts
        let mut importer = FileAnalysis {
            path: "app.ts".to_string(),
            ..Default::default()
        };
        let mut import = ImportEntry::new("./a".to_string(), ImportKind::Static);
        import.resolved_path = Some("a.ts".to_string());
        import.symbols.push(ImportSymbol {
            name: "Foo".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(import);

        let twins = detect_exact_twins(&[a, b, importer]);
        assert_eq!(twins.len(), 1);

        // Canonical should be the one with imports (a.ts)
        let canonical = twins[0].locations.iter().find(|l| l.is_canonical).unwrap();
        assert_eq!(canonical.file_path, "a.ts");
        assert_eq!(canonical.import_count, 1);
    }

    #[test]
    fn test_detect_exact_twins_three_locations() {
        let analyses = vec![
            mock_file_with_exports("a.ts", vec![("Common", "type")]),
            mock_file_with_exports("b.ts", vec![("Common", "type")]),
            mock_file_with_exports("c.ts", vec![("Common", "type")]),
        ];

        let twins = detect_exact_twins(&analyses);
        assert_eq!(twins.len(), 1);
        assert_eq!(twins[0].locations.len(), 3);
    }
}
