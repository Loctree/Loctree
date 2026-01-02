//! Query API for fast lookups against the cached snapshot.
//!
//! Provides interactive queries without re-scanning:
//! - `who-imports <file>` - Find all files that import a given file
//! - `where-symbol <symbol>` - Find where a symbol is defined
//! - `component-of <file>` - Show what component/module a file belongs to

use serde::{Deserialize, Serialize};

use crate::analyzer::dead_parrots::search_symbol;
use crate::snapshot::Snapshot;

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
/// Follows re-export chains transitively to find all importers
///
/// If the input looks like a symbol name (no path separators), it will first
/// resolve the symbol to file paths where it's defined, then find importers.
pub fn query_who_imports(snapshot: &Snapshot, target: &str) -> QueryResult {
    use std::collections::HashSet;

    let mut results = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    // Determine if target is a symbol name or file path
    // Symbol names typically don't contain '/' or file extensions
    let is_symbol = !target.contains('/')
        && !target.ends_with(".ts")
        && !target.ends_with(".tsx")
        && !target.ends_with(".js")
        && !target.ends_with(".jsx")
        && !target.ends_with(".rs")
        && !target.ends_with(".py");

    // Collect starting files to check
    let mut to_check: Vec<String> = if is_symbol {
        // Resolve symbol to file paths
        let symbol_query = query_where_symbol(snapshot, target);
        if symbol_query.results.is_empty() {
            // No definition found - return empty result
            return QueryResult {
                kind: "who-imports".to_string(),
                target: target.to_string(),
                results: vec![],
            };
        }
        // Start with files where symbol is defined
        symbol_query.results.into_iter().map(|m| m.file).collect()
    } else {
        // Target is already a file path
        vec![target.to_string()]
    };

    // For each initial file, also check variants without index suffix
    let initial_files: Vec<String> = to_check.clone();
    for file in &initial_files {
        let normalized = file
            .trim_end_matches("/index.ts")
            .trim_end_matches("/index.tsx")
            .trim_end_matches("/index.js");
        if normalized != file {
            to_check.push(normalized.to_string());
        }
    }

    // BFS to follow re-export chains
    while let Some(current) = to_check.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());

        // Also add the index.ts variant if this is a folder path
        // This handles: edge.to = "foo/bar" when we need to match edge.from = "foo/bar/index.ts"
        if !current.ends_with(".ts") && !current.ends_with(".tsx") && !current.ends_with(".js") {
            let index_variants = [
                format!("{}/index.ts", current),
                format!("{}/index.tsx", current),
                format!("{}/index.js", current),
            ];
            for variant in index_variants {
                if !visited.contains(&variant) {
                    to_check.push(variant);
                }
            }
        }

        for edge in &snapshot.edges {
            // Check if this edge points to our target (direct or via folder)
            // Also handle folder references: if current is "foo/index.ts", match edge.to = "foo"
            let current_folder = current
                .strip_suffix("/index.ts")
                .or_else(|| current.strip_suffix("/index.tsx"))
                .or_else(|| current.strip_suffix("/index.js"));

            let matches = edge.to == current
                || edge.to.ends_with(&format!("/{}", current))
                || (current.contains('/') && edge.to.contains(&current))
                || current_folder
                    .map(|f| edge.to == f || edge.to.ends_with(f))
                    .unwrap_or(false);

            if matches {
                // If this is a reexport, add the source to our search queue
                // This follows the chain: App.tsx → index.ts → Component.tsx
                if edge.label == "reexport" {
                    if !visited.contains(&edge.from) {
                        to_check.push(edge.from.clone());
                    }
                } else {
                    // Regular import - this is an actual importer
                    results.push(QueryMatch {
                        file: edge.from.clone(),
                        line: None,
                        context: Some(format!("imports via {}", edge.label)),
                    });
                }
            }
        }
    }

    // Deduplicate results
    results.sort_by(|a, b| a.file.cmp(&b.file));
    results.dedup_by(|a, b| a.file == b.file);

    QueryResult {
        kind: "who-imports".to_string(),
        target: target.to_string(),
        results,
    }
}

/// Query for where a symbol is defined (where-symbol)
pub fn query_where_symbol(snapshot: &Snapshot, symbol: &str) -> QueryResult {
    let mut results = Vec::new();

    // Use search_symbol from dead_parrots
    let search_result = search_symbol(symbol, &snapshot.files);

    for file_match in search_result.files {
        for symbol_match in file_match.matches {
            if symbol_match.is_definition {
                results.push(QueryMatch {
                    file: file_match.file.clone(),
                    line: Some(symbol_match.line),
                    context: Some(symbol_match.context.clone()),
                });
            }
        }
    }

    // Fallback: if no matches found, look at exports directly
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
        });

        let mut file2 = FileAnalysis::new("src/app.ts".into());
        file2.exports.push(crate::types::ExportSymbol {
            name: "PostAuthBootstrapOverlay".to_string(),
            kind: "class".to_string(),
            export_type: "named".to_string(),
            line: Some(42),
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
}
