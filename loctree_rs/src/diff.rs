//! Snapshot comparison engine for temporal analysis
//!
//! This module compares loctree snapshots between different commits,
//! providing semantic analysis of how the codebase structure changed.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::git::{ChangeStatus, ChangedFile, CommitInfo};
use crate::snapshot::Snapshot;

/// Result of comparing two snapshots
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotDiff {
    /// Information about the source commit
    pub from_commit: Option<CommitInfo>,
    /// Information about the target commit (None = working tree)
    pub to_commit: Option<CommitInfo>,
    /// Files that changed between snapshots
    pub files: FilesDiff,
    /// Changes in the import/export graph
    pub graph: GraphDiff,
    /// Changes in exported symbols
    pub exports: ExportsDiff,
    /// Impact analysis
    pub impact: ImpactAnalysis,
}

/// Diff of files between snapshots
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FilesDiff {
    /// Files added in the new snapshot
    pub added: Vec<PathBuf>,
    /// Files removed from the old snapshot
    pub removed: Vec<PathBuf>,
    /// Files modified between snapshots
    pub modified: Vec<PathBuf>,
    /// Files renamed (old_path -> new_path)
    pub renamed: Vec<(PathBuf, PathBuf)>,
}

impl FilesDiff {
    pub fn from_changed_files(changes: &[ChangedFile]) -> Self {
        let mut diff = FilesDiff::default();

        for change in changes {
            match change.status {
                ChangeStatus::Added => {
                    if let Some(path) = &change.new_path {
                        diff.added.push(path.clone());
                    }
                }
                ChangeStatus::Deleted => {
                    if let Some(path) = &change.old_path {
                        diff.removed.push(path.clone());
                    }
                }
                ChangeStatus::Modified => {
                    if let Some(path) = &change.new_path {
                        diff.modified.push(path.clone());
                    }
                }
                ChangeStatus::Renamed | ChangeStatus::Copied => {
                    if let (Some(old), Some(new)) = (&change.old_path, &change.new_path) {
                        diff.renamed.push((old.clone(), new.clone()));
                    }
                }
            }
        }

        diff
    }

    /// Total number of changes
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len() + self.renamed.len()
    }
}

/// Edge in the import graph
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source file (importer)
    pub from: PathBuf,
    /// Target file (imported)
    pub to: PathBuf,
    /// Imported symbols (if known)
    pub symbols: Vec<String>,
}

/// Diff of the import graph
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphDiff {
    /// New import edges added
    pub edges_added: Vec<GraphEdge>,
    /// Import edges removed
    pub edges_removed: Vec<GraphEdge>,
}

/// An exported symbol
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExportedSymbol {
    /// File containing the export
    pub file: PathBuf,
    /// Symbol name
    pub name: String,
    /// Symbol kind (function, class, const, etc.)
    pub kind: String,
}

/// Diff of exports
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExportsDiff {
    /// New exports added
    pub added: Vec<ExportedSymbol>,
    /// Exports removed
    pub removed: Vec<ExportedSymbol>,
}

/// Impact analysis of the changes
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    /// Number of files affected by changes
    pub affected_files: usize,
    /// Files that consume changed exports
    pub affected_consumers: Vec<PathBuf>,
    /// Risk score (0.0 - 1.0)
    pub risk_score: f64,
    /// Summary of the impact
    pub summary: String,
}

impl SnapshotDiff {
    /// Compare two snapshots and produce a diff
    pub fn compare(
        from_snapshot: &Snapshot,
        to_snapshot: &Snapshot,
        from_commit: Option<CommitInfo>,
        to_commit: Option<CommitInfo>,
        changed_files: &[ChangedFile],
    ) -> Self {
        let files = FilesDiff::from_changed_files(changed_files);
        let graph = Self::compare_graphs(from_snapshot, to_snapshot);
        let exports = Self::compare_exports(from_snapshot, to_snapshot);
        let impact = Self::analyze_impact(&files, &graph, &exports, to_snapshot);

        Self {
            from_commit,
            to_commit,
            files,
            graph,
            exports,
            impact,
        }
    }

    /// Compare import graphs between snapshots
    fn compare_graphs(from: &Snapshot, to: &Snapshot) -> GraphDiff {
        let mut diff = GraphDiff::default();

        // Build edge sets for comparison
        let from_edges = Self::extract_edges(from);
        let to_edges = Self::extract_edges(to);

        // Find added edges
        for edge in &to_edges {
            if !from_edges.contains(edge) {
                diff.edges_added.push(edge.clone());
            }
        }

        // Find removed edges
        for edge in &from_edges {
            if !to_edges.contains(edge) {
                diff.edges_removed.push(edge.clone());
            }
        }

        diff
    }

    /// Extract edges from snapshot
    fn extract_edges(snapshot: &Snapshot) -> HashSet<GraphEdge> {
        let mut edges = HashSet::new();

        // Use snapshot.edges which contains GraphEdge structs
        for edge in &snapshot.edges {
            // Parse symbols from label (label format: "symbol1, symbol2" or empty)
            let symbols: Vec<String> = if edge.label.is_empty() {
                Vec::new()
            } else {
                edge.label.split(", ").map(|s| s.to_string()).collect()
            };

            edges.insert(GraphEdge {
                from: PathBuf::from(&edge.from),
                to: PathBuf::from(&edge.to),
                symbols,
            });
        }

        edges
    }

