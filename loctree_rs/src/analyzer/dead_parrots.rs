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

use super::root_scan::{normalize_module_id, RootContext};

/// Result of symbol search across the codebase
pub struct SymbolSearchResult {
    pub found: bool,
    pub matches: Vec<serde_json::Value>,
}

/// Result of impact analysis
pub struct ImpactResult {
    pub targets: Vec<String>,
    pub dependents: Vec<String>,
}

/// Result of similarity check
pub struct SimilarityCandidate {
    pub name: String,
    pub location: String,
    pub score: f64,
}

/// Result of dead exports analysis
pub struct DeadExport {
    pub file: String,
    pub symbol: String,
    pub line: Option<usize>,
    pub confidence: String,
}

/// Search for symbol occurrences across analyzed files
/// Note: The actual symbol search is performed during file scanning (in `analyze_file`).
/// This function only collects the pre-computed matches from analyses.
pub fn search_symbol(_symbol: &str, analyses: &[FileAnalysis]) -> SymbolSearchResult {
    let mut matches = Vec::new();
    for analysis in analyses {
        if !analysis.matches.is_empty() {
            matches.push(json!({
                "path": analysis.path,
                "matches": analysis.matches
            }));
        }
    }
    SymbolSearchResult {
        found: !matches.is_empty(),
        matches,
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
            serde_json::to_string_pretty(&result.matches).unwrap_or_default()
        );
    } else {
        println!(
            "Symbol '{}' found in {} files:",
            symbol,
            result.matches.len()
        );
        for m in &result.matches {
            let path = m["path"].as_str().unwrap_or_default();
            let items = m["matches"].as_array();
            println!("\nFile: {}", path);
            if let Some(items) = items {
                for item in items {
                    let line = item["line"].as_u64().unwrap_or(0);
                    let ctx = item["context"].as_str().unwrap_or_default();
                    println!("  {}: {}", line, ctx);
                }
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
        if analysis.path.contains(target_path) {
            targets.push(analysis.path.clone());
        }
    }

    if targets.is_empty() {
        return None;
    }

    let normalized_targets: HashSet<String> =
        targets.iter().map(|t| normalize_module_id(t)).collect();
    let mut dependent_ids = HashSet::new();

    for ctx in contexts {
        for (source, target, _weight) in &ctx.graph_edges {
            if normalized_targets.contains(target) {
                dependent_ids.insert(source.clone());
            }
        }
    }

    let mut deps = Vec::new();
    for analysis in analyses {
        let id = normalize_module_id(&analysis.path);
        if dependent_ids.contains(&id) {
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
                name: analysis.path.clone(),
                location: "file path".to_string(),
                score: path_score,
            });
        }

        // Check exported symbols
        for exp in &analysis.exports {
            let sym_score = similarity(query, &exp.name);
            if sym_score > 0.4 {
                candidates.push(SimilarityCandidate {
                    name: exp.name.clone(),
                    location: format!("export in {}", analysis.path),
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
    candidates.dedup_by(|a, b| a.name == b.name && a.location == b.location);
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
                    "name": c.name,
                    "location": c.location,
                    "similarity": c.score
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_items).unwrap_or_default()
        );
    } else {
        println!("Checking for '{}' (similarity > 0.3):", query);
        if candidates.is_empty() {
            println!("  No similar components or symbols found.");
        } else {
            for c in candidates {
                println!("  - {} ({}) [score: {:.2}]", c.name, c.location, c.score);
            }
        }
    }
}

/// Find potentially dead (unused) exports in the codebase
pub fn find_dead_exports(analyses: &[FileAnalysis], high_confidence: bool) -> Vec<DeadExport> {
    // Build usage set: (resolved_path, symbol_name)
    let mut used_exports: HashSet<(String, String)> = HashSet::new();

    for analysis in analyses {
        for imp in &analysis.imports {
            if let Some(target) = &imp.resolved_path {
                let target_norm = normalize_module_id(target);
                // Track named imports
                for sym in &imp.symbols {
                    used_exports.insert((target_norm.clone(), sym.name.clone()));
                }
            }
        }
        // Track re-exports as usage (if A re-exports B, A uses B)
        for re in &analysis.reexports {
            if let Some(target) = &re.resolved {
                let target_norm = normalize_module_id(target);
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
    }

    // Identify dead exports
    let mut dead_candidates = Vec::new();

    for analysis in analyses {
        if analysis.is_test
            || analysis.path.contains("stories")
            || analysis.path.contains("__tests__")
        {
            continue;
        }
        let path_norm = normalize_module_id(&analysis.path);

        // Skip if file is dynamically imported (assume all exports used)
        let is_dyn_imported = analyses.iter().any(|a| {
            a.dynamic_imports
                .iter()
                .any(|imp| imp.contains(&path_norm) || imp.contains(&analysis.path))
        });
        if is_dyn_imported {
            continue;
        }

        for exp in &analysis.exports {
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

            if !is_used && !star_used {
                dead_candidates.push(DeadExport {
                    file: analysis.path.clone(),
                    symbol: exp.name.clone(),
                    line: exp.line,
                    confidence: if high_confidence {
                        "very-high".to_string()
                    } else {
                        "high".to_string()
                    },
                });
            }
        }
    }

    dead_candidates
}

/// Print dead exports results to stdout
pub fn print_dead_exports(dead_exports: &[DeadExport], output: OutputMode, high_confidence: bool) {
    if matches!(output, OutputMode::Json) {
        let json_items: Vec<_> = dead_exports
            .iter()
            .map(|d| {
                json!({
                    "file": d.file,
                    "symbol": d.symbol,
                    "line": d.line,
                    "confidence": d.confidence
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_items).unwrap_or_default()
        );
    } else {
        let count = dead_exports.len();
        let suffix = if high_confidence {
            " (high confidence)"
        } else {
            ""
        };
        println!("Potential Dead Exports ({} found){}:", count, suffix);
        for item in dead_exports.iter().take(50) {
            println!("  - {} in {}", item.symbol, item.file);
        }
        if count > 50 {
            println!("  ... and {} more", count - 50);
        }
    }
}
