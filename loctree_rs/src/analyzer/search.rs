//! Unified search - aggregates symbol, semantic, and dead code results in one call
//!
//! Agent-friendly: no need to know which flag to use, get everything at once.

use crate::analyzer::dead_parrots::{
    DeadFilterConfig, SimilarityCandidate, SymbolSearchResult, find_dead_exports, find_similar,
    search_symbol,
};
use crate::types::{FileAnalysis, OutputMode};
use serde::Serialize;
use serde_json::json;

/// A match for a parameter in a function export.
#[derive(Debug, Serialize)]
pub struct ParamMatch {
    pub file: String,
    pub line: Option<usize>,
    pub function: String,
    pub param_name: String,
    pub param_type: Option<String>,
}

/// Aggregated search results
#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub symbol_matches: SymbolSearchResult,
    pub param_matches: Vec<ParamMatch>,
    pub semantic_matches: Vec<SimilarityCandidate>,
    pub dead_status: DeadStatus,
    /// Files containing 2+ different query terms (multi-query cross-match)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cross_matches: Vec<CrossMatchFile>,
}

/// A file that matches multiple query terms (cross-match)
#[derive(Debug, Serialize)]
pub struct CrossMatchFile {
    pub file: String,
    pub matched_terms: Vec<CrossMatchTerm>,
}

/// A single term match within a cross-match file
#[derive(Debug, Serialize)]
pub struct CrossMatchTerm {
    pub term: String,
    pub line: usize,
    pub context: String,
}

/// Dead code status for the searched symbol
#[derive(Debug, Serialize)]
pub struct DeadStatus {
    pub is_exported: bool,
    pub is_dead: bool,
    pub dead_in_files: Vec<String>,
}

/// Search for a query in function parameters across all analyses.
fn search_params(query: &str, analyses: &[FileAnalysis]) -> Vec<ParamMatch> {
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    for analysis in analyses {
        for export in &analysis.exports {
            for param in &export.params {
                if param.name.to_lowercase().contains(&query_lower) {
                    matches.push(ParamMatch {
                        file: analysis.path.clone(),
                        line: export.line,
                        function: export.name.clone(),
                        param_name: param.name.clone(),
                        param_type: param.type_annotation.clone(),
                    });
                }
            }
        }
    }

    matches
}

/// Run unified search - returns all result types
pub fn run_search(query: &str, analyses: &[FileAnalysis]) -> SearchResults {
    // 1. Symbol matches
    let symbol_matches = search_symbol(query, analyses);

    // 2. Parameter matches (NEW)
    let param_matches = search_params(query, analyses);

    // 3. Semantic/similarity matches
    let semantic_matches = find_similar(query, analyses);

    // 4. Dead code status - check if query appears in dead exports
    let all_dead = find_dead_exports(analyses, false, None, DeadFilterConfig::default());
    let dead_for_query: Vec<_> = all_dead
        .iter()
        .filter(|d| d.symbol.to_lowercase().contains(&query.to_lowercase()))
        .collect();

    let is_exported = !symbol_matches.files.is_empty()
        || analyses.iter().any(|a| {
            a.exports
                .iter()
                .any(|e| e.name.to_lowercase().contains(&query.to_lowercase()))
        });

    let dead_status = DeadStatus {
        is_exported,
        is_dead: !dead_for_query.is_empty(),
        dead_in_files: dead_for_query.iter().map(|d| d.file.clone()).collect(),
    };

    // 5. Cross-match analysis - find files with 2+ different query terms
    let cross_matches = if query.contains('|') {
        compute_cross_matches(query, analyses)
    } else {
        vec![]
    };

    SearchResults {
        query: query.to_string(),
        symbol_matches,
        param_matches,
        semantic_matches,
        dead_status,
        cross_matches,
    }
}