    /// Compare exports between snapshots
    fn compare_exports(from: &Snapshot, to: &Snapshot) -> ExportsDiff {
        let mut diff = ExportsDiff::default();

        let from_exports = Self::extract_exports(from);
        let to_exports = Self::extract_exports(to);

        for export in &to_exports {
            if !from_exports.contains(export) {
                diff.added.push(export.clone());
            }
        }

        for export in &from_exports {
            if !to_exports.contains(export) {
                diff.removed.push(export.clone());
            }
        }

        diff
    }

    /// Extract exports from snapshot
    fn extract_exports(snapshot: &Snapshot) -> HashSet<ExportedSymbol> {
        let mut exports = HashSet::new();

        // Use snapshot.files which is Vec<FileAnalysis>
        for file_info in &snapshot.files {
            let file_path = PathBuf::from(&file_info.path);

            for export in &file_info.exports {
                exports.insert(ExportedSymbol {
                    file: file_path.clone(),
                    name: export.name.clone(),
                    kind: format!("{:?}", export.kind),
                });
            }
        }

        exports
    }

    /// Analyze the impact of changes
    fn analyze_impact(
        files: &FilesDiff,
        graph: &GraphDiff,
        exports: &ExportsDiff,
        to_snapshot: &Snapshot,
    ) -> ImpactAnalysis {
        let affected_files = files.total_changes();

        // Find consumers of changed files
        let changed_paths: HashSet<String> = files
            .modified
            .iter()
            .chain(files.removed.iter())
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let mut affected_consumers = Vec::new();
        for file_info in &to_snapshot.files {
            for import in &file_info.imports {
                if let Some(resolved) = &import.resolved_path {
                    if changed_paths.contains(resolved) {
                        affected_consumers.push(PathBuf::from(&file_info.path));
                        break;
                    }
                }
            }
        }

        // Calculate risk score
        let risk_score = Self::calculate_risk_score(files, graph, exports);

        // Generate summary
        let summary = Self::generate_summary(files, graph, exports, &affected_consumers);

        ImpactAnalysis {
            affected_files,
            affected_consumers,
            risk_score,
            summary,
        }
    }

    /// Calculate risk score (0.0 - 1.0)
    fn calculate_risk_score(files: &FilesDiff, graph: &GraphDiff, exports: &ExportsDiff) -> f64 {
        let mut score = 0.0;

        // File changes
        score += files.removed.len() as f64 * 0.1;
        score += files.modified.len() as f64 * 0.05;

        // Graph changes
        score += graph.edges_removed.len() as f64 * 0.05;

        // Export changes
        score += exports.removed.len() as f64 * 0.15;

        // Clamp to 0.0 - 1.0
        score.min(1.0)
    }

    /// Generate human-readable summary
    fn generate_summary(
        files: &FilesDiff,
        graph: &GraphDiff,
        exports: &ExportsDiff,
        affected_consumers: &[PathBuf],
    ) -> String {
        let mut parts = Vec::new();

        if !files.added.is_empty() {
            parts.push(format!("{} files added", files.added.len()));
        }
        if !files.removed.is_empty() {
            parts.push(format!("{} files removed", files.removed.len()));
        }
        if !files.modified.is_empty() {
            parts.push(format!("{} files modified", files.modified.len()));
        }
        if !graph.edges_added.is_empty() {
            parts.push(format!("{} imports added", graph.edges_added.len()));
        }
        if !graph.edges_removed.is_empty() {
            parts.push(format!("{} imports removed", graph.edges_removed.len()));
        }
        if !exports.removed.is_empty() {
            parts.push(format!("{} exports removed", exports.removed.len()));
        }
        if !affected_consumers.is_empty() {
            parts.push(format!("{} consumers affected", affected_consumers.len()));
        }

        if parts.is_empty() {
            "No significant changes".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_files_diff_from_changed_files() {
        let changes = vec![
            ChangedFile {
                old_path: None,
                new_path: Some(PathBuf::from("new.ts")),
                status: ChangeStatus::Added,
            },
            ChangedFile {
                old_path: Some(PathBuf::from("old.ts")),
                new_path: None,
                status: ChangeStatus::Deleted,
            },
            ChangedFile {
                old_path: Some(PathBuf::from("mod.ts")),
                new_path: Some(PathBuf::from("mod.ts")),
                status: ChangeStatus::Modified,
            },
        ];

        let diff = FilesDiff::from_changed_files(&changes);

        assert_eq!(diff.added, vec![PathBuf::from("new.ts")]);
        assert_eq!(diff.removed, vec![PathBuf::from("old.ts")]);
        assert_eq!(diff.modified, vec![PathBuf::from("mod.ts")]);
        assert_eq!(diff.total_changes(), 3);
    }

    #[test]
    fn test_risk_score_clamped() {
        let files = FilesDiff {
            removed: (0..100)
                .map(|i| PathBuf::from(format!("file{}.ts", i)))
                .collect(),
            ..Default::default()
        };
        let graph = GraphDiff::default();
        let exports = ExportsDiff::default();

        let score = SnapshotDiff::calculate_risk_score(&files, &graph, &exports);
        assert!(score <= 1.0);
    }
}
