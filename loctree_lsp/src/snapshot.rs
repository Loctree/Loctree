//! Snapshot loading and watching for loctree LSP
//!
//! Loads `.loctree/snapshot.json` and watches for changes.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use std::path::{Path, PathBuf};
use std::sync::Arc;

use loctree::snapshot::Snapshot;
use tokio::sync::RwLock;

/// Snapshot state wrapper for async access
#[derive(Clone)]
pub struct SnapshotState {
    inner: Arc<RwLock<Option<LoadedSnapshot>>>,
}

/// Loaded snapshot with metadata
pub struct LoadedSnapshot {
    /// The parsed snapshot
    pub snapshot: Snapshot,
    /// Path to the snapshot file
    pub path: PathBuf,
}

impl SnapshotState {
    /// Create a new empty snapshot state
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    /// Load snapshot from workspace root
    pub async fn load(&self, workspace_root: &Path) -> Result<(), SnapshotError> {
        let snapshot_path = workspace_root.join(".loctree").join("snapshot.json");

        if !snapshot_path.exists() {
            return Err(SnapshotError::NotFound(snapshot_path));
        }

        let content = tokio::fs::read_to_string(&snapshot_path)
            .await
            .map_err(|e| SnapshotError::ReadError(snapshot_path.clone(), e.to_string()))?;

        let snapshot: Snapshot = serde_json::from_str(&content)
            .map_err(|e| SnapshotError::ParseError(snapshot_path.clone(), e.to_string()))?;

        let loaded = LoadedSnapshot {
            snapshot,
            path: snapshot_path,
        };

        let mut guard = self.inner.write().await;
        *guard = Some(loaded);

        Ok(())
    }

    /// Reload snapshot from disk
    pub async fn reload(&self) -> Result<(), SnapshotError> {
        let guard = self.inner.read().await;
        if let Some(loaded) = guard.as_ref() {
            let path = loaded.path.clone();
            drop(guard);

            let workspace_root = path
                .parent()
                .and_then(|p| p.parent())
                .ok_or_else(|| SnapshotError::InvalidPath(path.clone()))?;

            self.load(workspace_root).await
        } else {
            Err(SnapshotError::NotLoaded)
        }
    }

