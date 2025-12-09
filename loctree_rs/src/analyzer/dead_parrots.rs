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

use std::collections::{HashMap, HashSet};
use std::fs;

use globset::GlobSet;
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
#[derive(Debug, Clone, Default)]
pub struct DeadFilterConfig {
    /// Include tests and fixtures (default: false)
    pub include_tests: bool,
    /// Include helper/scripts/docs files (default: false)
    pub include_helpers: bool,
    /// Treat project as library/framework (ignore examples/demos noise)
    pub library_mode: bool,
    /// Extra example/demo globs to ignore when library_mode is enabled
    pub example_globs: Vec<String>,
    /// Python library mode: exports in __all__ are public API, not dead
    pub python_library_mode: bool,
}

/// Search for symbol occurrences across analyzed files (case-insensitive, substring).
/// Falls back to export list so it works even without `--symbol` pre-scan.
pub fn search_symbol(symbol: &str, analyses: &[FileAnalysis]) -> SymbolSearchResult {
    let needle = symbol.to_lowercase();
    let mut files = Vec::new();
    let mut total_matches = 0;

    for analysis in analyses {
        let mut matches = Vec::new();

        // 1) Recorded line matches (only present if scan was run with --symbol)
        for m in &analysis.matches {
            let ctx_lower = m.context.to_lowercase();
            if ctx_lower.contains(&needle) {
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
        }

        // 2) Exports list (always available) - substring / case-insensitive
        for exp in &analysis.exports {
            if exp.name.to_lowercase().contains(&needle) {
                matches.push(SymbolMatch {
                    line: exp.line.unwrap_or(0),
                    context: format!("export {} {}", exp.kind, exp.name),
                    is_definition: true,
                });
            }
        }

        if !matches.is_empty() {
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
fn should_skip_dead_export_check(
    analysis: &FileAnalysis,
    config: &DeadFilterConfig,
    example_globs: Option<&GlobSet>,
) -> bool {
    let path = &analysis.path;
    let lower_path = path.to_ascii_lowercase();

    // Go exports are primarily public API; static import graph is insufficient (FP-heavy)
    // Skip dead-export detection for Go to avoid noise.
    if analysis.language == "go" {
        return true;
    }

    // JSX runtime files - exports consumed by TypeScript/Babel compiler, not by imports
    // Files matching: *jsx-runtime*, jsx-runtime.js, jsx-runtime/index.js, jsx-dev-runtime.js, etc.
    if (analysis.language == "ts" || analysis.language == "js")
        && (lower_path.contains("jsx-runtime")
            || lower_path.contains("jsx_runtime")
            || lower_path.contains("jsx-dev-runtime"))
    {
        return true;
    }

    // Test files and fixtures
    if analysis.is_test && !config.include_tests {
        return true;
    }

    // Flutter generated/plugin registrant files
    if path.ends_with("generated_plugin_registrant.dart")
        || path.contains("/generated_plugin_registrant.dart")
    {
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
    if TEST_DIRS.iter().any(|d| lower_path.contains(d)) && !config.include_tests {
        return true;
    }

    // Example/demo/fixture packages (library-mode noise)
    const EXAMPLE_DIRS: &[&str] = &[
        "/examples/",
        "/example/",
        "/samples/",
        "/sample/",
        "/demo/",
        "/demos/",
        "/playground/",
        "/showcase/",
    ];
    if EXAMPLE_DIRS.iter().any(|d| lower_path.contains(d)) {
        return true;
    }
    if config.library_mode {
        if let Some(globs) = example_globs
            && (globs.is_match(path) || globs.is_match(&lower_path))
        {
            return true;
        }
        const LIBRARY_NOISE_DIRS: &[&str] = &[
            "/kitchen-sink/",
            "/kitchensink/",
            "/sandbox/",
            "/sandboxes/",
            "/cookbook/",
            "/gallery/",
            "/examples-",
            "/examples_",
            "/docs/examples/",
            "/documentation/examples/",
        ];
        if LIBRARY_NOISE_DIRS.iter().any(|d| lower_path.contains(d))
            || lower_path.starts_with("examples/")
            || lower_path.starts_with("example/")
            || lower_path.starts_with("demo/")
            || lower_path.contains("/examples/")
        {
            return true;
        }
    }
    // Lowercase check for testfixtures (common in codemods)
    if lower_path.contains("testfixtures") {
        return true;
    }

    // TypeScript declaration files (.d.ts) - only contain type declarations
    if path.ends_with(".d.ts") {
        return true;
    }

    // Dart/Flutter generated artifacts
    if path.ends_with(".g.dart")
        || path.ends_with(".freezed.dart")
        || path.ends_with(".gr.dart")
        || path.ends_with(".pb.dart")
        || path.ends_with(".pbjson.dart")
        || path.ends_with(".pbenum.dart")
        || path.ends_with(".pbserver.dart")
        || path.ends_with(".config.dart")
    {
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

    // Virtual/module-runtime entrypoints (framework-provided consumers)
    const RUNTIME_PATTERNS: &[&str] =
        &["/.svelte-kit/", "/runtime/", "/app/router/", "/app/routes/"];
    if (analysis.language == "ts" || analysis.language == "js")
        && RUNTIME_PATTERNS.iter().any(|p| path.contains(p))
    {
        return true;
    }

    // Library barrels and public API surfaces (avoid flagging public exports)
    if (path.ends_with("/index.ts")
        || path.ends_with("/index.tsx")
        || path.ends_with("/index.js")
        || path.ends_with("/index.mjs")
        || path.ends_with("/index.cjs")
        || path.ends_with("/mod.ts")
        || path.ends_with("/mod.js"))
        && (analysis.language == "ts" || analysis.language == "js")
        && (path.contains("/packages/") || path.contains("/libs/") || path.contains("/library/"))
    {
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

/// Check if an export is a JSX runtime export consumed by compilers.
/// These exports are used by TypeScript/Babel when compiling JSX, configured via tsconfig.json:
/// { "jsx": "react-jsx", "jsxImportSource": "solid-js" }
/// The compiler transforms JSX into calls to these functions without explicit imports.
fn is_jsx_runtime_export(export_name: &str, file_path: &str) -> bool {
    // JSX runtime export names defined by React JSX transform spec
    const JSX_RUNTIME_EXPORTS: &[&str] = &["jsx", "jsxs", "jsxDEV", "jsxsDEV", "Fragment"];

    if !JSX_RUNTIME_EXPORTS.contains(&export_name) {
        return false;
    }

    // Check if file is likely a JSX runtime
    // Patterns: jsx-runtime, jsx_runtime, jsx-dev-runtime (React dev mode)
    let lower_path = file_path.to_ascii_lowercase();
    lower_path.contains("jsx-runtime")
        || lower_path.contains("jsx_runtime")
        || lower_path.contains("jsx-dev-runtime")
}

/// Check if an export is likely a Flow type-only export.
/// Flow files (annotated with @flow) export types that are used via Flow's type system,
/// not via regular import statements. These exports don't appear in static import analysis
/// but are consumed by Flow type checker.
fn is_flow_type_export(export_symbol: &ExportSymbol, analysis: &FileAnalysis) -> bool {
    if !analysis.is_flow_file {
        return false;
    }

    // Flow type exports: type, interface, opaque type
    // These are type-only and won't appear in runtime imports
    matches!(export_symbol.kind.as_str(), "type" | "interface" | "opaque")
}

/// Check if an export is used in a WeakMap/WeakSet registry pattern.
/// These are common in React and other libraries for storing metadata about objects
/// without causing memory leaks. Exports stored in WeakMap/WeakSet are used dynamically.
/// Pattern: `const registry = new WeakMap(); registry.set(key, ExportedClass)`
fn is_weakmap_registry_export(_export_symbol: &ExportSymbol, analysis: &FileAnalysis) -> bool {
    // If a file contains WeakMap/WeakSet usage (detected by AST visitor),
    // conservatively assume all exports might be stored dynamically in the registry.
    // This reduces false positives in React DevTools and similar code where exports
    // are stored in WeakMaps for dynamic lookup.
    analysis.has_weak_collections
}

fn is_rust_const_table(analysis: &FileAnalysis) -> bool {
    if analysis.language != "rs" {
        return false;
    }
    let const_exports: Vec<_> = analysis
        .exports
        .iter()
        .filter(|e| e.kind == "const")
        .collect();
    if const_exports.len() < 8 {
        return false;
    }

    let shouting: usize = const_exports
        .iter()
        .filter(|e| {
            let name = e.name.as_str();
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        })
        .count();

    // Heuristic: mostly uppercase consts, very few non-const exports => treat as data table.
    let non_const_exports = analysis.exports.len().saturating_sub(const_exports.len());
    shouting * 4 >= const_exports.len() * 3 && non_const_exports <= 2
}
/// Detect if Python file is part of a library (has setup.py/pyproject.toml in tree)
fn is_python_library(root: &std::path::Path) -> bool {
    root.join("setup.py").exists()
        || root.join("pyproject.toml").exists()
        || root.join("setup.cfg").exists()
        // CPython stdlib pattern: Lib/ directory at root
        || root.join("Lib").is_dir()
}

/// Check if export is in __all__ list (public API in Python libraries)
fn is_in_python_all(analysis: &FileAnalysis, export_name: &str) -> bool {
    analysis
        .exports
        .iter()
        .any(|e| e.name == export_name && e.kind == "__all__")
}

/// Check if a Python export is part of the stdlib public API
/// Returns true for exports that are in __all__ lists in CPython's Lib/ directory
fn is_python_stdlib_export(analysis: &FileAnalysis, export_name: &str) -> bool {
    // Check if file is in CPython stdlib structure (Lib/ directory)
    if !analysis.path.contains("/Lib/") && !analysis.path.starts_with("Lib/") {
        return false;
    }

    // All exports in __all__ of stdlib modules are public API
    // This includes constants like calendar.APRIL, classes like csv.DictWriter, etc.
    if is_in_python_all(analysis, export_name) {
        return true;
    }

    // Additional stdlib patterns: top-level public symbols (not starting with _)
    // in stdlib modules that don't have explicit __all__
    if !export_name.starts_with('_') {
        // Constants (UPPER_CASE) in stdlib are typically public API
        if export_name
            .chars()
            .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
        {
            return true;
        }

        // Classes and functions in stdlib without __all__ are typically public
        // Only if the file doesn't have any __all__ (if __all__ exists, it's definitive)
        let has_explicit_all = analysis.exports.iter().any(|e| e.kind == "__all__");
        if !has_explicit_all {
            return true;
        }
    }

    false
}

/// Check if export is a Python dunder method (protocol methods, never dead)
fn is_python_dunder_method(export_name: &str) -> bool {
    export_name.starts_with("__") && export_name.ends_with("__")
}

/// Find potentially dead (unused) exports in the codebase
pub fn find_dead_exports(
    analyses: &[FileAnalysis],
    high_confidence: bool,
    open_base: Option<&str>,
    config: DeadFilterConfig,
) -> Vec<DeadExport> {
    let example_globset = if config.library_mode && !config.example_globs.is_empty() {
        let mut builder = globset::GlobSetBuilder::new();
        for pat in &config.example_globs {
            match globset::Glob::new(pat) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(e) => {
                    eprintln!(
                        "[loctree][warn] invalid library_example_glob '{}': {}",
                        pat, e
                    );
                }
            }
        }
        builder.build().ok()
    } else {
        None
    };

    // Detect Python library mode if enabled
    let is_py_library = config.python_library_mode
        && analyses.iter().any(|a| {
            a.path.ends_with(".py")
                && std::path::Path::new(&a.path)
                    .ancestors()
                    .any(is_python_library)
        });

    // Skip Go for now to avoid false positives until package-level usage is implemented
    let analyses: Vec<&FileAnalysis> = analyses
        .iter()
        .filter(|a| !a.path.ends_with(".go"))
        .collect();

    // Build usage set: (resolved_path, symbol_name)
    let mut used_exports: HashSet<(String, String)> = HashSet::new();
    // Track all imported symbol names as fallback (handles $lib/, @scope/, monorepo paths)
    let mut all_imported_symbols: HashSet<String> = HashSet::new();
    // Track crate-internal imports for Rust: (raw_path, symbol_name)
    let mut crate_internal_imports: Vec<(String, String)> = Vec::new();

    for analysis in &analyses {
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
                    all_imported_symbols.insert(used_name.clone());
                }

                // Track crate-internal imports (crate::, super::, self::)
                if imp.is_crate_relative || imp.is_super_relative || imp.is_self_relative {
                    crate_internal_imports.push((imp.raw_path.clone(), used_name));
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

    // CRITICAL FIX FOR SVELTE .d.ts RE-EXPORTS (60% of FPs):
    // TypeScript declaration files (.d.ts) re-export from implementation files (.js/.ts)
    // Pattern: foo.d.ts has `export { bar } from './foo.js'`
    // The exports in foo.js are NOT dead - they're the implementation for the .d.ts types
    // This fixes false positives like Svelte's easing functions being marked as dead
    let dts_reexports: Vec<_> = analyses
        .iter()
        .filter(|a| {
            a.path.ends_with(".d.ts") || a.path.ends_with(".d.mts") || a.path.ends_with(".d.cts")
        })
        .flat_map(|a| &a.reexports)
        .collect();

    for re in dts_reexports {
        // Mark the re-exported symbols from the source file as used
        let target_norm = re
            .resolved
            .as_ref()
            .map(|t| normalize_module_id(t).as_key())
            .unwrap_or_else(|| normalize_module_id(&re.source).as_key());

        match &re.kind {
            ReexportKind::Star => {
                // Star re-export: mark all exports from target as used
                used_exports.insert((target_norm, "*".to_string()));
            }
            ReexportKind::Named(names) => {
                // Named re-export: mark specific symbols as used
                for name in names {
                    used_exports.insert((target_norm.clone(), name.clone()));
                }
            }
        }
    }

    // Build set of all Tauri registered command handlers (used via generate_handler![])
    let tauri_handlers: HashSet<String> = analyses
        .iter()
        .flat_map(|a| a.tauri_registered_handlers.iter().cloned())
        .collect();

    // Go: gather identifiers used anywhere within the same directory (package-level)
    let mut go_local_uses_by_dir: HashMap<String, HashSet<String>> = HashMap::new();
    for analysis in analyses.iter().filter(|a| a.path.ends_with(".go")) {
        if let Some(dir) = std::path::Path::new(&analysis.path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
        {
            go_local_uses_by_dir
                .entry(dir)
                .or_default()
                .extend(analysis.local_uses.iter().cloned());
        }
    }

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
        for analysis in &analyses {
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
        for analysis in &analyses {
            for dyn_imp in &analysis.dynamic_imports {
                let dyn_norm = normalize_module_id(dyn_imp);
                let dyn_key = dyn_norm.as_key();
                let dyn_alias = strip_alias_prefix(&dyn_norm.path).to_string();
                // Find matching file in analyses
                for a in &analyses {
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
        if should_skip_dead_export_check(analysis, &config, example_globset.as_ref()) {
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

        if is_rust_const_table(analysis) {
            continue;
        }

        let path_norm = normalize_module_id(&analysis.path).as_key();
        let is_go_file = analysis.path.ends_with(".go");

        // Skip noisy generated Go bindings (protobuf/grpc)
        if is_go_file
            && (analysis.path.ends_with(".pb.go")
                || analysis.path.ends_with(".pb.gw.go")
                || analysis.path.contains(".pb.")
                || analysis.path.contains(".pbjson"))
        {
            continue;
        }

        // Temporarily skip Go dead detection to avoid high FP until full package-level usage is implemented
        if is_go_file {
            continue;
        }

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

            // Python library mode: skip exports in __all__ (public API)
            // Also check CPython stdlib pattern independent of is_py_library flag
            // because CPython stdlib has unique Lib/ structure that should be recognized
            let is_stdlib = is_python_file && is_python_stdlib_export(analysis, &exp.name);
            if (is_py_library || is_stdlib) && is_python_file {
                // Skip exports in __all__ (definitive public API marker)
                if is_in_python_all(analysis, &exp.name) {
                    continue;
                }
                // Skip CPython stdlib public API (in Lib/ directory)
                if is_stdlib {
                    continue;
                }
                // Skip dunder methods (__init__, __str__, etc. - runtime protocol)
                if is_python_dunder_method(&exp.name) {
                    continue;
                }
            }

            // Django/Wagtail mixin pattern heuristic:
            // Classes ending in "Mixin" are typically used via multiple inheritance
            // and their methods are called via MRO (Method Resolution Order), not directly imported
            // Common patterns: LoginRequiredMixin, PermissionRequiredMixin, ButtonsColumnMixin, etc.
            let is_django_mixin =
                is_python_file && exp.kind == "class" && exp.name.ends_with("Mixin");
            if is_django_mixin {
                // Skip mixin classes from dead export detection
                // They're used via inheritance which may not be fully tracked in complex codebases
                continue;
            }
            if is_python_test_export(analysis, exp) || is_python_test_path(&analysis.path) {
                continue;
            }

            if exp.name == "default"
                && (analysis.path.ends_with("page.tsx") || analysis.path.ends_with("layout.tsx"))
            {
                // Next.js / framework roots - ignore default export
                continue;
            }

            // JS/TS runtime/framework exports that are inherently used via tooling/framework
            let is_ts_file = analysis.path.ends_with(".ts")
                || analysis.path.ends_with(".tsx")
                || analysis.path.ends_with(".js")
                || analysis.path.ends_with(".jsx")
                || analysis.path.ends_with(".mjs")
                || analysis.path.ends_with(".cjs");
            let ts_runtime_symbol = is_ts_file
                && (matches!(
                    exp.name.as_str(),
                    "jsx" | "jsxs" | "jsxDEV" | "Fragment" | "VoidComponent" | "Component"
                ) || analysis.path.contains("jsx-runtime"));
            let ts_framework_magic = is_ts_file
                && (matches!(
                    exp.name.as_str(),
                    "start" | "resolveRoute" | "enhance" | "load" | "PageLoad" | "LayoutLoad"
                ) || analysis.path.contains("sveltekit")
                    || analysis.path.contains("app/navigation"));
            if ts_runtime_symbol || ts_framework_magic {
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
            let go_pkg_used = if analysis.path.ends_with(".go") {
                std::path::Path::new(&analysis.path)
                    .parent()
                    .and_then(|p| go_local_uses_by_dir.get(&p.to_string_lossy().to_string()))
                    .is_some_and(|set| set.contains(&exp.name))
            } else {
                false
            };
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

            // Check if this export is imported via crate-internal paths (crate::, super::, self::)
            // Use fuzzy matching since nested brace imports may have symbol names with extra chars
            let crate_import_count = crate_internal_imports
                .iter()
                .filter(|(raw_path, symbol)| {
                    // Exact match or symbol contains the export name (handles "MENU_GAP}" matching "MENU_GAP")
                    let symbol_matches = symbol == &exp.name
                        || symbol.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
                            == exp.name
                        || symbol.trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_')
                            == exp.name;
                    symbol_matches && crate_import_matches_file(raw_path, &analysis.path, &exp.name)
                })
                .count();
            let is_crate_imported = crate_import_count > 0;

            // Check if this is a JSX runtime export (jsx, jsxs, Fragment, etc.)
            // These are consumed by TypeScript/Babel compiler, not by regular imports
            let is_jsx_runtime = is_jsx_runtime_export(&exp.name, &analysis.path);

            // Check if this is a Flow type-only export
            // Flow type exports are consumed by Flow type checker, not by runtime imports
            let is_flow_type = is_flow_type_export(exp, analysis);

            // Check if this export is used in a WeakMap/WeakSet registry pattern
            // These are dynamically accessed and won't show up in static imports
            let is_weak_registry = is_weakmap_registry_export(exp, analysis);

            if !is_used
                && !star_used
                && !locally_used
                && !go_pkg_used
                && !is_tauri_handler
                && !imported_by_name
                && !is_svelte_api
                && !is_rust_path_qualified
                && !is_crate_imported
                && !is_jsx_runtime
                && !is_flow_type
                && !is_weak_registry
            {
                let open_url = super::build_open_url(&analysis.path, exp.line, open_base);

                // Build human-readable reason
                let reason = if is_rust_file {
                    format!(
                        "No imports found for '{}'. Checked: direct imports (0 matches), \
                         star imports (none), crate imports (0 matches), local uses (none), \
                         Tauri handlers (not registered)",
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
    fn test_find_dead_exports_respects_crate_imports() {
        let exporter = mock_file_with_exports("src/ui/constants.rs", vec!["MENU_GAP"]);
        let mut importer = mock_file("src/main.rs");
        let mut imp = ImportEntry::new(
            "crate::ui::constants::MENU_GAP".to_string(),
            ImportKind::Static,
        );
        imp.raw_path = "crate::ui::constants::MENU_GAP".to_string();
        imp.is_crate_relative = true;
        imp.symbols.push(ImportSymbol {
            name: "MENU_GAP".to_string(),
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
            "crate-internal imports should prevent dead export marking. Found: {:?}",
            result
        );
    }

    #[test]
    fn test_find_dead_exports_respects_super_imports() {
        let exporter = mock_file_with_exports("src/types.rs", vec!["Config"]);
        let mut importer = mock_file("src/ui/widget.rs");
        let mut imp = ImportEntry::new("super::types::Config".to_string(), ImportKind::Static);
        imp.raw_path = "super::types::Config".to_string();
        imp.is_super_relative = true;
        imp.symbols.push(ImportSymbol {
            name: "Config".to_string(),
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
            "super:: imports should prevent dead export marking. Found: {:?}",
            result
        );
    }

    #[test]
    fn test_crate_import_matches_file_basic() {
        // Test basic crate:: import matching
        assert!(
            crate_import_matches_file(
                "crate::ui::constants::MENU_GAP",
                "src/ui/constants.rs",
                "MENU_GAP"
            ),
            "should match crate::ui::constants with src/ui/constants.rs"
        );

        assert!(
            crate_import_matches_file("crate::types::Config", "src/types.rs", "Config"),
            "should match crate::types with src/types.rs"
        );

        assert!(
            crate_import_matches_file("super::utils::helper", "utils.rs", "helper"),
            "should match super::utils with utils.rs"
        );

        // Test non-matches
        assert!(
            !crate_import_matches_file("crate::ui::constants::X", "src/ui/layout.rs", "X"),
            "should NOT match constants with layout.rs"
        );

        assert!(
            !crate_import_matches_file("external::package::Foo", "src/foo.rs", "Foo"),
            "should NOT match non-crate imports"
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
                library_mode: false,
                example_globs: Vec::new(),
                python_library_mode: false,
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
    fn test_find_dead_exports_skips_jsx_runtime_files() {
        // JSX runtime files should be completely skipped from dead export detection
        let mut jsx_runtime = mock_file_with_exports(
            "packages/solid-js/jsx-runtime/index.ts",
            vec!["jsx", "jsxs", "jsxDEV", "Fragment"],
        );
        jsx_runtime.language = "ts".to_string();

        let result = find_dead_exports(&[jsx_runtime], false, None, DeadFilterConfig::default());
        assert!(
            result.is_empty(),
            "JSX runtime files should be completely skipped: {:?}",
            result
        );
    }

    #[test]
    fn test_find_dead_exports_skips_jsx_runtime_exports() {
        // Individual JSX runtime exports (jsx, jsxs, Fragment) in jsx-runtime paths should not be flagged
        let mut runtime_file = mock_file_with_exports(
            "node_modules/solid-js/jsx-runtime.js",
            vec!["jsx", "jsxs", "jsxDEV", "Fragment", "createComponent"],
        );
        runtime_file.language = "js".to_string();

        let result = find_dead_exports(&[runtime_file], false, None, DeadFilterConfig::default());
        // File should be skipped entirely due to jsx-runtime path pattern
        assert!(
            result.is_empty(),
            "JSX runtime exports should not be flagged as dead: {:?}",
            result
        );
    }

    #[test]
    fn test_jsx_runtime_export_detection() {
        // Test the helper function directly
        assert!(is_jsx_runtime_export(
            "jsx",
            "packages/solid-js/jsx-runtime/index.ts"
        ));
        assert!(is_jsx_runtime_export("jsxs", "vue/jsx-runtime.js"));
        assert!(is_jsx_runtime_export("jsxDEV", "react/jsx-dev-runtime.js"));
        assert!(is_jsx_runtime_export(
            "Fragment",
            "preact/jsx-runtime/index.mjs"
        ));
        assert!(is_jsx_runtime_export("jsxsDEV", "solid/jsx_runtime.ts"));

        // Non JSX runtime exports should not match
        assert!(!is_jsx_runtime_export("Component", "jsx-runtime/index.ts"));
        assert!(!is_jsx_runtime_export("jsx", "src/utils/helpers.ts"));
        assert!(!is_jsx_runtime_export("createElement", "jsx-runtime.js"));
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
    fn test_django_wagtail_mixin_not_dead() {
        // Test that Django/Wagtail mixins used in inheritance are not marked as dead
        // This tests the integration between py.rs (which tracks inheritance) and dead_parrots.rs

        use crate::types::{ExportSymbol, FileAnalysis};

        // Mixin definition file
        let mixin_file = FileAnalysis {
            path: "myapp/mixins.py".to_string(),
            language: "py".to_string(),
            exports: vec![
                ExportSymbol::new("LoginRequiredMixin".to_string(), "class", "named", Some(1)),
                ExportSymbol::new(
                    "PermissionRequiredMixin".to_string(),
                    "class",
                    "named",
                    Some(5),
                ),
                ExportSymbol::new("ButtonsColumnMixin".to_string(), "class", "named", Some(10)),
            ],
            ..Default::default()
        };

        // View file that uses the mixins
        let mut view_file = FileAnalysis {
            path: "myapp/views.py".to_string(),
            language: "py".to_string(),
            exports: vec![], // No exports to avoid noise
            // Simulate what py.rs does: add base classes to local_uses
            local_uses: vec![
                "LoginRequiredMixin".to_string(),
                "PermissionRequiredMixin".to_string(),
                "ButtonsColumnMixin".to_string(),
            ],
            ..Default::default()
        };

        // Add import entry to track the relationship
        use crate::types::{ImportEntry, ImportKind, ImportSymbol};
        let mut imp = ImportEntry::new("myapp.mixins".to_string(), ImportKind::Static);
        imp.resolved_path = Some("myapp/mixins.py".to_string());
        imp.symbols = vec![
            ImportSymbol {
                name: "LoginRequiredMixin".to_string(),
                alias: None,
                is_default: false,
            },
            ImportSymbol {
                name: "PermissionRequiredMixin".to_string(),
                alias: None,
                is_default: false,
            },
            ImportSymbol {
                name: "ButtonsColumnMixin".to_string(),
                alias: None,
                is_default: false,
            },
        ];
        view_file.imports.push(imp);

        let analyses = vec![mixin_file, view_file];
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());

        // All mixins should be marked as used (both via imports AND local_uses from inheritance tracking)
        assert!(
            result.is_empty(),
            "Django/Wagtail mixins should not be marked as dead. Found dead: {:?}",
            result
        );
    }

    #[test]
    fn test_django_mixin_pattern_common_names() {
        // Test common Django/Wagtail mixin naming patterns
        // These should not be flagged as dead even if static analysis misses some usage
        use crate::types::{ExportSymbol, FileAnalysis};

        let mixin_file = FileAnalysis {
            path: "django/contrib/auth/mixins.py".to_string(),
            language: "py".to_string(),
            exports: vec![
                // Standard Django mixins that are ALWAYS used via MRO, never called directly
                ExportSymbol::new("LoginRequiredMixin".to_string(), "class", "named", Some(1)),
                ExportSymbol::new(
                    "PermissionRequiredMixin".to_string(),
                    "class",
                    "named",
                    Some(10),
                ),
                ExportSymbol::new(
                    "UserPassesTestMixin".to_string(),
                    "class",
                    "named",
                    Some(20),
                ),
                // Non-mixin class (should be flagged if unused)
                ExportSymbol::new("AuthHelper".to_string(), "class", "named", Some(30)),
            ],
            ..Default::default()
        };

        // No imports - testing heuristic fallback for common Django patterns
        let analyses = vec![mixin_file];
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());

        // Mixins ending in "Mixin" should NOT be flagged (heuristic protection)
        let mixin_names: Vec<_> = result
            .iter()
            .filter(|d| d.symbol.ends_with("Mixin"))
            .collect();
        assert!(
            mixin_names.is_empty(),
            "Classes ending in 'Mixin' should not be flagged as dead (Django/Wagtail pattern). Found: {:?}",
            mixin_names
        );

        // Non-mixin classes (like AuthHelper) SHOULD be flagged if truly unused
        let has_non_mixin = result.iter().any(|d| d.symbol == "AuthHelper");
        assert!(
            has_non_mixin,
            "Non-mixin classes like 'AuthHelper' should still be flagged when unused"
        );
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

    #[test]
    fn test_python_stdlib_exports_not_dead() {
        // Test that CPython stdlib exports in __all__ are not marked as dead
        // This addresses the 100% FP rate on python/cpython smoke test

        // Simulate calendar.py module with APRIL constant in __all__
        let calendar_module = FileAnalysis {
            path: "Lib/calendar.py".to_string(),
            language: "py".to_string(),
            exports: vec![
                ExportSymbol::new("APRIL".to_string(), "__all__", "named", Some(1)),
                ExportSymbol::new("APRIL".to_string(), "const", "named", Some(10)),
            ],
            ..Default::default()
        };

        // Simulate csv.py module with DictWriter in __all__
        let csv_module = FileAnalysis {
            path: "Lib/csv.py".to_string(),
            language: "py".to_string(),
            exports: vec![
                ExportSymbol::new("DictWriter".to_string(), "__all__", "named", Some(1)),
                ExportSymbol::new("DictWriter".to_string(), "class", "named", Some(50)),
            ],
            ..Default::default()
        };

        // Simulate typing.py module with override in __all__
        let typing_module = FileAnalysis {
            path: "Lib/typing.py".to_string(),
            language: "py".to_string(),
            exports: vec![
                ExportSymbol::new("override".to_string(), "__all__", "named", Some(1)),
                ExportSymbol::new("override".to_string(), "function", "named", Some(200)),
            ],
            ..Default::default()
        };

        let analyses = vec![calendar_module, csv_module, typing_module];

        // Run dead export detection with python_library_mode enabled
        let dead_exports = find_dead_exports(
            &analyses,
            false,
            None,
            DeadFilterConfig {
                include_tests: false,
                include_helpers: false,
                library_mode: false,
                example_globs: Vec::new(),
                python_library_mode: true, // Enable Python library mode
            },
        );

        // Verify that NONE of these stdlib exports are marked as dead
        // They're all in __all__ lists and are public API for millions of Python programs
        assert!(
            dead_exports.is_empty(),
            "CPython stdlib exports in __all__ should NOT be marked as dead. Found: {:?}",
            dead_exports
        );
    }

    #[test]
    fn test_python_stdlib_uppercase_constants_not_dead() {
        // Test that UPPER_CASE constants in stdlib are treated as public API
        // even if not in __all__ (some stdlib modules don't have explicit __all__)

        let module = FileAnalysis {
            path: "Lib/socket.py".to_string(),
            language: "py".to_string(),
            exports: vec![
                ExportSymbol::new("AF_INET".to_string(), "const", "named", Some(10)),
                ExportSymbol::new("SOCK_STREAM".to_string(), "const", "named", Some(20)),
            ],
            ..Default::default()
        };

        let analyses = vec![module];

        let dead_exports = find_dead_exports(
            &analyses,
            false,
            None,
            DeadFilterConfig {
                include_tests: false,
                include_helpers: false,
                library_mode: false,
                example_globs: Vec::new(),
                python_library_mode: true,
            },
        );

        // UPPER_CASE constants in stdlib should not be marked as dead
        assert!(
            dead_exports.is_empty(),
            "CPython stdlib UPPER_CASE constants should NOT be dead. Found: {:?}",
            dead_exports
        );
    }

    #[test]
    fn test_python_non_stdlib_requires_all() {
        // Test that non-stdlib Python files still require proper __all__ or usage
        // to avoid being marked as dead

        let user_module = FileAnalysis {
            path: "myapp/utils.py".to_string(), // NOT in Lib/
            language: "py".to_string(),
            exports: vec![ExportSymbol::new(
                "helper".to_string(),
                "function",
                "named",
                Some(10),
            )],
            ..Default::default()
        };

        let analyses = vec![user_module];

        let dead_exports = find_dead_exports(
            &analyses,
            false,
            None,
            DeadFilterConfig {
                include_tests: false,
                include_helpers: false,
                library_mode: false,
                example_globs: Vec::new(),
                python_library_mode: true,
            },
        );

        // User code without __all__ or usage SHOULD be marked as dead
        assert_eq!(
            dead_exports.len(),
            1,
            "Non-stdlib exports without __all__ should be marked as dead"
        );
        assert_eq!(dead_exports[0].symbol, "helper");
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
    if !analysis.path.ends_with(".py") {
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

fn is_python_test_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_lowercase();
    lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.ends_with("_test.py")
        || lower.ends_with("_tests.py")
        || lower
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
}

/// Check if a crate-internal import (e.g., `use crate::foo::MENU_GAP`) might reference
/// an export from the given file path.
///
/// This is a heuristic approach that handles ~80% of cases without full module resolution:
/// - Extract the module path from the import (e.g., `foo` from `crate::foo::X`)
/// - Check if the export file's name/path matches that module segment
///
/// Examples:
/// - `use crate::ui::constants::MENU_GAP` matches `src/ui/constants.rs`
/// - `use super::types::Config` matches `types.rs` in parent dir
/// - `use self::utils::helper` matches `utils.rs` in same dir
fn crate_import_matches_file(
    import_raw_path: &str,
    export_file_path: &str,
    symbol_name: &str,
) -> bool {
    // Only handle Rust crate-internal imports
    if !import_raw_path.starts_with("crate::")
        && !import_raw_path.starts_with("super::")
        && !import_raw_path.starts_with("self::")
    {
        return false;
    }

    // Normalize the export file path for matching
    let export_normalized = export_file_path.replace('\\', "/");

    // Extract module path segments from import
    // e.g., "crate::ui::constants::MENU_GAP" -> ["ui", "constants"]
    let import_segments: Vec<&str> = import_raw_path
        .split("::")
        .filter(|s| *s != "crate" && *s != "super" && *s != "self" && *s != symbol_name)
        .collect();

    if import_segments.is_empty() {
        return false;
    }

    // Build potential module path patterns
    // For "crate::ui::constants::X", we check if file ends with:
    // - "ui/constants.rs"
    // - "ui/constants/mod.rs"
    // - just "constants.rs" (simple heuristic)

    let module_path = import_segments.join("/");

    // Check various patterns:
    // 1. Full path match: "src/ui/constants.rs"
    if export_normalized.contains(&format!("{}.rs", module_path))
        || export_normalized.contains(&format!("{}/mod.rs", module_path))
        || export_normalized.contains(&format!("{}/lib.rs", module_path))
    {
        return true;
    }

    // 2. Last segment match (simple heuristic): "constants.rs"
    if let Some(last_segment) = import_segments.last() {
        let file_stem = export_normalized
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".rs");

        if file_stem == *last_segment {
            return true;
        }
    }

    // 3. super:: relative match - check if export is in parent directory
    if import_raw_path.starts_with("super::") && !import_segments.is_empty() {
        // For super::types::Config, check if file name is "types.rs"
        if let Some(first_segment) = import_segments.first() {
            let file_name = export_normalized.rsplit('/').next().unwrap_or("");
            if file_name == format!("{}.rs", first_segment) {
                return true;
            }
        }
    }

    // 4. Fallback heuristic for complex nested imports like:
    //    crate::{..., code_context_menus::{..., MENU_GAP}, ...}
    // Check if BOTH the symbol name AND the file's module name appear in raw_path
    let file_stem = export_normalized
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim_end_matches(".rs")
        .trim_end_matches("/mod");

    // Symbol must appear as a word boundary (not part of another identifier)
    let symbol_pattern = format!(r"\b{}\b", regex::escape(symbol_name));
    let module_pattern = format!(r"\b{}\b", regex::escape(file_stem));

    if let (Ok(sym_re), Ok(mod_re)) = (
        regex::Regex::new(&symbol_pattern),
        regex::Regex::new(&module_pattern),
    ) && sym_re.is_match(import_raw_path)
        && mod_re.is_match(import_raw_path)
    {
        return true;
    }

    false
}
