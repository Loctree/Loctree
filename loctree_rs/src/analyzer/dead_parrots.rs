//! Dead Parrots Module - Janitor tools for code analysis and cleanup
//!
//! Named after the Monty Python sketch and the Vista project's "Dead Parrot Protocol"
//! for identifying unused/dead code that "just resting" but is actually dead.
//!
//! This module contains:
//! - Symbol search (`--symbol`)
//! - Impact analysis (`--impact`)
//! - Similarity check (`--check`/`--sim`)
//! - Dead exports detection (`--dead`)

use std::collections::HashSet;

use serde_json::json;

use crate::similarity::similarity;
use crate::types::{FileAnalysis, OutputMode, ReexportKind};

use super::root_scan::{RootContext, normalize_module_id};

use serde::Serialize;

/// Compare two paths for equality using proper path matching
/// Handles different separators and avoids false positives like "foo.ts" matching "foo.test.ts"
fn paths_match(a: &str, b: &str) -> bool {
    // Quick exact match check first
    if a == b {
        return true;
    }

    // Normalize separators to forward slashes
    let a_norm = a.replace('\\', "/");
    let b_norm = b.replace('\\', "/");
    // Trim leading "./" to align relative specs with normalized paths
    let a_clean = a_norm.trim_start_matches("./");
    let b_clean = b_norm.trim_start_matches("./");

    if a_clean == b_clean {
        return true;
    }

    // Normalize to module ids (collapse extensions/index) and compare paths
    let mod_a = normalize_module_id(a_clean);
    let mod_b = normalize_module_id(b_clean);
    if mod_a.path == mod_b.path || mod_a.as_key() == mod_b.as_key() {
        return true;
    }

    // Check if one is a suffix of the other at a path component boundary
    // This handles "src/App.tsx" vs "App.tsx" but prevents "foo.ts" matching "foo.test.ts"
    if a_clean.len() > b_clean.len() {
        // Check if a ends with b at a component boundary
        if let Some(suffix_start) = a_clean.rfind(b_clean) {
            // Valid if b is at the start OR preceded by a separator
            if suffix_start == 0 || a_clean.chars().nth(suffix_start - 1) == Some('/') {
                return true;
            }
        }
    } else if b_clean.len() > a_clean.len() {
        // Check if b ends with a at a component boundary
        if let Some(suffix_start) = b_clean.rfind(a_clean) {
            // Valid if a is at the start OR preceded by a separator
            if suffix_start == 0 || b_clean.chars().nth(suffix_start - 1) == Some('/') {
                return true;
            }
        }
    }

    false
}

/// Result of symbol search across the codebase
#[derive(Debug, Clone, Serialize)]
pub struct SymbolSearchResult {
    pub found: bool,
    pub total_matches: usize,
    pub files: Vec<SymbolFileMatch>,
}

/// Matches in a single file
#[derive(Debug, Clone, Serialize)]
pub struct SymbolFileMatch {
    pub file: String,
    pub matches: Vec<SymbolMatch>,
}

/// A single symbol match
#[derive(Debug, Clone, Serialize)]
pub struct SymbolMatch {
    pub line: usize,
    pub context: String,
    pub is_definition: bool,
}

/// Result of impact analysis
#[derive(Debug, Clone, Serialize)]
pub struct ImpactResult {
    pub targets: Vec<String>,
    pub dependents: Vec<String>,
}

/// Result of similarity check
#[derive(Debug, Clone, Serialize)]
pub struct SimilarityCandidate {
    pub symbol: String,
    pub file: String,
    pub score: f64,
}

/// Result of dead exports analysis
#[derive(Debug, Clone, Serialize)]
pub struct DeadExport {
    pub file: String,
    pub symbol: String,
    pub line: Option<usize>,
    pub confidence: String,
    /// Human-readable reason explaining why this export is considered dead
    pub reason: String,
    /// IDE integration URL (loctree://open?f={file}&l={line})
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_url: Option<String>,
}

