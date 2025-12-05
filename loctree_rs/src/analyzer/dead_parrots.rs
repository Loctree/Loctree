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
use std::fs;

use serde_json::json;

use crate::similarity::similarity;
use crate::types::{ExportSymbol, FileAnalysis, OutputMode, ReexportKind};

use super::root_scan::{RootContext, normalize_module_id};

fn strip_alias_prefix(path: &str) -> &str {
    // Drop leading alias markers like @core/... -> core/...
    let without_at = path.trim_start_matches('@');
    if let Some(idx) = without_at.find('/') {
        &without_at[idx + 1..]
    } else {
        without_at
    }
}

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
    let a_clean = a_norm.trim_start_matches("./").to_string();
    let b_clean = b_norm.trim_start_matches("./").to_string();

    // On Windows, compare case-insensitively to avoid false mismatches on case variants
    let (a_clean, b_clean) = if cfg!(windows) {
        (a_clean.to_lowercase(), b_clean.to_lowercase())
    } else {
        (a_clean, b_clean)
    };

    if a_clean == b_clean {
        return true;
    }

    // Also allow alias-stripped comparisons (e.g., @core/utils vs src/core/utils)
    let a_alias = strip_alias_prefix(&a_clean);
    let b_alias = strip_alias_prefix(&b_clean);
    if a_alias == b_clean || b_alias == a_clean || a_alias == b_alias {
        return true;
    }

    // Normalize to module ids (collapse extensions/index) and compare paths
    let mod_a = normalize_module_id(&a_clean);
    let mod_b = normalize_module_id(&b_clean);
    if mod_a.path == mod_b.path || mod_a.as_key() == mod_b.as_key() {
        return true;
    }

    // Check if one is a suffix of the other at a path component boundary
    // This handles "src/App.tsx" vs "App.tsx" but prevents "foo.ts" matching "foo.test.ts"
    if a_clean.len() > b_clean.len() {
        // Check if a ends with b at a component boundary
        if let Some(suffix_start) = a_clean.rfind(&b_clean) {
            // Valid if b is at the start OR preceded by a separator
            if suffix_start == 0 || a_clean.chars().nth(suffix_start - 1) == Some('/') {
                return true;
            }
        }
    } else if b_clean.len() > a_clean.len() {
        // Check if b ends with a at a component boundary
        if let Some(suffix_start) = b_clean.rfind(&a_clean) {
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

/// Controls which files are considered during dead-export detection.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeadFilterConfig {
    /// Include tests and fixtures (default: false)
    pub include_tests: bool,
    /// Include helper/scripts/docs files (default: false)
    pub include_helpers: bool,
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
fn should_skip_dead_export_check(analysis: &FileAnalysis, config: DeadFilterConfig) -> bool {
    let path = &analysis.path;

    // Test files and fixtures
    if analysis.is_test && !config.include_tests {
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
    if TEST_DIRS.iter().any(|d| path.contains(d)) && !config.include_tests {
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

    if !config.include_helpers {
        const HELPER_DIRS: &[&str] = &["/scripts/", "/script/", "/tools/", "/docs/"];
        if HELPER_DIRS.iter().any(|d| path.contains(d))
            || path.starts_with("scripts/")
            || path.starts_with("script/")
            || path.starts_with("tools/")
            || path.starts_with("docs/")
        {
            return true;
        }
    }

    // Framework routing/entry point conventions
    // SvelteKit: +page.ts, +layout.ts, +server.ts, +page.server.ts, hooks.*.ts
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
        // SvelteKit hooks (auto-loaded by framework)
        "hooks.client.",
        "hooks.server.",
        "/hooks.",
    ];
    if FRAMEWORK_ENTRY_PATTERNS.iter().any(|p| path.contains(p)) {
        return true;
    }

    false
}

/// Check if an export from a Svelte file is likely a component API method.
/// Svelte components expose methods via `export function` that are called via `bind:this`:
///   let modal: MyModal;
///   <MyModal bind:this={modal} />
///   modal.show();  // calling the exported function
///
/// These are NOT imported via ES imports, so they appear as "dead" in static analysis.
/// Common patterns: show/hide/open/close for modals, focus/blur for inputs, scroll* for containers.
fn is_svelte_component_api(file_path: &str, export_name: &str) -> bool {
    // Only applies to .svelte and .svelte.ts files (Svelte modules)
    let is_svelte_file = file_path.ends_with(".svelte") || file_path.ends_with(".svelte.ts");
    if !is_svelte_file {
        return false;
    }

    // Common component API method names used via bind:this
    const COMPONENT_API_METHODS: &[&str] = &[
        // Modal/dialog patterns
        "show",
        "hide",
        "open",
        "close",
        "toggle",
        "dismiss",
        // Form/input patterns
        "focus",
        "blur",
        "select",
        "selectAll",
        "clear",
        "reset",
        "validate",
        "submit",
        // Text/editor patterns
        "getText",
        "setText",
        "getValue",
        "setValue",
        "getContent",
        "setContent",
        "insertText",
        "replaceText",
        // Scroll patterns
        "scrollTo",
        "scrollToTop",
        "scrollToBottom",
        "scrollIntoView",
        // Animation/transition patterns
        "play",
        "pause",
        "stop",
        "restart",
        "animate",
        // State patterns
        "enable",
        "disable",
        "activate",
        "deactivate",
        "expand",
        "collapse",
        // Lifecycle patterns
        "init",
        "destroy",
        "refresh",
        "update",
        "reload",
        // Svelte reactive getter object patterns (exposed via bind:this)
        "imports",
        "exports",
        "getters",
        "state",
        "values",
    ];

    // Check exact match
    if COMPONENT_API_METHODS.contains(&export_name) {
        return true;
    }

    // Check prefix patterns (e.g., scrollToElement, setFoo, getFoo, applyPr, isActive)
    // These are common patterns for component methods called via bind:this
    const API_PREFIXES: &[&str] = &[
        "scroll",
        "get",
        "set",
        "on",
        "handle",
        "apply",
        "is",
        "has",
        "can",
        "should",
        "do",
        "trigger",
        "emit",
        "fire",
        "dispatch",
        "notify",
        "load",
        "fetch",
        "save",
        "delete",
        "add",
        "remove",
        "insert",
        "append",
        "prepend",
        "move",
        "swap",
        "sort",
        "filter",
        "find",
        "search",
        "check",
        "verify",
        "compute",
        "calculate",
        "render",
        "draw",
        // CRUD patterns
        "create",
        "update",
        "edit",
        "reset",
        "clear",
        "refresh",
        "submit",
        // Navigation/UI patterns
        "show",
        "hide",
        "open",
        "close",
        "toggle",
        "select",
        "click",
        "press",
        // Validation patterns
        "validate",
        "sanitize",
        "normalize",
        "format",
        "parse",
        "serialize",
        "deserialize",
    ];
    for prefix in API_PREFIXES {
        if export_name.starts_with(prefix)
            && export_name.len() > prefix.len()
            && export_name
                .chars()
                .nth(prefix.len())
                .is_some_and(|c| c.is_uppercase())
        {
            return true;
        }
    }

    false
}

/// Find potentially dead (unused) exports in the codebase
pub fn find_dead_exports(
    analyses: &[FileAnalysis],
    high_confidence: bool,
    open_base: Option<&str>,
    config: DeadFilterConfig,
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

    // Build set of all path-qualified symbols from Rust files
    // These are calls like `command::branch::handle()` that don't use `use` imports
    let rust_path_qualified_symbols: HashSet<String> = analyses
        .iter()
        .filter(|a| a.path.ends_with(".rs"))
        .flat_map(|a| a.local_uses.iter().cloned())
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
                let dyn_norm = normalize_module_id(dyn_imp);
                let dyn_key = dyn_norm.as_key();
                let dyn_alias = strip_alias_prefix(&dyn_norm.path).to_string();
                // Find matching file in analyses
                for a in analyses {
                    let a_norm = normalize_module_id(&a.path);
                    let a_key = a_norm.as_key();
                    if paths_match(dyn_imp, &a_norm.path)
                        || paths_match(dyn_imp, &a.path)
                        || a_norm.path.starts_with(&dyn_norm.path)
                        || a_norm.path.starts_with(&dyn_alias)
                        || a_norm.path.ends_with(&dyn_alias)
                    {
                        reachable.insert(a_key);
                        break;
                    }
                }
                // Also add the normalized dynamic import path itself
                reachable.insert(dyn_key.clone());
                // Alias-prefix fallback for unresolvable module ids (e.g., @core/foo)
                if !dyn_alias.is_empty() {
                    reachable.insert(dyn_alias.clone());
                }
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
        if should_skip_dead_export_check(analysis, config) {
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
            let is_rust_file = analysis.path.ends_with(".rs");
            if exp.kind == "reexport" {
                // Skip barrel bindings to avoid double-reporting re-exported symbols
                continue;
            }

            // Rust-specific heuristics: skip common macro-derived public types and CLI args
            let rust_macro_marked = is_rust_file
                && rust_has_known_derives(
                    &analysis.path,
                    &[
                        "serialize",
                        "deserialize",
                        "parser",
                        "args",
                        "valueenum",
                        "subcommand",
                        "fromargmatches",
                    ],
                );
            let rust_cli_pattern = is_rust_file
                && (exp.name.ends_with("Args")
                    || exp.name.ends_with("Command")
                    || exp.name.ends_with("Response")
                    || exp.name.ends_with("Request"));
            if rust_macro_marked || rust_cli_pattern {
                continue;
            }

            // Python-specific heuristics: skip framework magic patterns
            let is_python_file = analysis.path.ends_with(".py");
            let python_framework_magic = is_python_file
                && (
                    // arq framework looks up WorkerSettings by name convention
                    exp.name == "WorkerSettings"
                    // Standard Python package versioning
                    || exp.name == "__version__"
                    // pytest fixtures can be used without explicit import (via conftest.py)
                    || (analysis.path.contains("conftest") && exp.kind == "def")
                );
            if python_framework_magic {
                continue;
            }
            if is_python_test_export(analysis, exp) {
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
            // Check if this is likely a Svelte component API method (called via bind:this)
            let is_svelte_api = is_svelte_component_api(&analysis.path, &exp.name);
            // Check if this Rust symbol is called via path qualification (e.g., `module::func()`)
            let is_rust_path_qualified =
                analysis.path.ends_with(".rs") && rust_path_qualified_symbols.contains(&exp.name);

            if !is_used
                && !star_used
                && !locally_used
                && !is_tauri_handler
                && !imported_by_name
                && !is_svelte_api
                && !is_rust_path_qualified
            {
                let open_url = super::build_open_url(&analysis.path, exp.line, open_base);

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

fn rust_has_known_derives(path: &str, keywords: &[&str]) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    let lower = content.to_lowercase();
    lower.contains("derive(") && keywords.iter().any(|kw| lower.contains(kw))
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

        let result = find_dead_exports(
            &[importer, exporter],
            false,
            None,
            DeadFilterConfig::default(),
        );
        assert!(
            result.is_empty(),
            "export imported with explicit symbol should not be dead"
        );
    }

    #[test]
    fn test_find_dead_exports_respects_local_usage() {
        let mut file = mock_file_with_exports("app.py", vec!["refresh"]);
        file.local_uses.push("refresh".to_string());
        let result = find_dead_exports(&[file], false, None, DeadFilterConfig::default());
        assert!(
            result.is_empty(),
            "locally referenced export should not be marked dead"
        );
    }

    #[test]
    fn test_find_dead_exports_respects_type_imports() {
        let exporter = mock_file_with_exports("client/actions.ts", vec!["Action"]);
        let mut importer = mock_file("client/state.ts");
        let mut imp = ImportEntry::new("client/actions".to_string(), ImportKind::Type);
        imp.resolved_path = Some("client/actions.ts".to_string());
        imp.symbols.push(ImportSymbol {
            name: "Action".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(imp);

        let result = find_dead_exports(
            &[importer, exporter],
            true,
            None,
            DeadFilterConfig::default(),
        );
        assert!(
            result.is_empty(),
            "type-only import should count as usage for dead export detection"
        );
    }

    #[test]
    fn test_find_dead_exports_cross_extension_match() {
        let exporter = mock_file_with_exports("src/ComboBox.tsx", vec!["ComboBox"]);
        let mut importer = mock_file("src/app.js");
        let mut imp = ImportEntry::new("./ComboBox".to_string(), ImportKind::Static);
        imp.resolved_path = Some("src/ComboBox.tsx".to_string());
        imp.symbols.push(ImportSymbol {
            name: "ComboBox".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(imp);

        let result = find_dead_exports(
            &[importer, exporter],
            false,
            None,
            DeadFilterConfig::default(),
        );
        assert!(
            result.is_empty(),
            "imports across JS/TSX extensions should prevent dead export marking"
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
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());
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
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_dead_exports_unused() {
        let analyses = vec![
            mock_file("src/app.ts"),
            mock_file_with_exports("src/utils.ts", vec!["unusedHelper"]),
        ];
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].symbol, "unusedHelper");
    }

    #[test]
    fn test_find_dead_exports_skips_tests() {
        let mut test_file =
            mock_file_with_exports("src/__tests__/utils.test.ts", vec!["testHelper"]);
        test_file.is_test = true;

        let analyses = vec![mock_file("src/app.ts"), test_file];
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_dead_exports_includes_tests_when_requested() {
        let mut test_file =
            mock_file_with_exports("src/__tests__/utils.test.ts", vec!["testHelper"]);
        test_file.is_test = true;

        let analyses = vec![mock_file("src/app.ts"), test_file];
        let result = find_dead_exports(
            &analyses,
            false,
            None,
            DeadFilterConfig {
                include_tests: true,
                include_helpers: false,
            },
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].symbol, "testHelper");
    }

    #[test]
    fn test_find_dead_exports_skips_helpers_by_default() {
        let helper = mock_file_with_exports("scripts/cleanup.py", vec!["orphan"]);
        let analyses = vec![mock_file("src/app.ts"), helper];
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());
        assert!(result.is_empty(), "helper scripts should be skipped");
    }

    #[test]
    fn test_find_dead_exports_high_confidence_skips_default() {
        let analyses = vec![
            mock_file("src/app.ts"),
            mock_file_with_exports("src/utils.ts", vec!["default", "helper"]),
        ];
        let result = find_dead_exports(&analyses, true, None, DeadFilterConfig::default());
        assert!(!result.iter().any(|d| d.symbol == "default"));
    }

    #[test]
    fn test_find_dead_exports_skips_dynamic_import_without_extension() {
        let mut importer = mock_file("src/app.tsx");
        importer.dynamic_imports = vec!["./utils".to_string()];

        let exporter = mock_file_with_exports("src/utils/index.ts", vec!["foo"]);

        let result = find_dead_exports(
            &[importer, exporter],
            false,
            None,
            DeadFilterConfig::default(),
        );
        assert!(
            result.is_empty(),
            "dynamic import should mark module as used"
        );
    }

    #[test]
    fn test_dynamic_import_with_alias_prefix_marks_reachable() {
        let mut importer = mock_file("src/app.ts");
        importer.dynamic_imports = vec!["@core/utils".to_string()];

        let exporter = mock_file_with_exports("src/core/utils/index.ts", vec!["helper"]);

        let result = find_dead_exports(
            &[importer, exporter],
            false,
            None,
            DeadFilterConfig::default(),
        );
        assert!(
            result.is_empty(),
            "alias-prefixed dynamic import should keep target reachable"
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

        let result = find_dead_exports(
            &[importer, exporter],
            false,
            None,
            DeadFilterConfig::default(),
        );
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

        let result = find_dead_exports(&[barrel], false, None, DeadFilterConfig::default());
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

        let result = find_dead_exports(
            &[importer, exporter],
            false,
            None,
            DeadFilterConfig::default(),
        );
        assert!(
            result.is_empty(),
            "RecommendationsPDFTemplate should NOT be dead. Found: {:?}",
            result
        );
    }
}
fn is_python_test_export(analysis: &FileAnalysis, exp: &ExportSymbol) -> bool {
    if !analysis.path.ends_with(".py") || !analysis.is_test {
        return false;
    }
    if exp.kind == "class" && exp.name.starts_with("Test") {
        return true;
    }
    if exp.kind == "def" && exp.name.starts_with("test_") {
        return true;
    }
    false
}
