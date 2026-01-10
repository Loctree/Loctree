//! Cycle diagnostics
//!
//! Converts circular import findings to LSP diagnostics.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    Position, Range, Url,
};

use crate::snapshot::SnapshotState;

/// Generate diagnostics for circular imports in a file
pub async fn cycle_diagnostics(snapshot: &SnapshotState, file_path: &str) -> Vec<Diagnostic> {
    let cycles = snapshot.cycles_for_file(file_path).await;

    cycles
        .into_iter()
        .map(|cycle| {
            let severity = match cycle.cycle_type.as_str() {
                "breaking" | "bidirectional" => DiagnosticSeverity::WARNING,
                "structural" => DiagnosticSeverity::WARNING,
                _ => DiagnosticSeverity::INFORMATION,
            };

            let cycle_str = cycle.files.join(" -> ");

            // Create related information for other files in cycle
            let related = cycle
                .files
                .iter()
                .filter(|f| !f.ends_with(file_path) && !file_path.ends_with(*f))
                .filter_map(|f| {
                    Url::from_file_path(f)
                        .ok()
                        .map(|uri| DiagnosticRelatedInformation {
                            location: Location {
                                uri,
                                range: Range {
                                    start: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                },
                            },
                            message: "Part of import cycle".to_string(),
                        })
                })
                .collect::<Vec<_>>();

            Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 100,
                    },
                },
                severity: Some(severity),
                code: Some(NumberOrString::String("circular-import".to_string())),
                code_description: None,
                source: Some("loctree".to_string()),
                message: format!("Circular import: {}", cycle_str),
                related_information: if related.is_empty() {
                    None
                } else {
                    Some(related)
                },
                tags: None,
                data: None,
            }
        })
        .collect()
}
