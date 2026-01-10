//! Dead export diagnostics
//!
//! Converts dead export findings to LSP diagnostics.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

use crate::snapshot::SnapshotState;

/// Generate diagnostics for dead exports in a file
pub async fn dead_export_diagnostics(snapshot: &SnapshotState, file_path: &str) -> Vec<Diagnostic> {
    let dead_exports = snapshot.dead_exports_for_file(file_path).await;

    dead_exports
        .into_iter()
        .map(|dead| {
            let severity = match dead.confidence.as_str() {
                "high" | "very-high" => DiagnosticSeverity::WARNING,
                "normal" => DiagnosticSeverity::INFORMATION,
                _ => DiagnosticSeverity::HINT,
            };

            // Line numbers in LSP are 0-indexed
            let line = dead.line.saturating_sub(1) as u32;

            Diagnostic {
                range: Range {
                    start: Position { line, character: 0 },
                    end: Position {
                        line,
                        character: 100,
                    },
                },
                severity: Some(severity),
                code: Some(NumberOrString::String("dead-export".to_string())),
                code_description: None,
                source: Some("loctree".to_string()),
                message: format!("Export '{}' is unused (0 imports)", dead.symbol),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect()
}