/// Compute cross-matches: files containing 2+ different terms from a multi-query
fn compute_cross_matches(query: &str, analyses: &[FileAnalysis]) -> Vec<CrossMatchFile> {
    use std::collections::HashMap;

    // Split query into individual terms
    let terms: Vec<&str> = query.split('|').filter(|t| !t.is_empty()).collect();
    if terms.len() < 2 {
        return vec![];
    }

    // For each file, track which terms match
    let mut file_matches: HashMap<String, Vec<CrossMatchTerm>> = HashMap::new();

    for analysis in analyses {
        for term in &terms {
            let term_lower = term.to_lowercase();

            // Check exports
            for exp in &analysis.exports {
                if exp.name.to_lowercase().contains(&term_lower) {
                    file_matches
                        .entry(analysis.path.clone())
                        .or_default()
                        .push(CrossMatchTerm {
                            term: term.to_string(),
                            line: exp.line.unwrap_or(0),
                            context: format!("{} {}", exp.kind, exp.name),
                        });
                }
            }
        }
    }

    // Filter to files with 2+ DIFFERENT terms
    let mut results: Vec<CrossMatchFile> = file_matches
        .into_iter()
        .filter_map(|(file, matches)| {
            // Count unique terms
            let unique_terms: std::collections::HashSet<_> =
                matches.iter().map(|m| &m.term).collect();
            if unique_terms.len() >= 2 {
                Some(CrossMatchFile {
                    file,
                    matched_terms: matches,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by number of matched terms (most first)
    results.sort_by(|a, b| b.matched_terms.len().cmp(&a.matched_terms.len()));
    results
}

/// Print search results
pub fn print_search_results(
    results: &SearchResults,
    output: OutputMode,
    symbol_only: bool,
    dead_only: bool,
    semantic_only: bool,
) {
    if matches!(output, OutputMode::Json) {
        print_search_json(results, symbol_only, dead_only, semantic_only);
        return;
    }

    if matches!(output, OutputMode::Jsonl) {
        print_search_jsonl(results, symbol_only, dead_only, semantic_only);
        return;
    }

    // Human-readable output
    println!("Search results for: {}\n", results.query);

    // Symbol matches
    if !dead_only && !semantic_only {
        println!(
            "=== Symbol Matches ({}) ===",
            results.symbol_matches.total_matches
        );
        if results.symbol_matches.files.is_empty() {
            println!("  No symbol matches found.\n");
        } else {
            for file_match in &results.symbol_matches.files {
                println!("  File: {}", file_match.file);
                for m in &file_match.matches {
                    let kind = if m.is_definition { "DEF" } else { "USE" };
                    println!(
                        "    [{}] {}:{} - {}",
                        kind, file_match.file, m.line, m.context
                    );
                }
            }
            println!();
        }
    }

    // Parameter matches (NEW)
    if !dead_only && !semantic_only && !results.param_matches.is_empty() {
        println!(
            "=== Parameter Matches ({}) ===",
            results.param_matches.len()
        );
        for pm in &results.param_matches {
            let type_info = pm
                .param_type
                .as_ref()
                .map(|t| format!(": {}", t))
                .unwrap_or_default();
            let line_info = pm.line.map(|l| format!(":{}", l)).unwrap_or_default();
            println!(
                "  {}{} - {}{} in {}()",
                pm.file, line_info, pm.param_name, type_info, pm.function
            );
        }
        println!();
    }

    // Cross-match files (multi-query: files with 2+ different terms)
    if !dead_only && !semantic_only && !results.cross_matches.is_empty() {
        println!(
            "=== Cross-Match Files ({}) ===",
            results.cross_matches.len()
        );
        println!("  Files containing 2+ different query terms:\n");
        for cm in &results.cross_matches {
            let unique_terms: std::collections::HashSet<_> =
                cm.matched_terms.iter().map(|t| &t.term).collect();
            println!("  {} ({} terms)", cm.file, unique_terms.len());
            for term in &cm.matched_terms {
                println!(
                    "    ├─ {} (line {}) - {}",
                    term.term, term.line, term.context
                );
            }
        }
        println!();
    }

    // Semantic matches
    if !dead_only && !symbol_only {
        println!(
            "=== Semantic Matches ({}) ===",
            results.semantic_matches.len()
        );
        if results.semantic_matches.is_empty() {
            println!("  No semantic matches found.\n");
        } else {
            for candidate in &results.semantic_matches {
                println!("  {} (score: {:.2})", candidate.symbol, candidate.score);
                println!("    in {}", candidate.file);
            }
            println!();
        }
    }

    // Dead code status
    if !symbol_only && !semantic_only {
        println!("=== Dead Code Status ===");
        if !results.dead_status.is_exported {
            println!("  Symbol not found as export.\n");
        } else if results.dead_status.is_dead {
            println!("  WARNING: Symbol appears to be dead code in:");
            for file in &results.dead_status.dead_in_files {
                println!("    - {}", file);
            }
            println!();
        } else {
            println!("  OK: Symbol is used.\n");
        }
    }
}

fn print_search_json(
    results: &SearchResults,
    symbol_only: bool,
    dead_only: bool,
    semantic_only: bool,
) {
    let output = if symbol_only {
        json!({
            "query": results.query,
            "symbol_matches": results.symbol_matches,
            "param_matches": results.param_matches,
        })
    } else if dead_only {
        json!({
            "query": results.query,
            "dead_status": results.dead_status,
        })
    } else if semantic_only {
        json!({
            "query": results.query,
            "semantic_matches": results.semantic_matches,
        })
    } else {
        json!({
            "query": results.query,
            "symbol_matches": results.symbol_matches,
            "param_matches": results.param_matches,
            "semantic_matches": results.semantic_matches,
            "dead_status": results.dead_status,
        })
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn print_search_jsonl(
    results: &SearchResults,
    symbol_only: bool,
    dead_only: bool,
    semantic_only: bool,
) {
    // Each result type on its own line
    if !dead_only && !semantic_only {
        println!(
            "{}",
            json!({"type": "symbol_matches", "data": results.symbol_matches})
        );
        if !results.param_matches.is_empty() {
            println!(
                "{}",
                json!({"type": "param_matches", "data": results.param_matches})
            );
        }
    }
    if !dead_only && !symbol_only {
        println!(
            "{}",
            json!({"type": "semantic_matches", "data": results.semantic_matches})
        );
    }
    if !symbol_only && !semantic_only {
        println!(
            "{}",
            json!({"type": "dead_status", "data": results.dead_status})
        );
    }
}
