//! Query API for fast lookups against the cached snapshot.
//!
//! Provides interactive queries without re-scanning:
//! - `who-imports <file>` - Find all files that import a given file
//! - `where-symbol <symbol>` - Find where a symbol is defined
//! - `component-of <file>` - Show what component/module a file belongs to
//!
//! Vibecrafted with AI Agents by VetCoders (c)2026 VetCoders

use serde::{Deserialize, Serialize};

use crate::analyzer::dead_parrots::search_symbol;
use crate::snapshot::Snapshot;

// ============================================================================
// Constants
// ============================================================================

/// Maximum depth for BFS traversal of re-export chains.
/// Prevents infinite loops in pathological cases (circular re-exports).
const MAX_REEXPORT_DEPTH: usize = 50;

/// File extensions we recognize for index file detection
const INDEX_EXTENSIONS: [&str; 3] = ["ts", "tsx", "js"];

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate index file variants for a directory path.
/// `foo/bar` → `["foo/bar/index.ts", "foo/bar/index.tsx", "foo/bar/index.js"]`
fn index_variants(path: &str) -> Vec<String> {
    INDEX_EXTENSIONS
        .iter()
        .map(|ext| format!("{}/index.{}", path, ext))
        .collect()
}

/// Strip index file suffix from a path if present.
/// `foo/bar/index.ts` → `Some("foo/bar")`
/// `foo/bar/utils.ts` → `None`
fn strip_index_suffix(path: &str) -> Option<&str> {
    for ext in INDEX_EXTENSIONS {
        let suffix = format!("/index.{}", ext);
        if let Some(stripped) = path.strip_suffix(&suffix) {
            return Some(stripped);
        }
    }
    None
}

/// Check if a path looks like a file (has known extension)
fn has_file_extension(path: &str) -> bool {
    path.ends_with(".ts")
        || path.ends_with(".tsx")
        || path.ends_with(".js")
        || path.ends_with(".jsx")
        || path.ends_with(".rs")
        || path.ends_with(".py")
}

/// Normalize path for comparison (handles relative vs absolute, trailing slashes)
fn normalize_path(path: &str) -> String {
    path.trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

/// Check if two paths match, considering:
/// - Exact match
/// - Suffix match (edge.to ends with /target)
/// - Folder match (target is index file, edge.to is folder)
///
/// STRICTER than before: avoids `utils.ts` matching `other-utils.ts`
fn paths_match(edge_to: &str, target: &str) -> bool {
    let edge_norm = normalize_path(edge_to);
    let target_norm = normalize_path(target);

    // Exact match
    if edge_norm == target_norm {
        return true;
    }

    // Suffix match: edge.to ends with /target (full path segment)
    if edge_norm.ends_with(&format!("/{}", target_norm)) {
        return true;
    }

    // Folder match: target is index file, edge.to points to folder
    // e.g., target = "foo/index.ts", edge.to = "foo"
    if let Some(folder) = strip_index_suffix(&target_norm)
        && (edge_norm == folder || edge_norm.ends_with(&format!("/{}", folder)))
    {
        return true;
    }

    false
}

/// Result of a query operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Query kind (who-imports, where-symbol, component-of)
    pub kind: String,
    /// Target that was queried (file path or symbol name)
    pub target: String,
    /// Matching results
    pub results: Vec<QueryMatch>,
}

/// A single query match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMatch {
    /// File path
    pub file: String,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Additional context (e.g., import statement, symbol definition)
    pub context: Option<String>,
}

