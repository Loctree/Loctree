//! Output formatting for dead parrots analysis results

use serde_json::json;

use crate::types::OutputMode;

use super::{
    DeadExport,
    search::{ImpactResult, SimilarityCandidate, SymbolSearchResult},
};

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

            // Map confidence string to emoji
            let emoji = match item.confidence.as_str() {
                "certain" => "🔴",
                "high" | "very-high" => "🟡",
                "medium" | "smell" => "🟢",
                _ => "⚪",
            };

            println!(
                "  {} {} - {} in {}",
                emoji,
                item.confidence.to_uppercase(),
                item.symbol,
                location
            );
            println!("     {}", item.reason);
        }
        if count > limit {
            println!("  ... and {} more", count - limit);
        }
    }
}
