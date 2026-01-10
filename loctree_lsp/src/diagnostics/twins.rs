//! Twin/duplicate diagnostics
//!
//! Converts twin findings to LSP diagnostics.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    Position, Range, Url,
};

use crate::snapshot::SnapshotState;

/// Info about a twin export
#[derive(Debug, Clone)]
pub struct TwinInfo {
    pub symbol: String,
    pub locations: Vec<String>,
    pub line: usize,
}

/// Generate diagnostics for duplicate exports in a file
#[allow(dead_code)]
pub async fn twin_diagnostics(_snapshot: &SnapshotState, _file_path: &str) -> Vec<Diagnostic> {
    // TODO: Implement when snapshot exposes twin data
    // For now, return empty - twins require cross-file analysis
    Vec::new()
}

/// Create a twin diagnostic (for future use)
#[allow(dead_code)]
fn create_twin_diagnostic(twin: TwinInfo, current_file: &str) -> Diagnostic {
    // Create related information for other locations
    let related = twin
        .locations
        .iter()
        .filter(|loc| *loc != current_file)
        .filter_map(|loc| {
            Url::from_file_path(loc)
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
                    message: format!("Also exports '{}'", twin.symbol),
                })
        })
        .collect::<Vec<_>>();

    let other_count = twin.locations.len() - 1;
    let line = twin.line.saturating_sub(1) as u32;

    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: 100,
            },
        },
        severity: Some(DiagnosticSeverity::INFORMATION),
        code: Some(NumberOrString::String("duplicate-export".to_string())),
        code_description: None,
        source: Some("loctree".to_string()),
        message: format!(
            "Symbol '{}' also exported from {} other file{}",
            twin.symbol,
            other_count,
            if other_count == 1 { "" } else { "s" }
        ),
        related_information: if related.is_empty() {
            None
        } else {
            Some(related)
        },
        tags: None,
        data: None,
    }
}