/// Search for symbol occurrences across analyzed files
/// Note: The actual symbol search is performed during file scanning (in `analyze_file`).
/// This function only collects the pre-computed matches from analyses.
pub fn search_symbol(_symbol: &str, analyses: &[FileAnalysis]) -> SymbolSearchResult {
    let mut files = Vec::new();
    let mut total_matches = 0;

    for analysis in analyses {
        if !analysis.matches.is_empty() {
            let mut matches = Vec::new();
            for m in &analysis.matches {
                // Infer if it's a definition from context keywords
                let ctx_lower = m.context.to_lowercase();
                let is_def = ctx_lower.contains("export ")
                    || ctx_lower.contains("pub ")
                    || ctx_lower.contains("function ")
                    || ctx_lower.contains("class ")
                    || ctx_lower.contains("const ")
                    || ctx_lower.contains("let ")
                    || ctx_lower.contains("var ")
                    || ctx_lower.starts_with("fn ");
                matches.push(SymbolMatch {
                    line: m.line,
                    context: m.context.clone(),
                    is_definition: is_def,
                });
            }
            total_matches += matches.len();
            files.push(SymbolFileMatch {
                file: analysis.path.clone(),
                matches,
            });
        }
    }

    SymbolSearchResult {
        found: !files.is_empty(),
        total_matches,
        files,
    }
}

/// Print symbol search results to stdout
pub fn print_symbol_results(symbol: &str, result: &SymbolSearchResult, json_output: bool) {
    if !result.found {
        eprintln!("No matches found for symbol '{}'", symbol);
        return;
    }

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&result)
                .expect("Failed to serialize symbol search results to JSON")
        );
    } else {
        println!("Symbol '{}' found in {} files:", symbol, result.files.len());
        for file_match in &result.files {
            println!("\nFile: {}", file_match.file);
            for m in &file_match.matches {
                println!("  {}: {}", m.line, m.context);
            }
        }
    }
}

/// Analyze impact of changing a file - find all files that depend on it
pub fn analyze_impact(
    target_path: &str,
    analyses: &[FileAnalysis],
    contexts: &[RootContext],
) -> Option<ImpactResult> {
    let mut targets = Vec::new();
    for analysis in analyses {
        // Use proper path matching to avoid false positives
        if paths_match(&analysis.path, target_path) {
            targets.push(analysis.path.clone());
        }
    }

    if targets.is_empty() {
        return None;
    }

    // Build target sets for both normalized and full paths
    let normalized_targets: HashSet<String> = targets
        .iter()
        .map(|t| normalize_module_id(t).as_key())
        .collect();
    let full_targets: HashSet<String> = targets.iter().cloned().collect();
    let mut dependent_ids = HashSet::new();

    for ctx in contexts {
        for (source, target, _weight) in &ctx.graph_edges {
            // Match against both normalized module IDs and full paths
            // (edges may use full paths after snapshot format changes)
            let target_normalized = normalize_module_id(target).as_key();
            if normalized_targets.contains(target)
                || normalized_targets.contains(&target_normalized)
                || full_targets.contains(target)
            {
                dependent_ids.insert(source.clone());
            }
        }
    }

    let mut deps = Vec::new();
    for analysis in analyses {
        // Match against both full path and normalized (edges may use either)
        let id = normalize_module_id(&analysis.path).as_key();
        if dependent_ids.contains(&id) || dependent_ids.contains(&analysis.path) {
            deps.push(analysis.path.clone());
        }
    }
    deps.sort();
    deps.dedup();

    Some(ImpactResult {
        targets,
        dependents: deps,
    })
}

/// Print impact analysis results to stdout
pub fn print_impact_results(target_path: &str, result: &ImpactResult, json_output: bool) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "target": result.targets,
                "dependents": result.dependents
            }))
            .unwrap_or_default()
        );
    } else {
        println!("Impact analysis for '{}':", target_path);
        println!("Matched targets: {:?}", result.targets);
        println!(
            "Files that import these targets ({}):",
            result.dependents.len()
        );
        for d in &result.dependents {
            println!("  - {}", d);
        }
    }
}

