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

use serde::Serialize;

use crate::types::{FileAnalysis, ReexportKind};

use super::root_scan::normalize_module_id;

/// Re-export info: (reexporter_file, original_name, exported_alias)
type ReexportInfoEntry = (String, String, String);
/// Map from (file_norm, symbol) to list of re-export entries
type ReexportInfoMap = HashMap<(String, String), Vec<ReexportInfoEntry>>;

/// Shadow export: export that exists but is never imported because another file exports the same symbol
#[derive(Debug, Clone, Serialize)]
pub struct ShadowExport {
    /// Symbol name that is shadowed
    pub symbol: String,
    /// File that exports the symbol but is USED (imported through barrel/re-export)
    pub used_file: String,
    /// Line number in used file
    pub used_line: Option<usize>,
    /// Files that export the same symbol but are DEAD (never imported)
    pub dead_files: Vec<ShadowExportFile>,
    /// Total LOC across all dead files
    pub total_dead_loc: usize,
}

/// Individual dead file in a shadow export scenario
#[derive(Debug, Clone, Serialize)]
pub struct ShadowExportFile {
    /// File path
    pub file: String,
    /// Line number where symbol is exported
    pub line: Option<usize>,
    /// Lines of code in this file
    pub loc: usize,
}

// Submodules
mod filters;
mod languages;
pub mod output;
pub mod search;

// Re-export public types and functions
pub use output::{
    print_dead_exports, print_impact_results, print_shadow_exports, print_similarity_results,
    print_symbol_results,
};
pub use search::{
    ImpactResult, SimilarityCandidate, SymbolFileMatch, SymbolMatch, SymbolSearchResult,
    analyze_impact, find_similar, search_symbol,
};

// Internal imports
use filters::{
    is_flow_type_export, is_jsx_runtime_export, is_python_test_export, is_python_test_path,
    is_weakmap_registry_export, should_skip_dead_export_check,
};
use languages::{
    crate_import_matches_file, is_in_python_all, is_python_dunder_method, is_python_library,
    is_python_stdlib_export, is_rust_const_table, is_svelte_component_api, rust_has_known_derives,
};

