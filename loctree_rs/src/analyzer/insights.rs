use std::collections::HashMap;
use std::path::Path;

use super::{AiInsight, CommandGap, RankedDup};
use crate::types::FileAnalysis;

/// Detect files with the same stem (filename without extension) across different languages.
/// This helps identify potential binding pairs (e.g., py/ts/rs files that wrap the same functionality).
fn find_cross_lang_stem_matches(files: &[FileAnalysis]) -> Vec<(String, Vec<(String, String)>)> {
    let binding_langs: &[&str] = &["py", "ts", "rs", "js"];

    // Group files by stem -> Vec<(path, language)>
    let mut stem_map: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for file in files {
        // Skip test/generated files
        if file.is_test || file.is_generated {
            continue;
        }

        // Only consider binding-relevant languages
        if !binding_langs.contains(&file.language.as_str()) {
            continue;
        }

        let path = Path::new(&file.path);
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Skip common generic names that would create noise
            let lower_stem = stem.to_lowercase();
            if matches!(
                lower_stem.as_str(),
                "index" | "mod" | "lib" | "main" | "utils" | "helpers" | "types" | "constants"
            ) {
                continue;
            }

            stem_map
                .entry(stem.to_string())
                .or_default()
                .push((file.path.clone(), file.language.clone()));
        }
    }

    // Filter to only stems with multiple languages
    let mut matches: Vec<(String, Vec<(String, String)>)> = stem_map
        .into_iter()
        .filter(|(_, entries)| {
            let langs: std::collections::HashSet<_> = entries.iter().map(|(_, l)| l).collect();
            langs.len() > 1 // At least 2 different languages
        })
        .collect();

    // Sort for deterministic output
    matches.sort_by(|a, b| a.0.cmp(&b.0));
    matches
}

pub fn collect_ai_insights(
    files: &[FileAnalysis],
    dups: &[RankedDup],
    cascades: &[(String, String)],
    gap_missing: &[CommandGap],
    _gap_unused: &[CommandGap],
) -> Vec<AiInsight> {
    let mut insights = Vec::new();

    // Cross-language stem hint (Objective 7)
    let cross_lang_matches = find_cross_lang_stem_matches(files);
    if !cross_lang_matches.is_empty() {
        let examples: Vec<String> = cross_lang_matches
            .iter()
            .take(5)
            .map(|(stem, entries)| {
                let langs: Vec<_> = entries.iter().map(|(_, l)| l.as_str()).collect();
                format!("'{}' ({})", stem, langs.join("/"))
            })
            .collect();

        insights.push(AiInsight {
            title: "Potential cross-language binding pairs".to_string(),
            severity: "info".to_string(),
            message: format!(
                "Found {} file stem(s) shared across languages: {}. These may be binding pairs (e.g., Python/Rust FFI or TS/Rust Tauri commands). Check if they should share types/interfaces.",
                cross_lang_matches.len(),
                examples.join(", ")
            ),
        });
    }

    let huge_files: Vec<_> = files.iter().filter(|f| f.loc > 2000).collect();
    if !huge_files.is_empty() {
        insights.push(AiInsight {
            title: "Huge files detected".to_string(),
            severity: "medium".to_string(),
            message: format!(
                "Found {} files with > 2000 LOC (e.g. {}). Consider splitting them.",
                huge_files.len(),
                huge_files[0].path
            ),
        });
    }

    if dups.len() > 10 {
        insights.push(AiInsight {
            title: "High number of duplicate exports".to_string(),
            severity: "medium".to_string(),
            message: format!(
                "Found {} duplicate export groups. Consider refactoring.",
                dups.len()
            ),
        });
    }

    if cascades.len() > 20 {
        insights.push(AiInsight {
            title: "Many re-export chains".to_string(),
            severity: "low".to_string(),
            message: format!(
                "Found {} re-export cascades. This might affect tree-shaking/bundling.",
                cascades.len()
            ),
        });
    }

    if !gap_missing.is_empty() {
        insights.push(AiInsight {
            title: "Missing Tauri Handlers".to_string(),
            severity: "high".to_string(),
            message: format!(
                "Frontend calls {} commands that are missing in Backend.",
                gap_missing.len()
            ),
        });
    }

    insights
}