/// Query for files that import a given file or symbol (who-imports)
/// Follows re-export chains transitively to find all importers.
///
/// If the input looks like a symbol name (no path separators), it will first
/// resolve the symbol to file paths where it's defined, then find importers.
///
/// ## Algorithm
/// Uses BFS with depth limiting to traverse re-export chains:
/// `App.tsx → features/index.ts (reexport) → Component.tsx`
///
/// ## Path Matching
/// Uses `paths_match()` for strict comparison - avoids false positives
/// like `utils.ts` matching `other-utils.ts`.
pub fn query_who_imports(snapshot: &Snapshot, target: &str) -> QueryResult {
    use std::collections::HashSet;

    let mut results = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    // Determine if target is a symbol name or file path
    let is_symbol = !target.contains('/') && !has_file_extension(target);

    // Collect starting files to check
    let mut to_check: Vec<String> = if is_symbol {
        // Resolve symbol to file paths first
        let symbol_query = query_where_symbol(snapshot, target);
        if symbol_query.results.is_empty() {
            return QueryResult {
                kind: "who-imports".to_string(),
                target: target.to_string(),
                results: vec![],
            };
        }
        symbol_query.results.into_iter().map(|m| m.file).collect()
    } else {
        vec![normalize_path(target)]
    };

    // For each initial file, also check folder variant (strip index suffix)
    let initial_files: Vec<String> = to_check.clone();
    for file in &initial_files {
        if let Some(folder) = strip_index_suffix(file) {
            to_check.push(folder.to_string());
        }
    }

    // BFS with depth limiting
    let mut depth = 0;
    while let Some(current) = to_check.pop() {
        // Safety: prevent infinite loops in pathological cases
        if depth > MAX_REEXPORT_DEPTH {
            break;
        }

        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());
        depth += 1;

        // If this looks like a folder, also check index file variants
        if !has_file_extension(&current) {
            for variant in index_variants(&current) {
                if !visited.contains(&variant) {
                    to_check.push(variant);
                }
            }
        }

        // Find edges pointing to current target
        for edge in &snapshot.edges {
            if paths_match(&edge.to, &current) {
                if edge.label == "reexport" {
                    // Follow re-export chain
                    if !visited.contains(&edge.from) {
                        to_check.push(edge.from.clone());
                    }
                } else {
                    // Regular import - this is an actual consumer
                    results.push(QueryMatch {
                        file: edge.from.clone(),
                        line: None,
                        context: Some(format!("imports via {}", edge.label)),
                    });
                }
            }
        }
    }

    // Deduplicate and sort results
    results.sort_by(|a, b| a.file.cmp(&b.file));
    results.dedup_by(|a, b| a.file == b.file);

    QueryResult {
        kind: "who-imports".to_string(),
        target: target.to_string(),
        results,
    }
}

/// Query for where a symbol is defined (where-symbol)
/// Uses exact matching first, then falls back to fuzzy/semantic matching
pub fn query_where_symbol(snapshot: &Snapshot, symbol: &str) -> QueryResult {
    use crate::analyzer::dead_parrots::find_similar;

    let mut results = Vec::new();

    // Use search_symbol from dead_parrots
    let search_result = search_symbol(symbol, &snapshot.files);

    for file_match in search_result.files {
        for symbol_match in file_match.matches {
            if matches!(
                symbol_match.kind,
                crate::analyzer::dead_parrots::SymbolMatchKind::Definition
            ) {
                results.push(QueryMatch {
                    file: file_match.file.clone(),
                    line: Some(symbol_match.line),
                    context: Some(symbol_match.context.clone()),
                });
            }
        }
    }

    // Fallback 1: if no matches found, look at exports directly
    if results.is_empty() {
        for file in &snapshot.files {
            for exp in &file.exports {
                if exp.name == symbol {
                    results.push(QueryMatch {
                        file: file.path.clone(),
                        line: exp.line,
                        context: Some(format!("export {}", exp.kind)),
                    });
                }
            }
        }
    }

    // Fallback 2: if still no matches, try fuzzy/semantic matching
    if results.is_empty() {
        let similar = find_similar(symbol, &snapshot.files);
        // Take top 5 fuzzy matches with score > 0.5
        for candidate in similar.into_iter().filter(|c| c.score > 0.5).take(5) {
            // For export matches, extract file path from "export in <path>" format
            let (file, context) = if candidate.file.starts_with("export in ") {
                let path = candidate
                    .file
                    .strip_prefix("export in ")
                    .unwrap_or(&candidate.file);
                (
                    path.to_string(),
                    format!(
                        "fuzzy match: {} (score: {:.2})",
                        candidate.symbol, candidate.score
                    ),
                )
            } else {
                (
                    candidate.symbol.clone(),
                    format!("fuzzy match (score: {:.2})", candidate.score),
                )
            };

            results.push(QueryMatch {
                file,
                line: None,
                context: Some(context),
            });
        }
    }

    QueryResult {
        kind: "where-symbol".to_string(),
        target: symbol.to_string(),
        results,
    }
}