fn strip_alias_prefix(path: &str) -> &str {
    // Drop leading alias markers like @core/... -> core/...
    let without_at = path.trim_start_matches('@');
    if let Some(idx) = without_at.find('/') {
        &without_at[idx + 1..]
    } else {
        without_at
    }
}
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
    /// Whether this is a test file
    #[serde(default)]
    pub is_test: bool,
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

    // === INFORMATIVE OUTPUT: Build detailed lookup maps for reason messages ===
    // Import counts: how many times each (file, symbol) is imported
    let mut import_counts: HashMap<(String, String), usize> = HashMap::new();
    // Re-export info: (file_norm, symbol) -> Vec<(reexporter_file, original_name, exported_alias)>
    let mut reexport_info: ReexportInfoMap = HashMap::new();
    // Dynamic import sources: file_norm -> Vec<importer_file>
    let mut dynamic_import_sources: HashMap<String, Vec<String>> = HashMap::new();

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
                    crate_internal_imports.push((imp.raw_path.clone(), used_name.clone()));
                }

                // INFORMATIVE: Count imports per (file, symbol)
                *import_counts
                    .entry((target_norm.clone(), used_name))
                    .or_insert(0) += 1;
            }
        }
        // Track dynamic imports for informative output
        for dyn_imp in &analysis.dynamic_imports {
            let dyn_norm = normalize_module_id(dyn_imp).as_key();
            dynamic_import_sources
                .entry(dyn_norm)
                .or_default()
                .push(analysis.path.clone());
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
                    for (original, exported) in names {
                        // Mark original name as used in target module
                        used_exports.insert((target_norm.clone(), original.clone()));
                        // INFORMATIVE: Track re-export info with alias
                        reexport_info
                            .entry((target_norm.clone(), original.clone()))
                            .or_default()
                            .push((analysis.path.clone(), original.clone(), exported.clone()));
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
                for (original, _exported) in names {
                    used_exports.insert((target_norm.clone(), original.clone()));
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

                // Calculate actual counts for informative output
                let import_count = import_counts
                    .get(&(path_norm.clone(), exp.name.clone()))
                    .copied()
                    .unwrap_or(0);
                let reexport_entries = reexport_info
                    .get(&(path_norm.clone(), exp.name.clone()))
                    .cloned()
                    .unwrap_or_default();
                let reexport_count = reexport_entries.len();
                let dynamic_count = dynamic_import_sources
                    .get(&path_norm)
                    .map(|v| v.len())
                    .unwrap_or(0);

                // Build human-readable reason with context for user decision
                let reason = if is_rust_file {
                    format!(
                        "Exported symbol '{}' has no detected usages. \
                         Checked: use statements ({}), path-qualified calls (0), \
                         crate:: imports ({}), Tauri invoke_handler (not found). \
                         Consider: If this is a public API consumed externally, it's expected. \
                         If internal-only, consider removing or making private.",
                        exp.name, import_count, crate_import_count
                    )
                } else {
                    // Build detailed re-export info for informative output
                    let reexport_details = if !reexport_entries.is_empty() {
                        let details: Vec<String> = reexport_entries
                            .iter()
                            .take(3) // Limit to 3 for readability
                            .map(|(file, original, alias)| {
                                if original != alias {
                                    format!("as '{}' in {}", alias, file)
                                } else {
                                    file.clone()
                                }
                            })
                            .collect();
                        let more = if reexport_entries.len() > 3 {
                            format!(" (+{} more)", reexport_entries.len() - 3)
                        } else {
                            String::new()
                        };
                        format!(" ({}{})", details.join(", "), more)
                    } else {
                        String::new()
                    };

                    // Build dynamic import details
                    let dynamic_details = if dynamic_count > 0 {
                        let sources: Vec<String> = dynamic_import_sources
                            .get(&path_norm)
                            .map(|v| v.iter().take(2).cloned().collect())
                            .unwrap_or_default();
                        let more = if dynamic_count > 2 {
                            format!(" +{} more", dynamic_count - 2)
                        } else {
                            String::new()
                        };
                        format!(" (by {}{})", sources.join(", "), more)
                    } else {
                        String::new()
                    };

                    format!(
                        "Exported symbol '{}' has no detected imports. \
                         Checked: import statements ({}), re-exports ({}){}, \
                         dynamic imports ({}){}, JSX references (0). \
                         Consider: If used via barrel exports or external packages, verify manually. \
                         If truly unused, safe to remove.",
                        exp.name,
                        import_count,
                        reexport_count,
                        reexport_details,
                        dynamic_count,
                        dynamic_details
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
                    is_test: analysis.is_test,
                });
            }
        }
    }

    dead_candidates
}