    /// Get read access to the snapshot
    pub async fn get(&self) -> Option<tokio::sync::RwLockReadGuard<'_, Option<LoadedSnapshot>>> {
        let guard = self.inner.read().await;
        if guard.is_some() { Some(guard) } else { None }
    }

    /// Check if snapshot is loaded
    pub async fn is_loaded(&self) -> bool {
        self.inner.read().await.is_some()
    }

    /// Get dead exports for a specific file
    ///
    /// NOTE: This requires findings.json which contains pre-computed dead exports.
    /// The snapshot.json alone doesn't have import counts - edges need to be analyzed.
    pub async fn dead_exports_for_file(&self, file_path: &str) -> Vec<DeadExportInfo> {
        let guard = self.inner.read().await;
        if let Some(loaded) = guard.as_ref() {
            // Count imports for each export by analyzing edges
            // An export is "dead" if no edge points to it
            use std::collections::HashSet;

            // Collect all imported files
            let imported_files: HashSet<&str> = loaded
                .snapshot
                .edges
                .iter()
                .map(|e| e.to.as_str())
                .collect();

            // Find files that have exports but no incoming edges
            loaded
                .snapshot
                .files
                .iter()
                .filter(|f| f.path.ends_with(file_path) || file_path.ends_with(&f.path))
                .filter(|f| !imported_files.contains(f.path.as_str()))
                .flat_map(|f| {
                    // This file has no imports - all exports are potentially dead
                    f.exports.iter().map(|e| DeadExportInfo {
                        symbol: e.name.clone(),
                        line: e.line.unwrap_or(1),
                        confidence: if f.path.contains("test") || f.path.contains("spec") {
                            "low".to_string()
                        } else {
                            "normal".to_string() // Not "high" since we can't verify symbol-level
                        },
                        reason: format!("File has no importers, export '{}' may be unused", e.name),
                    })
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get cycles involving a specific file
    pub async fn cycles_for_file(&self, file_path: &str) -> Vec<CycleInfo> {
        let guard = self.inner.read().await;
        if let Some(loaded) = guard.as_ref() {
            // Build cycle detection from edges
            let edges = &loaded.snapshot.edges;

            // Simple cycle detection: find any cycles that include this file
            let mut cycles = Vec::new();
            for edge in edges {
                if edge.from.ends_with(file_path) || file_path.ends_with(&edge.from) {
                    // Check if there's a reverse edge (simple bidirectional cycle)
                    for other_edge in edges {
                        if other_edge.from == edge.to && other_edge.to == edge.from {
                            cycles.push(CycleInfo {
                                files: vec![edge.from.clone(), edge.to.clone()],
                                cycle_type: "bidirectional".to_string(),
                            });
                            break;
                        }
                    }
                }
            }
            cycles
        } else {
            Vec::new()
        }
    }

    /// Find where a symbol is defined.
    ///
    /// This looks at:
    /// 1. Edges: if current file imports from another file, check if symbol matches edge label
    /// 2. Export index: if symbol name exists in export_index, find which file exports it
    /// 3. File exports: look through files to find exports matching the symbol name
    ///
    /// # Arguments
    /// * `current_file` - The file where the cursor is (for context on imports)
    /// * `symbol` - The symbol name to find definition for
    ///
    /// # Returns
    /// * `Some(DefinitionLocation)` if definition found
    /// * `None` if no definition found
    pub async fn find_definition(
        &self,
        current_file: &str,
        symbol: &str,
    ) -> Option<DefinitionLocation> {
        let guard = self.inner.read().await;
        let loaded = guard.as_ref()?;
        let snapshot = &loaded.snapshot;

        // Normalize file path for matching (strip leading ./ or /)
        let normalized_current = normalize_path(current_file);

        // Strategy 1: Look at edges from current file and match symbol with edge label
        for edge in &snapshot.edges {
            let edge_from = normalize_path(&edge.from);
            if paths_match(&edge_from, &normalized_current) {
                // Check if edge label matches symbol (edge.label contains the imported symbol)
                if edge.label == symbol || edge.label.contains(symbol) {
                    // Found an edge - now find the export line in target file
                    let target_file = &edge.to;
                    if let Some(line) = find_export_line_in_snapshot(snapshot, target_file, symbol)
                    {
                        return Some(DefinitionLocation {
                            file: target_file.clone(),
                            line,
                        });
                    }
                    // Even without exact line, return the target file at line 1
                    return Some(DefinitionLocation {
                        file: target_file.clone(),
                        line: 1,
                    });
                }
            }
        }

        // Strategy 2: Use export_index to find symbol
        if let Some(files) = snapshot.export_index.get(symbol)
            && let Some(file_path) = files.first()
        {
            if let Some(line) = find_export_line_in_snapshot(snapshot, file_path, symbol) {
                return Some(DefinitionLocation {
                    file: file_path.clone(),
                    line,
                });
            }
            return Some(DefinitionLocation {
                file: file_path.clone(),
                line: 1,
            });
        }

        // Strategy 3: Search all files' exports for the symbol
        for file in &snapshot.files {
            for export in &file.exports {
                if export.name == symbol {
                    return Some(DefinitionLocation {
                        file: file.path.clone(),
                        line: export.line.unwrap_or(1),
                    });
                }
            }
        }

        None
    }

    /// Find all references to a symbol exported from a file
    ///
    /// Returns a list of ReferenceInfo for all files that import the symbol.
    ///
    /// # Arguments
    /// * `file_path` - The file containing the export (can be relative or absolute)
    /// * `symbol` - The symbol name to find references for (optional, if None finds all importers)
    pub async fn find_references(
        &self,
        file_path: &str,
        symbol: Option<&str>,
    ) -> Vec<ReferenceInfo> {
        let guard = self.inner.read().await;
        let Some(loaded) = guard.as_ref() else {
            return Vec::new();
        };

        let mut references = Vec::new();

        // Normalize the file path for comparison
        let normalized_target = normalize_path(file_path);

        // Find all edges where this file is the "to" (imported) target
        for edge in &loaded.snapshot.edges {
            let edge_to_normalized = normalize_path(&edge.to);

            // Check if this edge points to our target file
            if paths_match(&normalized_target, &edge_to_normalized) {
                // If a symbol is specified, check if the edge label matches
                if let Some(sym) = symbol {
                    let label_contains_symbol = edge
                        .label
                        .split(',')
                        .map(|s: &str| s.trim())
                        .any(|s| s == sym || s == "*");

                    if !label_contains_symbol {
                        continue;
                    }
                }

                // Try to find the specific import line in the importing file
                let import_line = find_import_line(&loaded.snapshot.files, &edge.from, &edge.to);

                references.push(ReferenceInfo {
                    file: edge.from.clone(),
                    line: import_line.unwrap_or(0),
                });
            }
        }

        // Deduplicate references
        references.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
        references.dedup_by(|a, b| a.file == b.file && a.line == b.line);

        references
    }

    /// Find the export location for a symbol in a file
    ///
    /// Returns (file_path, line) if found
    pub async fn find_export_location(
        &self,
        file_path: &str,
        symbol: &str,
    ) -> Option<(String, usize)> {
        let guard = self.inner.read().await;
        let loaded = guard.as_ref()?;

        let normalized_target = normalize_path(file_path);

        // Find the file in the snapshot
        for file in &loaded.snapshot.files {
            let file_normalized = normalize_path(&file.path);
            if paths_match(&normalized_target, &file_normalized) {
                // Find the export
                for export in &file.exports {
                    if export.name == symbol {
                        return Some((file.path.clone(), export.line.unwrap_or(1)));
                    }
                }
            }
        }

        None
    }
}

/// Reference information for a symbol
#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    /// File path where the reference occurs
    pub file: String,
    /// Line number (0 if unknown)
    pub line: usize,
}

/// Definition lookup result
#[derive(Debug, Clone)]
pub struct DefinitionLocation {
    /// Target file path (relative to project root)
    pub file: String,
    /// 1-based line number where symbol is defined
    pub line: usize,
}

/// Find the line number where a file imports another file
fn find_import_line(
    files: &[loctree::types::FileAnalysis],
    importer_path: &str,
    imported_path: &str,
) -> Option<usize> {
    let importer_normalized = normalize_path(importer_path);
    let imported_normalized = normalize_path(imported_path);

    // Find the importer file
    for file in files {
        let file_normalized = normalize_path(&file.path);
        if paths_match(&importer_normalized, &file_normalized) {
            // Find the import statement
            for import in &file.imports {
                if let Some(ref resolved) = import.resolved_path {
                    let resolved_normalized = normalize_path(resolved);
                    if paths_match(&imported_normalized, &resolved_normalized) {
                        // Import entries don't have line numbers directly
                        // Return None to use default line 0
                        return None;
                    }
                }
                // Also check source_raw for path matching
                let source_normalized = normalize_path(&import.source);
                if paths_match(&imported_normalized, &source_normalized) {
                    return None;
                }
            }
        }
    }

    None
}

/// Find the line number where a symbol is exported in a given file
fn find_export_line_in_snapshot(
    snapshot: &Snapshot,
    file_path: &str,
    symbol: &str,
) -> Option<usize> {
    let normalized_target = normalize_path(file_path);

    for file in &snapshot.files {
        let normalized_file = normalize_path(&file.path);
        if paths_match(&normalized_file, &normalized_target) {
            for export in &file.exports {
                if export.name == symbol {
                    return export.line;
                }
            }
        }
    }
    None
}

/// Normalize a file path by stripping leading ./ or /
fn normalize_path(path: &str) -> String {
    path.trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

/// Check if two normalized paths match (handles suffix matching for relative paths)
fn paths_match(a: &str, b: &str) -> bool {
    a == b || a.ends_with(b) || b.ends_with(a)
}

impl Default for SnapshotState {
    fn default() -> Self {
        Self::new()
    }
}

/// Dead export info for diagnostics
#[derive(Debug, Clone)]
pub struct DeadExportInfo {
    pub symbol: String,
    pub line: usize,
    pub confidence: String,
    pub reason: String,
}

/// Cycle info for diagnostics
#[derive(Debug, Clone)]
pub struct CycleInfo {
    pub files: Vec<String>,
    pub cycle_type: String,
}

/// Snapshot loading errors
#[derive(Debug)]
pub enum SnapshotError {
    /// Snapshot file not found
    NotFound(PathBuf),
    /// Error reading snapshot file
    ReadError(PathBuf, String),
    /// Error parsing snapshot JSON
    ParseError(PathBuf, String),
    /// Invalid path structure
    InvalidPath(PathBuf),
    /// Snapshot not yet loaded
    NotLoaded,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::NotFound(path) => {
                write!(
                    f,
                    "Snapshot not found at {:?}. Run `loct` to scan your project first.",
                    path
                )
            }
            SnapshotError::ReadError(path, e) => {
                write!(f, "Error reading snapshot {:?}: {}", path, e)
            }
            SnapshotError::ParseError(path, e) => {
                write!(f, "Error parsing snapshot {:?}: {}", path, e)
            }
            SnapshotError::InvalidPath(path) => {
                write!(f, "Invalid snapshot path: {:?}", path)
            }
            SnapshotError::NotLoaded => {
                write!(f, "Snapshot not loaded")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}