/// Query for what component a file belongs to (component-of)
pub fn query_component_of(snapshot: &Snapshot, file: &str) -> QueryResult {
    let mut results = Vec::new();

    // Look for barrel files (index.ts) that re-export this file
    for barrel in &snapshot.barrels {
        if barrel
            .targets
            .iter()
            .any(|t| t == file || t.ends_with(file))
        {
            results.push(QueryMatch {
                file: barrel.path.clone(),
                line: None,
                context: Some(format!("barrel with {} re-exports", barrel.reexport_count)),
            });
        }
    }

    // Also check edges to find parent directories
    for edge in &snapshot.edges {
        if edge.to == file || edge.to.ends_with(file) {
            // Parent module that imports this file
            results.push(QueryMatch {
                file: edge.from.clone(),
                line: None,
                context: Some("parent module".to_string()),
            });
        }
    }

    QueryResult {
        kind: "component-of".to_string(),
        target: file.to_string(),
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileAnalysis;

    fn mock_snapshot() -> Snapshot {
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        // Add some test files
        let mut file1 = FileAnalysis::new("src/utils.ts".into());
        file1.exports.push(crate::types::ExportSymbol {
            name: "helper".to_string(),
            kind: "function".to_string(),
            export_type: "named".to_string(),
            line: Some(10),
            params: Vec::new(),
        });

        let mut file2 = FileAnalysis::new("src/app.ts".into());
        file2.exports.push(crate::types::ExportSymbol {
            name: "PostAuthBootstrapOverlay".to_string(),
            kind: "class".to_string(),
            export_type: "named".to_string(),
            line: Some(42),
            params: Vec::new(),
        });

        snapshot.files.push(file1);
        snapshot.files.push(file2);

        // Add an edge (app.ts imports utils.ts)
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/app.ts".to_string(),
            to: "src/utils.ts".to_string(),
            label: "import".to_string(),
        });

        snapshot
    }

    #[test]
    fn test_query_who_imports() {
        let snapshot = mock_snapshot();
        let result = query_who_imports(&snapshot, "src/utils.ts");

        assert_eq!(result.kind, "who-imports");
        assert_eq!(result.target, "src/utils.ts");
        assert!(!result.results.is_empty());
    }

    #[test]
    fn test_query_where_symbol() {
        let snapshot = mock_snapshot();
        let result = query_where_symbol(&snapshot, "helper");

        assert_eq!(result.kind, "where-symbol");
        assert_eq!(result.target, "helper");
    }

    #[test]
    fn test_query_where_symbol_substring_case_insensitive() {
        let snapshot = mock_snapshot();
        let result = query_where_symbol(&snapshot, "bootstrap");

        assert_eq!(result.kind, "where-symbol");
        assert_eq!(result.target, "bootstrap");
        assert!(
            result.results.iter().any(|r| r.file == "src/app.ts"),
            "Should find substring matches in exports"
        );
    }

    #[test]
    fn test_query_component_of() {
        let snapshot = mock_snapshot();
        let result = query_component_of(&snapshot, "src/utils.ts");

        assert_eq!(result.kind, "component-of");
        assert_eq!(result.target, "src/utils.ts");
    }

    #[test]
    fn test_query_who_imports_follows_reexport_chain() {
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        // Setup: App.tsx → index.ts (import) → Component.tsx (reexport)
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/App.tsx".to_string(),
            to: "src/features/index.ts".to_string(),
            label: "import".to_string(),
        });
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/features/index.ts".to_string(),
            to: "src/features/Component.tsx".to_string(),
            label: "reexport".to_string(),
        });

        // Query who imports Component.tsx - should find App.tsx through the chain
        let result = query_who_imports(&snapshot, "src/features/Component.tsx");

        assert_eq!(result.kind, "who-imports");
        assert!(
            !result.results.is_empty(),
            "Should find App.tsx as importer"
        );
        assert!(
            result.results.iter().any(|r| r.file == "src/App.tsx"),
            "App.tsx should be in results"
        );
    }

    #[test]
    fn test_query_who_imports_multi_level_reexport() {
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        // Setup: App.tsx → ai-suite/index.ts → system/index.ts → AISystemHost.tsx
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/App.tsx".to_string(),
            to: "src/features/ai-suite/index.ts".to_string(),
            label: "import".to_string(),
        });
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/features/ai-suite/index.ts".to_string(),
            to: "src/features/ai-suite/system".to_string(),
            label: "reexport".to_string(),
        });
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/features/ai-suite/system/index.ts".to_string(),
            to: "src/features/ai-suite/system/AISystemHost.tsx".to_string(),
            label: "reexport".to_string(),
        });

        // Query who imports AISystemHost.tsx - should find App.tsx through the 3-level chain
        let result = query_who_imports(&snapshot, "src/features/ai-suite/system/AISystemHost.tsx");

        assert!(
            !result.results.is_empty(),
            "Should find importers through re-export chain"
        );
    }

    // ========================================
    // Path matching tests (stricter matching)
    // ========================================

    #[test]
    fn test_paths_match_exact() {
        assert!(paths_match("src/utils.ts", "src/utils.ts"));
        assert!(paths_match("./src/utils.ts", "src/utils.ts"));
        assert!(paths_match("src/utils.ts", "./src/utils.ts"));
    }

    #[test]
    fn test_paths_match_suffix() {
        assert!(paths_match("src/components/utils.ts", "utils.ts"));
        assert!(paths_match("src/deep/nested/file.ts", "file.ts"));
    }

    #[test]
    fn test_paths_match_no_false_positives() {
        // CRITICAL: utils.ts should NOT match other-utils.ts
        assert!(!paths_match("src/other-utils.ts", "utils.ts"));
        assert!(!paths_match("src/my-utils.ts", "utils.ts"));
        assert!(!paths_match("src/utils-helper.ts", "utils.ts"));
    }

    #[test]
    fn test_paths_match_folder_to_index() {
        // foo/index.ts should match foo
        assert!(paths_match("src/components", "src/components/index.ts"));
        assert!(paths_match("features", "features/index.tsx"));
    }

    #[test]
    fn test_index_variants() {
        let variants = index_variants("src/components");
        assert_eq!(variants.len(), 3);
        assert!(variants.contains(&"src/components/index.ts".to_string()));
        assert!(variants.contains(&"src/components/index.tsx".to_string()));
        assert!(variants.contains(&"src/components/index.js".to_string()));
    }

    #[test]
    fn test_strip_index_suffix() {
        assert_eq!(strip_index_suffix("foo/bar/index.ts"), Some("foo/bar"));
        assert_eq!(strip_index_suffix("foo/bar/index.tsx"), Some("foo/bar"));
        assert_eq!(strip_index_suffix("foo/bar/index.js"), Some("foo/bar"));
        assert_eq!(strip_index_suffix("foo/bar/utils.ts"), None);
        assert_eq!(strip_index_suffix("foo/bar"), None);
    }

    #[test]
    fn test_has_file_extension() {
        assert!(has_file_extension("foo.ts"));
        assert!(has_file_extension("bar.tsx"));
        assert!(has_file_extension("baz.rs"));
        assert!(has_file_extension("qux.py"));
        assert!(!has_file_extension("foo"));
        assert!(!has_file_extension("foo/bar"));
    }

    #[test]
    fn test_query_who_imports_stricter_matching() {
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        // Setup: app.ts imports utils.ts, NOT other-utils.ts
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/app.ts".to_string(),
            to: "src/utils.ts".to_string(),
            label: "import".to_string(),
        });
        snapshot.edges.push(crate::snapshot::GraphEdge {
            from: "src/other.ts".to_string(),
            to: "src/other-utils.ts".to_string(),
            label: "import".to_string(),
        });

        // Query who imports utils.ts - should find app.ts but NOT other.ts
        let result = query_who_imports(&snapshot, "src/utils.ts");

        assert!(
            result.results.iter().any(|r| r.file == "src/app.ts"),
            "Should find app.ts as importer of utils.ts"
        );
        assert!(
            !result.results.iter().any(|r| r.file == "src/other.ts"),
            "Should NOT find other.ts (imports other-utils.ts, not utils.ts)"
        );
    }
}