/// Detect shadow exports: same symbol exported by multiple files, but only one is actually used.
///
/// This identifies "zombie" files that export symbols which are masked by barrel re-exports
/// from other files. For example:
/// - `stores/conversationHostStore.ts` exports `conversationHostStore` (361 LOC) - DEAD
/// - `aiStore/slices/conversationHostSlice.ts` exports `conversationHostStore` - USED
/// - Barrel `@ai-suite/state` re-exports from the NEW file, old file is zombie
pub fn find_shadow_exports(analyses: &[FileAnalysis]) -> Vec<ShadowExport> {
    // Build map of symbol_name -> Vec<(file_path, line, export)>
    let mut symbol_map: HashMap<String, Vec<(String, Option<usize>, String)>> = HashMap::new();

    for analysis in analyses {
        for exp in &analysis.exports {
            // Skip re-export bindings (we only care about original exports)
            if exp.kind == "reexport" {
                continue;
            }

            symbol_map.entry(exp.name.clone()).or_default().push((
                analysis.path.clone(),
                exp.line,
                exp.kind.clone(),
            ));
        }
    }

    // Build set of (file, symbol) that are actually imported
    let mut used_exports: HashSet<(String, String)> = HashSet::new();

    for analysis in analyses {
        for imp in &analysis.imports {
            let target_norm = if let Some(target) = &imp.resolved_path {
                normalize_module_id(target).as_key()
            } else {
                normalize_module_id(&imp.source).as_key()
            };

            for sym in &imp.symbols {
                let used_name = if sym.is_default {
                    "default".to_string()
                } else {
                    sym.name.clone()
                };
                used_exports.insert((target_norm.clone(), used_name));
            }
        }

        // Also check re-exports
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
                    for (original, _exported) in names {
                        used_exports.insert((target_norm.clone(), original.clone()));
                    }
                }
            }
        }
    }

    let mut shadows = Vec::new();

    // Find symbols exported by multiple files
    for (symbol, exporters) in symbol_map {
        if exporters.len() <= 1 {
            continue; // Not a duplicate, skip
        }

        // Check which files are actually imported
        let mut used_files = Vec::new();
        let mut dead_files = Vec::new();

        for (file, line, _kind) in &exporters {
            let file_norm = normalize_module_id(file).as_key();
            let is_used = used_exports.contains(&(file_norm.clone(), symbol.clone()))
                || used_exports.contains(&(file_norm, "*".to_string()));

            if is_used {
                used_files.push((file.clone(), *line));
            } else {
                // Get LOC for this file
                let loc = analyses
                    .iter()
                    .find(|a| a.path == *file)
                    .map(|a| a.loc)
                    .unwrap_or(0);

                dead_files.push(ShadowExportFile {
                    file: file.clone(),
                    line: *line,
                    loc,
                });
            }
        }

        // Only report if we have both used and dead files
        if !used_files.is_empty() && !dead_files.is_empty() {
            // Use the first used file as the canonical one
            let (used_file, used_line) = used_files.into_iter().next().unwrap();
            let total_dead_loc = dead_files.iter().map(|f| f.loc).sum();

            shadows.push(ShadowExport {
                symbol,
                used_file,
                used_line,
                dead_files,
                total_dead_loc,
            });
        }
    }

    // Sort by total_dead_loc descending (highest impact first)
    shadows.sort_by(|a, b| b.total_dead_loc.cmp(&a.total_dead_loc));

    shadows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputMode;
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
            kind: ReexportKind::Named(vec![("Foo".to_string(), "Foo".to_string())]),
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
            is_test: false,
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
            is_test: false,
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
                is_test: false,
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
    fn test_weakmap_registry_skips_dead_exports() {
        // Test that exports in files with WeakMap/WeakSet are not marked as dead
        // This handles React DevTools and similar code where exports are stored dynamically

        let weakmap_file = FileAnalysis {
            path: "src/devtools.ts".to_string(),
            language: "ts".to_string(),
            has_weak_collections: true, // File contains new WeakMap() or new WeakSet()
            exports: vec![
                ExportSymbol::new(
                    "registerComponent".to_string(),
                    "function",
                    "named",
                    Some(10),
                ),
                ExportSymbol::new(
                    "getComponentData".to_string(),
                    "function",
                    "named",
                    Some(20),
                ),
            ],
            ..Default::default()
        };

        // Simulate that these exports are NOT imported anywhere (would normally be dead)
        // But they should not be flagged because the file has WeakMap/WeakSet

        let analyses = vec![weakmap_file];
        let result = find_dead_exports(&analyses, false, None, DeadFilterConfig::default());

        assert!(
            result.is_empty(),
            "Exports in files with WeakMap/WeakSet should NOT be flagged as dead. Found: {:?}",
            result
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
    fn test_shadow_export_detection() {
        // Test shadow export detection: same symbol exported by multiple files, only one used
        // Pattern: stores/conversationHostStore.ts exports conversationHostStore (DEAD)
        //          aiStore/slices/conversationHostSlice.ts exports conversationHostStore (USED)

        use crate::types::{ImportEntry, ImportKind, ImportSymbol};

        // Old file that exports conversationHostStore (361 LOC) - will be DEAD
        let old_store = FileAnalysis {
            path: "stores/conversationHostStore.ts".to_string(),
            language: "ts".to_string(),
            loc: 361,
            exports: vec![ExportSymbol::new(
                "conversationHostStore".to_string(),
                "const",
                "named",
                Some(42),
            )],
            ..Default::default()
        };

        // New file that exports conversationHostStore - will be USED
        let new_slice = FileAnalysis {
            path: "aiStore/slices/conversationHostSlice.ts".to_string(),
            language: "ts".to_string(),
            loc: 120,
            exports: vec![ExportSymbol::new(
                "conversationHostStore".to_string(),
                "const",
                "named",
                Some(15),
            )],
            ..Default::default()
        };

        // File that imports from the NEW location
        let mut importer = FileAnalysis {
            path: "components/Chat.tsx".to_string(),
            language: "tsx".to_string(),
            ..Default::default()
        };
        let mut imp = ImportEntry::new(
            "aiStore/slices/conversationHostSlice".to_string(),
            ImportKind::Static,
        );
        imp.resolved_path = Some("aiStore/slices/conversationHostSlice.ts".to_string());
        imp.symbols.push(ImportSymbol {
            name: "conversationHostStore".to_string(),
            alias: None,
            is_default: false,
        });
        importer.imports.push(imp);

        let analyses = vec![old_store, new_slice, importer];
        let shadows = find_shadow_exports(&analyses);

        assert_eq!(shadows.len(), 1, "Should find exactly one shadow export");

        let shadow = &shadows[0];
        assert_eq!(shadow.symbol, "conversationHostStore");
        assert_eq!(
            shadow.used_file, "aiStore/slices/conversationHostSlice.ts",
            "New file should be marked as USED"
        );
        assert_eq!(shadow.dead_files.len(), 1, "Should have one dead file");
        assert_eq!(
            shadow.dead_files[0].file, "stores/conversationHostStore.ts",
            "Old file should be marked as DEAD"
        );
        assert_eq!(
            shadow.dead_files[0].loc, 361,
            "Should track LOC of dead file"
        );
        assert_eq!(shadow.total_dead_loc, 361);
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
    use crate::types::{
        ExportSymbol, ImportEntry, ImportKind, ImportSymbol, ReexportEntry, ReexportKind,
    };

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

    #[test]
    fn test_dts_reexport_marks_implementation_as_used() {
        // Test the Svelte .d.ts re-export pattern (60% of FPs)
        // Pattern: easing/index.d.ts re-exports from easing/index.js
        // The exports in index.js should NOT be marked as dead

        // Implementation file (.js)
        let mut implementation = FileAnalysis {
            path: "packages/svelte/src/easing/index.js".to_string(),
            language: "js".to_string(),
            ..Default::default()
        };
        implementation.exports = vec![
            ExportSymbol::new("linear".to_string(), "function", "named", Some(1)),
            ExportSymbol::new("backIn".to_string(), "function", "named", Some(5)),
            ExportSymbol::new("backOut".to_string(), "function", "named", Some(10)),
        ];

        // Declaration file (.d.ts) that re-exports from implementation
        let mut declaration = FileAnalysis {
            path: "packages/svelte/src/easing/index.d.ts".to_string(),
            language: "ts".to_string(),
            ..Default::default()
        };
        declaration.reexports.push(ReexportEntry {
            source: "./index.js".to_string(),
            kind: ReexportKind::Named(vec![
                ("linear".to_string(), "linear".to_string()),
                ("backIn".to_string(), "backIn".to_string()),
                ("backOut".to_string(), "backOut".to_string()),
            ]),
            resolved: Some("packages/svelte/src/easing/index.js".to_string()),
        });

        let result = find_dead_exports(
            &[implementation, declaration],
            false,
            None,
            DeadFilterConfig::default(),
        );

        // All easing functions should be marked as used (re-exported by .d.ts)
        assert!(
            result.is_empty(),
            "Exports re-exported by .d.ts should NOT be marked as dead. Found dead: {:?}",
            result
        );
    }

    #[test]
    fn test_dts_star_reexport_marks_all_as_used() {
        // Test .d.ts star re-export pattern
        // Pattern: index.d.ts has `export * from './impl.js'`

        let mut implementation = FileAnalysis {
            path: "lib/impl.js".to_string(),
            language: "js".to_string(),
            ..Default::default()
        };
        implementation.exports = vec![
            ExportSymbol::new("funcA".to_string(), "function", "named", Some(1)),
            ExportSymbol::new("funcB".to_string(), "function", "named", Some(5)),
            ExportSymbol::new("funcC".to_string(), "function", "named", Some(10)),
        ];

        let mut declaration = FileAnalysis {
            path: "lib/index.d.ts".to_string(),
            language: "ts".to_string(),
            ..Default::default()
        };
        declaration.reexports.push(ReexportEntry {
            source: "./impl.js".to_string(),
            kind: ReexportKind::Star,
            resolved: Some("lib/impl.js".to_string()),
        });

        let result = find_dead_exports(
            &[implementation, declaration],
            false,
            None,
            DeadFilterConfig::default(),
        );

        // All functions should be marked as used (star re-export from .d.ts)
        assert!(
            result.is_empty(),
            "Exports re-exported via star by .d.ts should NOT be marked as dead. Found dead: {:?}",
            result
        );
    }
}
