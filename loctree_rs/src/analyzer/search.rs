//! Unified search - aggregates symbol, semantic, and dead code results in one call
//!
//! Agent-friendly: no need to know which flag to use, get everything at once.

use crate::analyzer::dead_parrots::{
    SimilarityCandidate, SymbolSearchResult, find_dead_exports, find_similar, search_symbol,
};
use crate::types::{FileAnalysis, OutputMode};
use serde::Serialize;
use serde_json::json;

/// Aggregated search results
#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub symbol_matches: SymbolSearchResult,
    pub semantic_matches: Vec<SimilarityCandidate>,
    pub dead_status: DeadStatus,
}

/// Dead code status for the searched symbol
#[derive(Debug, Serialize)]
pub struct DeadStatus {
    pub is_exported: bool,
    pub is_dead: bool,
    pub dead_in_files: Vec<String>,
}

/// Run unified search - returns all result types
pub fn run_search(query: &str, analyses: &[FileAnalysis]) -> SearchResults {
    // 1. Symbol matches
    let symbol_matches = search_symbol(query, analyses);

    // 2. Semantic/similarity matches
    let semantic_matches = find_similar(query, analyses);

    // 3. Dead code status - check if query appears in dead exports
    let all_dead = find_dead_exports(analyses, false);
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

    SearchResults {
        query: query.to_string(),
        symbol_matches,
        semantic_matches,
        dead_status,
    }
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
