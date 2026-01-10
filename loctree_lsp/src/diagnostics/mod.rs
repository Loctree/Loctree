//! Diagnostics generation for loctree LSP
//!
//! Converts loctree analysis results into LSP Diagnostic objects.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

mod cycles;
mod dead;
mod twins;

pub use cycles::cycle_diagnostics;
pub use dead::dead_export_diagnostics;
pub use twins::twin_diagnostics;

use tower_lsp::lsp_types::Diagnostic;

use crate::snapshot::SnapshotState;

/// Collect all diagnostics for a file
pub async fn collect_diagnostics(snapshot: &SnapshotState, file_path: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Dead exports
    diagnostics.extend(dead_export_diagnostics(snapshot, file_path).await);

    // Cycles
    diagnostics.extend(cycle_diagnostics(snapshot, file_path).await);

    // Twins (returns empty for now - needs cross-file analysis)
    diagnostics.extend(twin_diagnostics(snapshot, file_path).await);

    diagnostics
}