/// Find similar components/symbols in the codebase
pub fn find_similar(query: &str, analyses: &[FileAnalysis]) -> Vec<SimilarityCandidate> {
    let mut candidates: Vec<SimilarityCandidate> = Vec::new();

    for analysis in analyses {
        // Check file path similarity
        let path_score = similarity(query, &analysis.path);
        if path_score > 0.3 {
            candidates.push(SimilarityCandidate {
                symbol: analysis.path.clone(),
                file: "file path".to_string(),
                score: path_score,
            });
        }

        // Check exported symbols
        for exp in &analysis.exports {
            let sym_score = similarity(query, &exp.name);
            if sym_score > 0.4 {
                candidates.push(SimilarityCandidate {
                    symbol: exp.name.clone(),
                    file: format!("export in {}", analysis.path),
                    score: sym_score,
                });
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.dedup_by(|a, b| a.symbol == b.symbol && a.file == b.file);
    candidates.truncate(20);

    candidates
}

/// Print similarity check results to stdout
pub fn print_similarity_results(
    query: &str,
    candidates: &[SimilarityCandidate],
    json_output: bool,
) {
    if json_output {
        let json_items: Vec<_> = candidates
            .iter()
            .map(|c| {
                json!({
                    "symbol": c.symbol,
                    "file": c.file,
                    "score": c.score
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_items)
                .expect("Failed to serialize similarity results to JSON")
        );
    } else {
        println!("Checking for '{}' (similarity > 0.3):", query);
        if candidates.is_empty() {
            println!("  No similar components or symbols found.");
        } else {
            for c in candidates {
                println!("  - {} ({}) [score: {:.2}]", c.symbol, c.file, c.score);
            }
        }
    }
}

/// Check if a file should be skipped from dead export detection.
/// These are files whose exports are consumed by external tools/frameworks,
/// not by regular imports in the codebase.
fn should_skip_dead_export_check(analysis: &FileAnalysis) -> bool {
    let path = &analysis.path;

    // Test files and fixtures
    if analysis.is_test {
        return true;
    }

    // Test-related directories
    const TEST_DIRS: &[&str] = &[
        "stories",
        "__tests__",
        "__mocks__",
        "__fixtures__",
        "/cypress/",
        "/e2e/",
        "/playwright/",
        "/test/",
        "/tests/",
        "/spec/",
    ];
    if TEST_DIRS.iter().any(|d| path.contains(d)) {
        return true;
    }

    // TypeScript declaration files (.d.ts) - only contain type declarations
    if path.ends_with(".d.ts") {
        return true;
    }

    // Config files loaded dynamically by build tools (Vite, Jest, Cypress, etc.)
    if path.contains(".config.") || path.ends_with(".config.ts") || path.ends_with(".config.js") {
        return true;
    }

    // Framework routing/entry point conventions
    // SvelteKit: +page.ts, +layout.ts, +server.ts, +page.server.ts
    // Next.js: page.tsx, layout.tsx, route.ts (in app/ directory)
    const FRAMEWORK_ENTRY_PATTERNS: &[&str] = &[
        "+page.",
        "+layout.",
        "+server.",
        "+error.",
        "/page.tsx",
        "/page.ts",
        "/layout.tsx",
        "/layout.ts",
        "/route.ts",
        "/route.tsx",
        "/error.tsx",
        "/loading.tsx",
        "/not-found.tsx",
    ];
    if FRAMEWORK_ENTRY_PATTERNS.iter().any(|p| path.contains(p)) {
        return true;
    }

    false
}

/// Find potentially dead (unused) exports in the codebase
pub fn find_dead_exports(
    analyses: &[FileAnalysis],
    high_confidence: bool,
    open_base: Option<&str>,
) -> Vec<DeadExport> {
    // Build usage set: (resolved_path, symbol_name)
    let mut used_exports: HashSet<(String, String)> = HashSet::new();
    // Track all imported symbol names as fallback (handles $lib/, @scope/, monorepo paths)
    let mut all_imported_symbols: HashSet<String> = HashSet::new();

    for analysis in analyses {
        for imp in &analysis.imports {
            let target_norm = if let Some(target) = &imp.resolved_path {
                // Use resolved path if available
                normalize_module_id(target).as_key()
            } else {
                // Fallback to source for bare imports (e.g., npm packages)
                // This ensures we don't mark exports as dead when they're imported without resolution
                normalize_module_id(&imp.source).as_key()
            };

            // Track named imports
            for sym in &imp.symbols {
                let used_name = if sym.is_default {
                    "default".to_string()
                } else {
                    sym.name.clone()
                };
                used_exports.insert((target_norm.clone(), used_name.clone()));
                // Track all imported symbol names as fallback for unresolved/incorrectly resolved paths
                // This catches symbols imported via $lib/, @scope/, or other aliases that may not resolve correctly
                if !used_name.is_empty() {
                    all_imported_symbols.insert(used_name);
                }
            }
        }
        // Track re-exports as usage (if A re-exports B, A uses B)
        for re in &analysis.reexports {
            let target_norm = re
                .resolved
                .as_ref()
                .map(|t| normalize_module_id(t).as_key())
                .unwrap_or_else(|| normalize_module_id(&re.source).as_key());
            match &re.kind {
                ReexportKind::Star => {
                    used_exports.insert((target_norm, "*".to_string()));
                }
                ReexportKind::Named(names) => {
                    for name in names {
                        used_exports.insert((target_norm.clone(), name.clone()));
                    }
                }
            }
        }
    }

    // Build set of all Tauri registered command handlers (used via generate_handler![])
    let tauri_handlers: HashSet<String> = analyses
        .iter()
        .flat_map(|a| a.tauri_registered_handlers.iter().cloned())
        .collect();

    // Build transitive closure of files reachable from dynamic imports.
    // React.lazy(), Next.js dynamic(), and other code-splitting patterns use dynamic imports.
    // Files imported this way (and all their dependencies) should not be considered "dead".
    let dynamically_reachable: HashSet<String> = {
        // Build import graph: file_path -> list of resolved import paths
        let mut import_graph: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for analysis in analyses {
            let key = normalize_module_id(&analysis.path).as_key();
            let imports: Vec<String> = analysis
                .imports
                .iter()
                .filter_map(|imp| imp.resolved_path.as_ref())
                .map(|p| normalize_module_id(p).as_key())
                .collect();
            import_graph.insert(key, imports);
        }

        // Collect initial set of dynamically imported files
        let mut reachable: HashSet<String> = HashSet::new();
        for analysis in analyses {
            for dyn_imp in &analysis.dynamic_imports {
                let dyn_norm = normalize_module_id(dyn_imp).as_key();
                // Find matching file in analyses
                for a in analyses {
                    let a_norm = normalize_module_id(&a.path).as_key();
                    if paths_match(dyn_imp, &a_norm) || paths_match(dyn_imp, &a.path) {
                        reachable.insert(a_norm);
                        break;
                    }
                }
                // Also add the normalized dynamic import path itself
                reachable.insert(dyn_norm);
            }
        }

        // BFS to compute transitive closure
        let mut queue: std::collections::VecDeque<String> = reachable.iter().cloned().collect();
        while let Some(current) = queue.pop_front() {
            if let Some(imports) = import_graph.get(&current) {
                for imp in imports {
                    if !reachable.contains(imp) {
                        reachable.insert(imp.clone());
                        queue.push_back(imp.clone());
                    }
                }
            }
        }

        reachable
    };

    // Identify dead exports
    let mut dead_candidates = Vec::new();

    for analysis in analyses {
        // Skip files that should be excluded from dead export detection
        if should_skip_dead_export_check(analysis) {
            continue;
        }

        // Skip lib.rs and main.rs - they are crate entry points:
        // - lib.rs is the crate's public API, called via qualified paths like `crate_name::func()`
        // - main.rs is the binary entry point, its exports are not meant to be imported
        let is_crate_root = analysis.path == "lib.rs"
            || analysis.path == "main.rs"
            || analysis.path.ends_with("/lib.rs")
            || analysis.path.ends_with("/main.rs");
        if is_crate_root {
            continue;
        }

        let path_norm = normalize_module_id(&analysis.path).as_key();

        // Skip if file is reachable from dynamic imports (directly or transitively)
        // This handles React.lazy(), Next.js dynamic(), and other code-splitting patterns
        if dynamically_reachable.contains(&path_norm) {
            continue;
        }

        let local_uses: HashSet<_> = analysis.local_uses.iter().cloned().collect();

        for exp in &analysis.exports {
            if exp.kind == "reexport" {
                // Skip barrel bindings to avoid double-reporting re-exported symbols
                continue;
            }

            if exp.name == "default"
                && (analysis.path.ends_with("page.tsx") || analysis.path.ends_with("layout.tsx"))
            {
                // Next.js / framework roots - ignore default export
                continue;
            }

            if high_confidence && exp.name == "default" {
                // High confidence: ignore "default" exports (too often implicit usage)
                continue;
            }

            let is_used = used_exports.contains(&(path_norm.clone(), exp.name.clone()));
            // Also check if "*" was imported from this file
            let star_used = used_exports.contains(&(path_norm.clone(), "*".to_string()));
            let locally_used = local_uses.contains(&exp.name);
            // Check if this is a Tauri command handler registered via generate_handler![]
            let is_tauri_handler = tauri_handlers.contains(&exp.name);
            // Fallback: check if symbol is imported anywhere by name
            // This handles cases where path resolution fails (monorepos, $lib/, @scope/ packages)
            let imported_by_name = all_imported_symbols.contains(&exp.name);

            if !is_used && !star_used && !locally_used && !is_tauri_handler && !imported_by_name {
                let open_url = super::build_open_url(&analysis.path, exp.line, open_base);
                let is_rust_file = analysis.path.ends_with(".rs");

                // Build human-readable reason
                let reason = if is_rust_file {
                    format!(
                        "No imports found for '{}'. Checked: direct imports (0 matches), \
                         star imports (none), local uses (none), Tauri handlers (not registered)",
                        exp.name
                    )
                } else {
                    format!(
                        "No imports found for '{}'. Checked: resolved imports (0 matches), \
                         star re-exports (none), local references (none)",
                        exp.name
                    )
                };

                dead_candidates.push(DeadExport {
                    file: analysis.path.clone(),
                    symbol: exp.name.clone(),
                    line: exp.line,
                    confidence: if high_confidence {
                        "very-high".to_string()
                    } else {
                        "high".to_string()
                    },
                    reason,
                    open_url: Some(open_url),
                });
            }
        }
    }

    dead_candidates
}

/// Print dead exports results to stdout
pub fn print_dead_exports(
    dead_exports: &[DeadExport],
    output: OutputMode,
    high_confidence: bool,
    limit: usize,
) {
    if matches!(output, OutputMode::Json) {
        let json_items: Vec<_> = dead_exports
            .iter()
            .take(limit)
            .map(|d| {
                json!({
                    "file": d.file,
                    "symbol": d.symbol,
                    "line": d.line,
                    "confidence": d.confidence,
                    "reason": d.reason
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_items)
                .expect("Failed to serialize dead exports to JSON")
        );
    } else if matches!(output, OutputMode::Jsonl) {
        for item in dead_exports.iter().take(limit) {
            let json_line = json!({
                "file": item.file,
                "symbol": item.symbol,
                "line": item.line,
                "confidence": item.confidence,
                "reason": item.reason
            });
            println!(
                "{}",
                serde_json::to_string(&json_line).expect("Failed to serialize dead export to JSON")
            );
        }
    } else {
        let count = dead_exports.len();
        let suffix = if high_confidence {
            " (high confidence)"
        } else {
            ""
        };
        println!("Potential Dead Exports ({} found){}:", count, suffix);
        for item in dead_exports.iter().take(limit) {
            let location = match item.line {
                Some(line) => format!("{}:{}", item.file, line),
                None => item.file.clone(),
            };
            println!("  - {} in {}", item.symbol, location);
            println!("    Reason: {}", item.reason);
        }
        if count > limit {
            println!("  ... and {} more", count - limit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ExportSymbol, ImportEntry, ImportKind, ImportSymbol, ReexportEntry, ReexportKind,
        SymbolMatch as TypesSymbolMatch,
    };

    fn mock_file(path: &str) -> FileAnalysis {
        FileAnalysis {
            path: path.to_string(),
            ..Default::default()
        }
    }

    fn mock_file_with_exports(path: &str, exports: Vec<&str>) -> FileAnalysis {
        FileAnalysis {
            path: path.to_string(),
            exports: exports
                .into_iter()
                .enumerate()
                .map(|(i, name)| ExportSymbol {
                    name: name.to_string(),
                    kind: "function".to_string(),
                    export_type: "named".to_string(),
                    line: Some(i + 1),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn mock_file_with_matches(path: &str, matches: Vec<(usize, &str)>) -> FileAnalysis {
        FileAnalysis {
            path: path.to_string(),
            matches: matches
                .into_iter()
                .map(|(line, ctx)| TypesSymbolMatch {
                    line,
                    context: ctx.to_string(),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_search_symbol_empty() {
        let analyses: Vec<FileAnalysis> = vec![];
        let result = search_symbol("foo", &analyses);
        assert!(!result.found);
        assert!(result.files.is_empty());
    }

    #[test]
    fn test_search_symbol_no_matches() {
        let analyses = vec![mock_file("src/utils.ts"), mock_file("src/helpers.ts")];
        let result = search_symbol("foo", &analyses);
        assert!(!result.found);
    }

    #[test]
    fn test_search_symbol_with_matches() {
        let analyses = vec![
            mock_file_with_matches(
                "src/utils.ts",
                vec![(10, "const foo = 1"), (20, "return foo")],
            ),
            mock_file("src/helpers.ts"),
        ];
        let result = search_symbol("foo", &analyses);
        assert!(result.found);
        assert_eq!(result.files.len(), 1);
    }

    #[test]
    fn test_find_dead_exports_respects_from_imports() {
        let exporter = mock_file_with_exports("pkg/module.py", vec!["Foo"]);
        let mut importer = mock_file("main.py");
        let mut imp = ImportEntry::new("pkg.module".to_string(), ImportKind::Static);
        imp.resolved_path = Some("pkg/module.py".to_string());
        imp.symbols.push(ImportSymbol {
            name: "Foo".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(imp);

        let result = find_dead_exports(&[importer, exporter], false, None);
        assert!(
            result.is_empty(),
            "export imported with explicit symbol should not be dead"
        );
    }

    #[test]
    fn test_find_dead_exports_respects_local_usage() {
        let mut file = mock_file_with_exports("app.py", vec!["refresh"]);
        file.local_uses.push("refresh".to_string());
        let result = find_dead_exports(&[file], false, None);
        assert!(
            result.is_empty(),
            "locally referenced export should not be marked dead"
        );
    }

    #[test]
    fn test_print_symbol_results_no_matches() {
        let result = SymbolSearchResult {
            found: false,
            total_matches: 0,
            files: vec![],
        };
        // Should not panic
        print_symbol_results("foo", &result, false);
        print_symbol_results("foo", &result, true);
    }

    #[test]
    fn test_print_symbol_results_with_matches() {
        let result = SymbolSearchResult {
            found: true,
            total_matches: 1,
            files: vec![SymbolFileMatch {
                file: "src/utils.ts".to_string(),
                matches: vec![SymbolMatch {
                    line: 10,
                    context: "const foo = 1".to_string(),
                    is_definition: true,
                }],
            }],
        };
        // Should not panic
        print_symbol_results("foo", &result, false);
        print_symbol_results("foo", &result, true);
    }

    #[test]
    fn test_find_similar_empty() {
        let analyses: Vec<FileAnalysis> = vec![];
        let result = find_similar("Button", &analyses);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_similar_by_path() {
        let analyses = vec![mock_file("Button.tsx"), mock_file("src/utils/helpers.ts")];
        let result = find_similar("Button", &analyses);
        // Path similarity is computed against full path - shorter path gives higher score
        assert!(!result.is_empty());
        assert!(result.iter().any(|c| c.symbol.contains("Button")));
    }

    #[test]
    fn test_find_similar_by_export() {
        let analyses = vec![mock_file_with_exports(
            "src/utils.ts",
            vec!["useButton", "formatDate"],
        )];
        let result = find_similar("Button", &analyses);
        assert!(result.iter().any(|c| c.symbol == "useButton"));
    }

    #[test]
    fn test_print_similarity_results_empty() {
        let candidates: Vec<SimilarityCandidate> = vec![];
        // Should not panic
        print_similarity_results("foo", &candidates, false);
        print_similarity_results("foo", &candidates, true);
    }

    #[test]
    fn test_print_similarity_results_with_matches() {
        let candidates = vec![SimilarityCandidate {
            symbol: "fooBar".to_string(),
            file: "export in src/utils.ts".to_string(),
            score: 0.8,
        }];
        // Should not panic
        print_similarity_results("foo", &candidates, false);
        print_similarity_results("foo", &candidates, true);
    }

    #[test]
    fn test_find_dead_exports_empty() {
        let analyses: Vec<FileAnalysis> = vec![];
        let result = find_dead_exports(&analyses, false, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_dead_exports_all_used() {
        let mut importer = mock_file("src/app.ts");
        importer.imports = vec![{
            let mut imp = ImportEntry::new("./utils".to_string(), ImportKind::Static);
            imp.resolved_path = Some("src/utils.ts".to_string());
            imp.symbols = vec![ImportSymbol {
                name: "helper".to_string(),
                alias: None,
                is_default: false,
            }];
            imp
        }];

        let exporter = mock_file_with_exports("src/utils.ts", vec!["helper"]);

        let analyses = vec![importer, exporter];
        let result = find_dead_exports(&analyses, false, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_dead_exports_unused() {
        let analyses = vec![
            mock_file("src/app.ts"),
            mock_file_with_exports("src/utils.ts", vec!["unusedHelper"]),
        ];
        let result = find_dead_exports(&analyses, false, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].symbol, "unusedHelper");
    }

    #[test]
    fn test_find_dead_exports_skips_tests() {
        let mut test_file =
            mock_file_with_exports("src/__tests__/utils.test.ts", vec!["testHelper"]);
        test_file.is_test = true;

        let analyses = vec![mock_file("src/app.ts"), test_file];
        let result = find_dead_exports(&analyses, false, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_dead_exports_high_confidence_skips_default() {
        let analyses = vec![
            mock_file("src/app.ts"),
            mock_file_with_exports("src/utils.ts", vec!["default", "helper"]),
        ];
        let result = find_dead_exports(&analyses, true, None);
        assert!(!result.iter().any(|d| d.symbol == "default"));
    }

    #[test]
    fn test_find_dead_exports_skips_dynamic_import_without_extension() {
        let mut importer = mock_file("src/app.tsx");
        importer.dynamic_imports = vec!["./utils".to_string()];

        let exporter = mock_file_with_exports("src/utils/index.ts", vec!["foo"]);

        let result = find_dead_exports(&[importer, exporter], false, None);
        assert!(
            result.is_empty(),
            "dynamic import should mark module as used"
        );
    }

    #[test]
    fn test_find_dead_exports_counts_default_import_usage() {
        let mut importer = mock_file("src/app.ts");
        importer.imports = vec![{
            let mut imp = ImportEntry::new("./utils".to_string(), ImportKind::Static);
            imp.resolved_path = Some("src/utils.ts".to_string());
            imp.symbols = vec![ImportSymbol {
                name: "AliasDefault".to_string(),
                alias: None,
                is_default: true,
            }];
            imp
        }];

        let mut exporter = mock_file_with_exports("src/utils.ts", vec!["default"]);
        exporter.exports[0].kind = "default".to_string();
        exporter.exports[0].export_type = "default".to_string();

        let result = find_dead_exports(&[importer, exporter], false, None);
        assert!(
            result.is_empty(),
            "default import should mark export as used"
        );
    }

    #[test]
    fn test_find_dead_exports_skips_reexport_bindings() {
        let mut barrel = mock_file_with_exports("src/index.ts", vec!["Foo"]);
        if let Some(first) = barrel.exports.first_mut() {
            first.kind = "reexport".to_string();
        }
        barrel.reexports.push(ReexportEntry {
            source: "./foo".to_string(),
            kind: ReexportKind::Named(vec!["Foo".to_string()]),
            resolved: Some("src/foo.ts".to_string()),
        });

        let result = find_dead_exports(&[barrel], false, None);
        assert!(
            result.is_empty(),
            "reexport-only barrels should not be reported as dead exports"
        );
    }

    #[test]
    fn test_print_dead_exports_json() {
        let dead = vec![DeadExport {
            file: "src/utils.ts".to_string(),
            symbol: "unused".to_string(),
            line: Some(10),
            confidence: "high".to_string(),
            reason: "No imports found for 'unused'. Checked: resolved imports (0 matches), star re-exports (none), local references (none)".to_string(),
            open_url: Some("loctree://open?f=src%2Futils.ts&l=10".to_string()),
        }];
        // Should not panic
        print_dead_exports(&dead, OutputMode::Json, false, 20);
    }

    #[test]
    fn test_print_dead_exports_human() {
        let dead = vec![DeadExport {
            file: "src/utils.ts".to_string(),
            symbol: "unused".to_string(),
            line: None,
            confidence: "high".to_string(),
            reason: "No imports found for 'unused'. Checked: resolved imports (0 matches), star re-exports (none), local references (none)".to_string(),
            open_url: None,
        }];
        // Should not panic
        print_dead_exports(&dead, OutputMode::Human, false, 20);
        print_dead_exports(&dead, OutputMode::Human, true, 20);
    }

    #[test]
    fn test_print_dead_exports_many() {
        let dead: Vec<DeadExport> = (0..60)
            .map(|i| DeadExport {
                file: format!("src/file{}.ts", i),
                symbol: format!("unused{}", i),
                line: Some(i),
                confidence: "high".to_string(),
                reason: format!("No imports found for 'unused{}'. Checked: resolved imports (0 matches), star re-exports (none), local references (none)", i),
                open_url: Some(format!("loctree://open?f=src%2Ffile{}.ts&l={}", i, i)),
            })
            .collect();
        // Should truncate to limit and show "... and N more"
        print_dead_exports(&dead, OutputMode::Human, false, 50);
    }

    #[test]
    fn test_paths_match_exact() {
        assert!(paths_match("src/App.tsx", "src/App.tsx"));
        assert!(paths_match("foo.ts", "foo.ts"));
    }

    #[test]
    fn test_paths_match_with_separators() {
        // Should handle different separators
        assert!(paths_match("src/App.tsx", "src\\App.tsx"));
        assert!(paths_match(
            "src\\components\\Button.tsx",
            "src/components/Button.tsx"
        ));
    }

    #[test]
    fn test_paths_match_normalizes_index_and_extension() {
        assert!(paths_match("src/utils/index.ts", "./utils"));
        assert!(paths_match("src/components/Foo.tsx", "src/components/Foo"));
        assert!(paths_match("components/Foo.tsx", "components/Foo.jsx"));
    }

    #[test]
    fn test_paths_match_suffix() {
        // Should match when one is a suffix of another at component boundary
        assert!(paths_match("src/App.tsx", "App.tsx"));
        assert!(paths_match("src/components/Button.tsx", "Button.tsx"));
        assert!(paths_match("Button.tsx", "src/components/Button.tsx"));
    }

    #[test]
    fn test_paths_match_no_false_positives() {
        // Should NOT match foo.ts with foo.test.ts (this is the critical fix)
        assert!(!paths_match("foo.ts", "foo.test.ts"));
        assert!(!paths_match("Button.tsx", "Button.test.tsx"));
        assert!(!paths_match("utils.ts", "utils.spec.ts"));

        // Should NOT match when substring is in the middle
        assert!(!paths_match("App.tsx", "src/MyApp.tsx"));
        assert!(!paths_match("Button.tsx", "src/BigButton.tsx"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::types::{ExportSymbol, ImportEntry, ImportKind, ImportSymbol};

    #[test]
    fn test_recommendations_pdf_not_dead() {
        let mut importer = FileAnalysis {
            path: "src/services/recommendationsExportService.ts".to_string(),
            ..Default::default()
        };
        let mut imp = ImportEntry::new(
            "../components/pdf/RecommendationsPDFTemplate".to_string(),
            ImportKind::Static,
        );
        imp.resolved_path = Some("src/components/pdf/RecommendationsPDFTemplate.tsx".to_string());
        imp.symbols.push(ImportSymbol {
            name: "RecommendationsPDFTemplate".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(imp);

        let exporter = FileAnalysis {
            path: "src/components/pdf/RecommendationsPDFTemplate.tsx".to_string(),
            exports: vec![ExportSymbol {
                name: "RecommendationsPDFTemplate".to_string(),
                kind: "function".to_string(),
                export_type: "named".to_string(),
                line: Some(25),
            }],
            ..Default::default()
        };

        let result = find_dead_exports(&[importer, exporter], false, None);
        assert!(
            result.is_empty(),
            "RecommendationsPDFTemplate should NOT be dead. Found: {:?}",
            result
        );
    }
}
