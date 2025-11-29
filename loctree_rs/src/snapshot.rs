//! Snapshot module for persisting the code graph to disk.
//!
//! This module implements the "scan once, slice many" philosophy:
//! - `loctree init` or bare `loctree` scans the project and saves a snapshot
//! - Subsequent queries load the snapshot for instant context slicing

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::args::ParsedArgs;
use crate::types::FileAnalysis;

/// Current schema version for snapshot format
pub const SNAPSHOT_SCHEMA_VERSION: &str = "0.5.0-rc";

/// Default snapshot directory name
pub const SNAPSHOT_DIR: &str = ".loctree";

/// Default snapshot file name
pub const SNAPSHOT_FILE: &str = "snapshot.json";

/// Metadata about the snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Schema version for compatibility checking
    #[serde(default)]
    pub schema_version: String,
    /// Timestamp when snapshot was generated (ISO 8601)
    #[serde(default)]
    pub generated_at: String,
    /// Root path(s) that were scanned
    #[serde(default)]
    pub roots: Vec<String>,
    /// Detected languages in the project
    #[serde(default)]
    pub languages: HashSet<String>,
    /// Total number of files scanned
    #[serde(default)]
    pub file_count: usize,
    /// Total lines of code
    #[serde(default)]
    pub total_loc: usize,
    /// Scan duration in milliseconds
    #[serde(default)]
    pub scan_duration_ms: u64,
    /// Resolver configuration (tsconfig paths, etc.)
    #[serde(default)]
    pub resolver_config: Option<ResolverConfig>,
}

/// Configuration for path resolution (aliases, etc.)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ResolverConfig {
    /// TypeScript/JavaScript path aliases from tsconfig.json
    pub ts_paths: HashMap<String, Vec<String>>,
    /// Base URL for TypeScript resolution
    pub ts_base_url: Option<String>,
    /// Python root paths
    pub py_roots: Vec<String>,
    /// Rust crate roots
    pub rust_crate_roots: Vec<String>,
}

/// Graph edge representing an import relationship
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source file path (importer)
    pub from: String,
    /// Target file path (imported)
    pub to: String,
    /// Edge label (import type, symbol name, etc.)
    pub label: String,
}

/// Command bridge mapping (FE invoke -> BE handler)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandBridge {
    /// Command name
    pub name: String,
    /// Frontend call locations (file, line)
    pub frontend_calls: Vec<(String, usize)>,
    /// Backend handler location (file, line)
    pub backend_handler: Option<(String, usize)>,
    /// Whether the command has a handler
    pub has_handler: bool,
    /// Whether the command is called from frontend
    pub is_called: bool,
}

/// Event bridge mapping (emit -> listen)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventBridge {
    /// Event name
    pub name: String,
    /// Emit locations (file, line, kind)
    pub emits: Vec<(String, usize, String)>,
    /// Listen locations (file, line)
    pub listens: Vec<(String, usize)>,
}

/// Export index entry (used by VS2 slice module)
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportEntry {
    /// Symbol name
    pub name: String,
    /// Files that export this symbol
    pub files: Vec<String>,
}

/// The complete snapshot of the code graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    /// Snapshot metadata
    pub metadata: SnapshotMetadata,
    /// All file analyses (nodes in the graph)
    #[serde(default)]
    pub files: Vec<FileAnalysis>,
    /// Graph edges (import relationships)
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    /// Export index (symbol -> files mapping)
    #[serde(default)]
    pub export_index: HashMap<String, Vec<String>>,
    /// Command bridges (FE <-> BE)
    #[serde(default)]
    pub command_bridges: Vec<CommandBridge>,
    /// Event bridges (emit <-> listen)
    #[serde(default)]
    pub event_bridges: Vec<EventBridge>,
    /// Detected barrel files
    #[serde(default)]
    pub barrels: Vec<BarrelFile>,
}

/// Information about a barrel file (index.ts re-exporting)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarrelFile {
    /// Path to the barrel file
    pub path: String,
    /// Module ID (normalized path)
    pub module_id: String,
    /// Number of re-exports
    pub reexport_count: usize,
    /// Target files being re-exported
    pub targets: Vec<String>,
}

