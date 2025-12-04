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

/// Query for files that import a given file (who-imports)
pub fn query_who_imports(snapshot: &Snapshot, file: &str) -> QueryResult {
    let mut results = Vec::new();

    // Simple implementation: check edges directly
    for edge in &snapshot.edges {
        if edge.to.contains(file) || edge.to == file {
            results.push(QueryMatch {
                file: edge.from.clone(),
                line: None,
                context: Some(format!("imports via {}", edge.label)),
            });
        }
    }

    // Deduplicate results
    results.sort_by(|a, b| a.file.cmp(&b.file));
    results.dedup_by(|a, b| a.file == b.file);

    QueryResult {
        kind: "who-imports".to_string(),
        target: file.to_string(),
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

        let file2 = FileAnalysis::new("src/app.ts".into());

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
    fn test_query_component_of() {
        let snapshot = mock_snapshot();
        let result = query_component_of(&snapshot, "src/utils.ts");

        assert_eq!(result.kind, "component-of");
        assert_eq!(result.target, "src/utils.ts");
    }
}