impl Snapshot {
    /// Create a new empty snapshot
    pub fn new(roots: Vec<String>) -> Self {
        let now = time::OffsetDateTime::now_utc();
        let generated_at = now
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            metadata: SnapshotMetadata {
                schema_version: SNAPSHOT_SCHEMA_VERSION.to_string(),
                generated_at,
                roots,
                languages: HashSet::new(),
                file_count: 0,
                total_loc: 0,
                scan_duration_ms: 0,
                resolver_config: None,
            },
            files: Vec::new(),
            edges: Vec::new(),
            export_index: HashMap::new(),
            command_bridges: Vec::new(),
            event_bridges: Vec::new(),
            barrels: Vec::new(),
        }
    }

    /// Get the snapshot file path for a given root
    pub fn snapshot_path(root: &Path) -> PathBuf {
        root.join(SNAPSHOT_DIR).join(SNAPSHOT_FILE)
    }

    /// Check if a snapshot exists for the given root (used by VS2 slice module)
    #[allow(dead_code)]
    pub fn exists(root: &Path) -> bool {
        Self::snapshot_path(root).exists()
    }

    /// Search upward for .loctree directory (like git finds .git/)
    /// Returns the directory containing .loctree/, or None if not found.
    pub fn find_loctree_root(start: &Path) -> Option<PathBuf> {
        let mut current = start.canonicalize().ok()?;
        loop {
            if current.join(SNAPSHOT_DIR).exists() {
                return Some(current);
            }
            match current.parent() {
                Some(parent) if parent != current => current = parent.to_path_buf(),
                _ => return None,
            }
        }
    }

    /// Save snapshot to disk
    pub fn save(&self, root: &Path) -> io::Result<()> {
        let snapshot_dir = root.join(SNAPSHOT_DIR);
        fs::create_dir_all(&snapshot_dir)?;

        let snapshot_path = Self::snapshot_path(root);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::write(&snapshot_path, json)?;

        Ok(())
    }

    /// Load snapshot from disk (used by VS2 slice module)
    #[allow(dead_code)]
    pub fn load(root: &Path) -> io::Result<Self> {
        let snapshot_path = Self::snapshot_path(root);

        if !snapshot_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "No snapshot found. Run `loctree` first to create one.\nExpected: {}",
                    snapshot_path.display()
                ),
            ));
        }

        // nosemgrep: rust.actix.path-traversal.tainted-path.tainted-path -- SAFETY: snapshot_path is derived from root/.loctree/snapshot.json where root is validated as existing directory by caller; no user-controlled path segments are interpolated
        let content = fs::read_to_string(&snapshot_path)?;
        let snapshot: Self = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Check schema version compatibility
        if snapshot.metadata.schema_version != SNAPSHOT_SCHEMA_VERSION {
            eprintln!(
                "[loctree][warn] Snapshot schema version mismatch: found {}, expected {}. Consider re-running `loctree`.",
                snapshot.metadata.schema_version, SNAPSHOT_SCHEMA_VERSION
            );
        }

        Ok(snapshot)
    }

    /// Get map of file path -> FileAnalysis for incremental reuse
    pub fn cached_analyses(&self) -> HashMap<String, FileAnalysis> {
        self.files
            .iter()
            .map(|f| (f.path.clone(), f.clone()))
            .collect()
    }

    /// Update metadata after scan
    pub fn finalize_metadata(&mut self, scan_duration_ms: u64) {
        self.metadata.file_count = self.files.len();
        self.metadata.total_loc = self.files.iter().map(|f| f.loc).sum();
        self.metadata.scan_duration_ms = scan_duration_ms;

        // Collect languages from files
        for file in &self.files {
            if !file.language.is_empty() {
                self.metadata.languages.insert(file.language.clone());
            }
        }
    }

    /// Print summary of the snapshot
    pub fn print_summary(&self) {
        println!(
            "Scanned {} files in {:.2}s",
            self.metadata.file_count,
            self.metadata.scan_duration_ms as f64 / 1000.0
        );
        println!("Graph saved to ./{}/{}", SNAPSHOT_DIR, SNAPSHOT_FILE);

        let languages: Vec<_> = self.metadata.languages.iter().collect();
        if !languages.is_empty() {
            println!(
                "Languages: {}",
                languages
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        let handler_count = self
            .command_bridges
            .iter()
            .filter(|b| b.has_handler)
            .count();
        let missing_handlers = self
            .command_bridges
            .iter()
            .filter(|b| !b.has_handler && b.is_called)
            .count();
        let unused_handlers = self
            .command_bridges
            .iter()
            .filter(|b| b.has_handler && !b.is_called)
            .count();

        if handler_count > 0 || missing_handlers > 0 {
            print!("Commands: {} handlers", handler_count);
            if missing_handlers > 0 {
                print!(", {} missing", missing_handlers);
            }
            if unused_handlers > 0 {
                print!(", {} unused", unused_handlers);
            }
            println!();
        }

        let event_count = self.event_bridges.len();
        if event_count > 0 {
            println!("Events: {} tracked", event_count);
        }

        // Check for cycles or issues
        let barrel_count = self.barrels.len();
        if barrel_count > 0 {
            println!("Barrels: {} detected", barrel_count);
        }

        println!("Status: OK");
        println!();
        println!("Next steps:");
        println!("  loctree . -A --json          # Full analysis with JSON output");
        println!("  loctree . -A --preset-tauri  # Tauri FE↔BE coverage analysis");
    }
}

/// Run the init command: scan the project and save snapshot
pub fn run_init(root_list: &[PathBuf], parsed: &ParsedArgs) -> io::Result<()> {
    use crate::analyzer::coverage::compute_command_gaps;
    use crate::analyzer::root_scan::{ScanConfig, scan_roots};
    use crate::analyzer::runner::default_analyzer_exts;
    use crate::analyzer::scan::{opt_globset, python_stdlib};

    let start_time = Instant::now();

    // Snapshot always saves to CWD (one snapshot per repo)
    let snapshot_root = std::env::current_dir()?;

    // Validate at least one root was specified
    if root_list.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No root directory specified",
        ));
    }

    // Try to load existing snapshot for incremental scanning
    let cached_analyses: Option<HashMap<String, FileAnalysis>> =
        if !parsed.full_scan && Snapshot::exists(&snapshot_root) {
            match Snapshot::load(&snapshot_root) {
                Ok(old_snapshot) => {
                    if parsed.verbose {
                        eprintln!(
                            "[loctree][incremental] Loaded existing snapshot ({} files cached)",
                            old_snapshot.files.len()
                        );
                    }
                    Some(old_snapshot.cached_analyses())
                }
                Err(e) => {
                    if parsed.verbose {
                        eprintln!(
                            "[loctree][warn] Could not load snapshot for incremental: {}",
                            e
                        );
                    }
                    None
                }
            }
        } else {
            None
        };

    // Log scan mode for clarity (especially in CI)
    let scan_mode = if parsed.full_scan {
        "full (--full-scan)"
    } else if cached_analyses.is_some() {
        "incremental (mtime-based)"
    } else {
        "fresh (no existing snapshot)"
    };
    eprintln!("[loctree] Scan mode: {}", scan_mode);

    // Try to load existing snapshot for incremental scanning
    let cached_analyses: Option<HashMap<String, FileAnalysis>> =
        if !parsed.full_scan && Snapshot::exists(&snapshot_root) {
            match Snapshot::load(&snapshot_root) {
                Ok(old_snapshot) => {
                    if parsed.verbose {
                        eprintln!(
                            "[loctree][incremental] Loaded existing snapshot ({} files cached)",
                            old_snapshot.files.len()
                        );
                    }
                    Some(old_snapshot.cached_analyses())
                }
                Err(e) => {
                    if parsed.verbose {
                        eprintln!(
                            "[loctree][warn] Could not load snapshot for incremental: {}",
                            e
                        );
                    }
                    None
                }
            }
        } else {
            None
        };

    // Log scan mode for clarity (especially in CI)
    let scan_mode = if parsed.full_scan {
        "full (--full-scan)"
    } else if cached_analyses.is_some() {
        "incremental (mtime-based)"
    } else {
        "fresh (no existing snapshot)"
    };
    eprintln!("[loctree] Scan mode: {}", scan_mode);
    // Prepare scan configuration (reusing existing infrastructure)
    let py_stdlib = python_stdlib();
    let focus_set = opt_globset(&parsed.focus_patterns);
    let exclude_set = opt_globset(&parsed.exclude_report_patterns);

    let base_extensions = parsed
        .extensions
        .clone()
        .or_else(|| Some(default_analyzer_exts()));

    let scan_config = ScanConfig {
        roots: root_list,
        parsed,
        extensions: base_extensions,
        focus_set: &focus_set,
        exclude_set: &exclude_set,
        ignore_exact: HashSet::new(),
        ignore_prefixes: Vec::new(),
        py_stdlib: &py_stdlib,
        cached_analyses: cached_analyses.as_ref(),
        collect_edges: true, // Always collect edges for snapshot (needed by slice)
    };

    // Perform the scan
    let scan_results = scan_roots(scan_config)?;

    // Build the snapshot from scan results
    let mut snapshot = Snapshot::new(root_list.iter().map(|p| p.display().to_string()).collect());

    // Populate files from all contexts
    for ctx in &scan_results.contexts {
        snapshot.files.extend(ctx.analyses.clone());

        // Add graph edges
        for (from, to, label) in &ctx.graph_edges {
            snapshot.edges.push(GraphEdge {
                from: from.clone(),
                to: to.clone(),
                label: label.clone(),
            });
        }

        // Add export index
        for (name, files) in &ctx.export_index {
            snapshot
                .export_index
                .entry(name.clone())
                .or_default()
                .extend(files.clone());
        }

        // Add barrels
        for barrel in &ctx.barrels {
            snapshot.barrels.push(BarrelFile {
                path: barrel.path.clone(),
                module_id: barrel.module_id.clone(),
                reexport_count: barrel.reexport_count,
                targets: barrel.targets.clone(),
            });
        }

        // Collect languages
        for lang in &ctx.languages {
            snapshot.metadata.languages.insert(lang.clone());
        }
    }

    // Build command bridges from global command data
    let (_missing_handlers, _unused_handlers) = compute_command_gaps(
        &scan_results.global_fe_commands,
        &scan_results.global_be_commands,
        &focus_set,
        &exclude_set,
    );

    // Create command bridges
    let mut all_commands: HashSet<String> = HashSet::new();
    for name in scan_results.global_fe_commands.keys() {
        all_commands.insert(name.clone());
    }
    for name in scan_results.global_be_commands.keys() {
        all_commands.insert(name.clone());
    }

    for cmd_name in all_commands {
        let fe_calls: Vec<(String, usize)> = scan_results
            .global_fe_commands
            .get(&cmd_name)
            .map(|v| v.iter().map(|(f, l, _)| (f.clone(), *l)).collect())
            .unwrap_or_default();

        let be_handler: Option<(String, usize)> = scan_results
            .global_be_commands
            .get(&cmd_name)
            .and_then(|v| v.first())
            .map(|(f, l, _)| (f.clone(), *l));

        let has_handler = be_handler.is_some();
        let is_called = !fe_calls.is_empty();

        snapshot.command_bridges.push(CommandBridge {
            name: cmd_name,
            frontend_calls: fe_calls,
            backend_handler: be_handler,
            has_handler,
            is_called,
        });
    }

    // Build event bridges from file analyses
    let mut event_emits_map: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();
    let mut event_listens_map: HashMap<String, Vec<(String, usize)>> = HashMap::new();

    for file in &snapshot.files {
        for emit in &file.event_emits {
            event_emits_map.entry(emit.name.clone()).or_default().push((
                file.path.clone(),
                emit.line,
                emit.kind.clone(),
            ));
        }
        for listen in &file.event_listens {
            event_listens_map
                .entry(listen.name.clone())
                .or_default()
                .push((file.path.clone(), listen.line));
        }
    }

    let mut all_events: HashSet<String> = HashSet::new();
    all_events.extend(event_emits_map.keys().cloned());
    all_events.extend(event_listens_map.keys().cloned());

    for event_name in all_events {
        snapshot.event_bridges.push(EventBridge {
            name: event_name.clone(),
            emits: event_emits_map
                .get(&event_name)
                .cloned()
                .unwrap_or_default(),
            listens: event_listens_map
                .get(&event_name)
                .cloned()
                .unwrap_or_default(),
        });
    }

    // Finalize metadata
    let duration_ms = start_time.elapsed().as_millis() as u64;
    snapshot.finalize_metadata(duration_ms);

    // Save snapshot
    snapshot.save(&snapshot_root)?;

    // Print summary
    snapshot.print_summary();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_snapshot_save_load_roundtrip() {
        let tmp = TempDir::new().expect("failed to create temp dir for snapshot roundtrip test");
        let root = tmp.path();

        let mut snapshot = Snapshot::new(vec![root.display().to_string()]);
        snapshot.metadata.languages.insert("rust".to_string());
        snapshot.metadata.languages.insert("typescript".to_string());

        // Save
        snapshot
            .save(root)
            .expect("failed to save snapshot in roundtrip test");

        // Verify file exists
        assert!(Snapshot::exists(root));

        // Load
        let loaded = Snapshot::load(root).expect("failed to load snapshot in roundtrip test");

        assert_eq!(loaded.metadata.schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert!(loaded.metadata.languages.contains("rust"));
        assert!(loaded.metadata.languages.contains("typescript"));
    }

    #[test]
    fn test_snapshot_not_found() {
        let tmp = TempDir::new().expect("failed to create temp dir for not_found test");
        let result = Snapshot::load(tmp.path());
        assert!(result.is_err());
    }
}
