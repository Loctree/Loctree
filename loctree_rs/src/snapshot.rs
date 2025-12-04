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
use std::process::Command;
use std::time::Instant;

use crate::args::ParsedArgs;
use crate::types::{FileAnalysis, OutputMode};

/// Current schema version for snapshot format
pub const SNAPSHOT_SCHEMA_VERSION: &str = "0.5.0-rc";

/// Default snapshot directory name
pub const SNAPSHOT_DIR: &str = ".loctree";

/// Default snapshot file name
pub const SNAPSHOT_FILE: &str = "snapshot.json";

/// Git workspace context for artifact isolation.
///
/// Used to store snapshots per branch@commit (e.g., `.loctree/main@abc123/snapshot.json`).
#[derive(Clone, Debug)]
pub struct GitContext {
    /// Repository name (extracted from remote origin).
    pub repo: Option<String>,
    /// Current branch name.
    pub branch: Option<String>,
    /// Short commit hash.
    pub commit: Option<String>,
    /// Combined identifier: `branch@commit` (sanitized for filesystem).
    pub scan_id: Option<String>,
}

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
    /// Git repository name (extracted from remote origin)
    #[serde(default)]
    pub git_repo: Option<String>,
    /// Git branch name
    #[serde(default)]
    pub git_branch: Option<String>,
    /// Git commit hash (short)
    #[serde(default)]
    pub git_commit: Option<String>,
    /// Combined scan identifier (e.g., branch@sha) for artifact isolation
    #[serde(default)]
    pub git_scan_id: Option<String>,
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

        // Get git info from current directory
        let git_info = Self::current_git_context();

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
                git_repo: git_info.repo,
                git_branch: git_info.branch,
                git_commit: git_info.commit,
                git_scan_id: git_info.scan_id,
            },
            files: Vec::new(),
            edges: Vec::new(),
            export_index: HashMap::new(),
            command_bridges: Vec::new(),
            event_bridges: Vec::new(),
            barrels: Vec::new(),
        }
    }

    /// Get git repository info (repo name, branch, commit)
    fn get_git_info() -> (Option<String>, Option<String>, Option<String>) {
        use std::process::{Command, Stdio};

        // Get repo name from remote origin URL
        // Suppress stderr to avoid "not a git repository" spam in non-git dirs
        let repo = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .and_then(|url| {
                // Extract repo name from URL like git@github.com:user/repo.git
                // or https://github.com/user/repo.git
                let url = url.trim();
                url.rsplit('/')
                    .next()
                    .or_else(|| url.rsplit(':').next())
                    .map(|s| s.trim_end_matches(".git").to_string())
            });

        // Get current branch
        let branch = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        // Get short commit hash
        let commit = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        (repo, branch, commit)
    }

    /// Sanitise branch/commit for filesystem path segments
    fn sanitize_ref(value: &str) -> String {
        value.replace(['/', '\\', ' ', ':'], "_").trim().to_string()
    }

    /// Build the current git context (repo, branch, commit, scan_id)
    pub fn current_git_context() -> GitContext {
        let (repo, branch, commit) = Self::get_git_info();
        let scan_id = branch.as_ref().map(|b| {
            let mut base = Self::sanitize_ref(b);
            if let Some(c) = &commit {
                base = format!("{}@{}", base, Self::sanitize_ref(c));
            }
            base
        });

        GitContext {
            repo,
            branch,
            commit,
            scan_id,
        }
    }

    fn candidate_snapshot_paths(root: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(seg) = Self::current_git_context().scan_id {
            paths.push(root.join(SNAPSHOT_DIR).join(seg).join(SNAPSHOT_FILE));
        }
        paths.push(root.join(SNAPSHOT_DIR).join(SNAPSHOT_FILE));
        paths
    }

    /// Get the snapshot file path for a given root
    pub fn snapshot_path(root: &Path) -> PathBuf {
        // Prefer the branch@sha path; fall back to legacy path
        let paths = Self::candidate_snapshot_paths(root);
        paths
            .first()
            .cloned()
            .unwrap_or_else(|| root.join(SNAPSHOT_DIR).join(SNAPSHOT_FILE))
    }

    /// Directory where snapshot and artifacts should be stored for the current scan
    pub fn artifacts_dir(root: &Path) -> PathBuf {
        let path = Self::snapshot_path(root);
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| root.join(SNAPSHOT_DIR))
    }

    /// Check if a snapshot exists for the given root (used by VS2 slice module)
    #[allow(dead_code)]
    pub fn exists(root: &Path) -> bool {
        Self::candidate_snapshot_paths(root)
            .iter()
            .any(|p| p.exists())
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
        // If a snapshot already exists for the same branch/commit, skip rewriting.
        if let (Some(commit), Some(branch), Ok(existing)) = (
            self.metadata.git_commit.as_ref(),
            self.metadata.git_branch.as_ref(),
            Self::load(root),
        ) && existing.metadata.git_commit.as_ref() == Some(commit)
            && existing.metadata.git_branch.as_ref() == Some(branch)
        {
            let dirty = is_git_dirty(root).unwrap_or(false);
            if dirty {
                eprintln!(
                    "[loctree] snapshot for {}@{} exists; worktree dirty → commit changes to refresh snapshot",
                    branch, commit
                );
            } else {
                eprintln!(
                    "[loctree] snapshot for {}@{} already exists; skipping write (no changes detected)",
                    branch, commit
                );
            }
            return Ok(());
        }

        let snapshot_path = Self::snapshot_path(root);
        if let Some(dir) = snapshot_path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::write(&snapshot_path, json)?;

        Ok(())
    }

    /// Load snapshot from disk (used by VS2 slice module)
    #[allow(dead_code)]
    pub fn load(root: &Path) -> io::Result<Self> {
        let mut snapshot_path = None;
        for candidate in Self::candidate_snapshot_paths(root) {
            if candidate.exists() {
                snapshot_path = Some(candidate);
                break;
            }
        }

        let snapshot_path = match snapshot_path {
            Some(p) => p,
            None => {
                let primary = Self::snapshot_path(root);
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "No snapshot found. Run `loctree` first to create one.\nExpected: {}",
                        primary.display()
                    ),
                ));
            }
        };

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
    pub fn print_summary(&self, root: &Path) {
        println!(
            "Scanned {} files in {:.2}s",
            self.metadata.file_count,
            self.metadata.scan_duration_ms as f64 / 1000.0
        );
        let snapshot_path = Self::snapshot_path(root);
        let pretty_path = snapshot_path
            .strip_prefix(root)
            .map(|p| format!("./{}", p.display()))
            .unwrap_or_else(|_| snapshot_path.display().to_string());
        println!("Graph saved to {}", pretty_path);

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
        println!("  loct dead                    # Find unused exports");
        println!("  loct commands                # Show Tauri FE↔BE command bridges");
        println!("  loct slice <file> --json     # Extract context for AI agent");
    }
}

/// Best-effort check for uncommitted changes in the working tree
fn is_git_dirty(root: &Path) -> Option<bool> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(root)
        .output()
        .ok()?;
    Some(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

/// Run the init command: scan the project and save snapshot
pub fn run_init(root_list: &[PathBuf], parsed: &ParsedArgs) -> io::Result<()> {
    use crate::analyzer::coverage::{compute_command_gaps, normalize_cmd_name};
    use crate::analyzer::root_scan::{ScanConfig, scan_roots};
    use crate::analyzer::runner::default_analyzer_exts;
    use crate::analyzer::scan::{opt_globset, python_stdlib};
    use crate::config::LoctreeConfig;

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

    // Prepare scan configuration (reusing existing infrastructure)
    let py_stdlib = python_stdlib();
    let focus_set = opt_globset(&parsed.focus_patterns);
    let exclude_set = opt_globset(&parsed.exclude_report_patterns);

    let base_extensions = parsed
        .extensions
        .clone()
        .or_else(|| Some(default_analyzer_exts()));

    // Load custom Tauri command macros from .loctree/config.toml
    let loctree_config = root_list
        .first()
        .map(|root| LoctreeConfig::load(root))
        .unwrap_or_default();
    let custom_command_macros = loctree_config.tauri.command_macros;
    let command_detection = crate::analyzer::ast_js::CommandDetectionConfig::new(
        &loctree_config.tauri.dom_exclusions,
        &loctree_config.tauri.non_invoke_exclusions,
        &loctree_config.tauri.invalid_command_names,
    );

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
        custom_command_macros: &custom_command_macros,
        command_detection,
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

    // Build registered handlers set to filter BE commands (same as in loct.rs/loctree.rs)
    let registered_impls: HashSet<String> = scan_results
        .global_analyses
        .iter()
        .flat_map(|a| a.tauri_registered_handlers.iter().cloned())
        .collect();

    // Filter BE commands to only include registered handlers (or all if no registration info)
    let mut global_be_registered: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();
    for (name, locs) in &scan_results.global_be_commands {
        for (path, line, impl_name) in locs {
            if registered_impls.is_empty() || registered_impls.contains(impl_name) {
                global_be_registered.entry(name.clone()).or_default().push((
                    path.clone(),
                    *line,
                    impl_name.clone(),
                ));
            }
        }
    }

    // Build command bridges from global command data
    // Use normalized names for matching (handles camelCase FE vs snake_case BE)
    let (_missing_handlers, _unused_handlers) = compute_command_gaps(
        &scan_results.global_fe_commands,
        &global_be_registered,
        &focus_set,
        &exclude_set,
    );

    // Build normalized lookup maps for cross-matching
    // FE: normalized_name -> original_names (can have multiple originals mapping to same normalized)
    let mut fe_by_norm: HashMap<String, Vec<String>> = HashMap::new();
    for name in scan_results.global_fe_commands.keys() {
        fe_by_norm
            .entry(normalize_cmd_name(name))
            .or_default()
            .push(name.clone());
    }

    // BE: normalized_name -> original_names (only registered handlers)
    let mut be_by_norm: HashMap<String, Vec<String>> = HashMap::new();
    for name in global_be_registered.keys() {
        be_by_norm
            .entry(normalize_cmd_name(name))
            .or_default()
            .push(name.clone());
    }

    // Collect all unique normalized command names
    let mut all_normalized: HashSet<String> = HashSet::new();
    all_normalized.extend(fe_by_norm.keys().cloned());
    all_normalized.extend(be_by_norm.keys().cloned());

    // Create command bridges using normalized matching
    for norm_name in all_normalized {
        // Get all FE original names that normalize to this
        let fe_originals = fe_by_norm.get(&norm_name).cloned().unwrap_or_default();
        // Get all BE original names that normalize to this (registered only)
        let be_originals = be_by_norm.get(&norm_name).cloned().unwrap_or_default();

        // Collect all FE calls (from all original names that map here)
        let fe_calls: Vec<(String, usize)> = fe_originals
            .iter()
            .flat_map(|orig| {
                scan_results
                    .global_fe_commands
                    .get(orig)
                    .map(|v| {
                        v.iter()
                            .map(|(f, l, _)| (f.clone(), *l))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();

        // Get BE handler (prefer first BE original name found, registered only)
        let (be_handler, canonical_name) = be_originals
            .first()
            .and_then(|orig| {
                global_be_registered
                    .get(orig)
                    .and_then(|v| v.first())
                    .map(|(f, l, _)| (Some((f.clone(), *l)), orig.clone()))
            })
            .unwrap_or_else(|| {
                // No BE handler, use first FE name as canonical
                (
                    None,
                    fe_originals.first().cloned().unwrap_or(norm_name.clone()),
                )
            });

        let has_handler = be_handler.is_some();
        let is_called = !fe_calls.is_empty();

        snapshot.command_bridges.push(CommandBridge {
            name: canonical_name,
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

    // Store resolver configuration from scan results for caching
    if scan_results.ts_resolver_config.is_some() || !scan_results.py_roots.is_empty() {
        snapshot.metadata.resolver_config = Some(ResolverConfig {
            ts_paths: scan_results
                .ts_resolver_config
                .as_ref()
                .map(|c| c.ts_paths.clone())
                .unwrap_or_default(),
            ts_base_url: scan_results
                .ts_resolver_config
                .as_ref()
                .and_then(|c| c.ts_base_url.clone()),
            py_roots: scan_results.py_roots.clone(),
            rust_crate_roots: vec![], // TODO: populate from Cargo.toml scanning
        });
    }

    // Finalize metadata
    let duration_ms = start_time.elapsed().as_millis() as u64;
    snapshot.finalize_metadata(duration_ms);

    // Save snapshot
    snapshot.save(&snapshot_root)?;

    // Print summary
    snapshot.print_summary(&snapshot_root);

    // Auto mode: emit full artifact set into ./.loctree to avoid extra commands
    if parsed.auto_outputs {
        match write_auto_artifacts(&snapshot_root, &scan_results, parsed) {
            Ok(paths) => {
                if !paths.is_empty() {
                    println!("Artifacts saved under ./.loctree:");
                    for p in paths {
                        println!("  - {}", p);
                    }
                }
            }
            Err(err) => {
                eprintln!("[loctree][warn] failed to write auto artifacts: {}", err);
            }
        }
    }

    Ok(())
}

/// In auto mode, generate the full set of analysis artifacts inside ./.loctree
fn write_auto_artifacts(
    snapshot_root: &Path,
    scan_results: &crate::analyzer::root_scan::ScanResults,
    parsed: &ParsedArgs,
) -> io::Result<Vec<String>> {
    use crate::analyzer::coverage::{
        CommandUsage, compute_command_gaps_with_confidence, compute_unregistered_handlers,
    };
    use crate::analyzer::cycles::find_cycles;
    use crate::analyzer::dead_parrots::find_dead_exports;
    use crate::analyzer::output::{RootArtifacts, process_root_context, write_report};
    use crate::analyzer::pipelines::build_pipeline_summary;
    use crate::analyzer::sarif::{SarifInputs, generate_sarif_string};
    use crate::analyzer::scan::opt_globset;
    use serde_json::json;

    const DEFAULT_EXCLUDE_REPORT_PATTERNS: &[&str] =
        &["**/__tests__/**", "scripts/semgrep-fixtures/**"];
    const SCHEMA_NAME: &str = "loctree-json";
    const SCHEMA_VERSION: &str = "1.2.0";

    let mut created = Vec::new();

    let loctree_dir = Snapshot::artifacts_dir(snapshot_root);
    fs::create_dir_all(&loctree_dir)?;

    let report_path = loctree_dir.join("report.html");
    let analysis_json_path = loctree_dir.join("analysis.json");
    let sarif_path = loctree_dir.join("report.sarif");
    let circular_json_path = loctree_dir.join("circular.json");
    let races_json_path = loctree_dir.join("py_races.json");

    let focus_set = opt_globset(&parsed.focus_patterns);
    let mut exclude_patterns = parsed.exclude_report_patterns.clone();
    exclude_patterns.extend(
        DEFAULT_EXCLUDE_REPORT_PATTERNS
            .iter()
            .map(|p| p.to_string()),
    );
    let exclude_set = opt_globset(&exclude_patterns);

    let registered_impls: HashSet<String> = scan_results
        .global_analyses
        .iter()
        .flat_map(|a| a.tauri_registered_handlers.iter().cloned())
        .collect();

    let mut global_be_registered: CommandUsage = std::collections::HashMap::new();
    for (name, locs) in &scan_results.global_be_commands {
        for (path, line, impl_name) in locs {
            if registered_impls.is_empty() || registered_impls.contains(impl_name) {
                global_be_registered.entry(name.clone()).or_default().push((
                    path.clone(),
                    *line,
                    impl_name.clone(),
                ));
            }
        }
    }

    let (global_missing_handlers, global_unused_handlers) = compute_command_gaps_with_confidence(
        &scan_results.global_fe_commands,
        &global_be_registered,
        &focus_set,
        &exclude_set,
        &scan_results.global_analyses,
    );

    let global_unregistered_handlers = compute_unregistered_handlers(
        &scan_results.global_be_commands,
        &registered_impls,
        &focus_set,
        &exclude_set,
    );

    let pipeline_summary = build_pipeline_summary(
        &scan_results.global_analyses,
        &focus_set,
        &exclude_set,
        &scan_results.global_fe_commands,
        &scan_results.global_be_commands,
        &scan_results.global_fe_payloads,
        &scan_results.global_be_payloads,
    );

    let mut json_results = Vec::new();
    let mut report_sections = Vec::new();
    let analysis_args = ParsedArgs {
        graph: true,
        report_path: Some(report_path.clone()),
        output: OutputMode::Json,
        summary: true,
        summary_limit: parsed.summary_limit,
        analyze_limit: parsed.analyze_limit,
        top_dead_symbols: parsed.top_dead_symbols,
        skip_dead_symbols: parsed.skip_dead_symbols,
        focus_patterns: parsed.focus_patterns.clone(),
        exclude_report_patterns: exclude_patterns.clone(),
        max_graph_nodes: parsed.max_graph_nodes,
        max_graph_edges: parsed.max_graph_edges,
        ..ParsedArgs::default()
    };

    let git_ctx = Snapshot::current_git_context();

    for (idx, ctx) in scan_results.contexts.iter().cloned().enumerate() {
        let RootArtifacts {
            json_items,
            report_section,
        } = process_root_context(
            idx,
            ctx,
            &analysis_args,
            &scan_results.global_fe_commands,
            &scan_results.global_be_commands,
            &global_missing_handlers,
            &global_unregistered_handlers,
            &global_unused_handlers,
            &pipeline_summary,
            Some(&git_ctx),
            SCHEMA_NAME,
            SCHEMA_VERSION,
            &scan_results.global_analyses,
        );
        json_results.extend(json_items);
        if let Some(section) = report_section {
            report_sections.push(section);
        }
    }

    write_report(&report_path, &report_sections, parsed.verbose)?;
    created.push(format!(
        "./{}",
        report_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&report_path)
            .display()
    ));

    let all_graph_edges: Vec<_> = scan_results
        .contexts
        .iter()
        .flat_map(|ctx| ctx.graph_edges.clone())
        .collect();
    let cycles = find_cycles(&all_graph_edges);
    fs::write(
        &circular_json_path,
        serde_json::to_string_pretty(&json!({ "circularImports": cycles }))
            .map_err(io::Error::other)?,
    )?;
    created.push(format!(
        "./{}",
        circular_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&circular_json_path)
            .display()
    ));

    let race_items: Vec<_> = scan_results
        .global_analyses
        .iter()
        .flat_map(|a| {
            a.py_race_indicators.iter().map(move |ind| {
                json!({
                    "path": a.path,
                    "line": ind.line,
                    "type": ind.concurrency_type,
                    "pattern": ind.pattern,
                    "risk": ind.risk,
                    "message": ind.message,
                })
            })
        })
        .collect();
    fs::write(
        &races_json_path,
        serde_json::to_string_pretty(&race_items).map_err(io::Error::other)?,
    )?;
    created.push(format!(
        "./{}",
        races_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&races_json_path)
            .display()
    ));

    let bundle = json!({
        "schema": { "name": SCHEMA_NAME, "version": SCHEMA_VERSION },
        "generatedAt": time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_else(|_| "unknown".to_string()),
        "git": {
            "repo": git_ctx.repo,
            "branch": git_ctx.branch,
            "commit": git_ctx.commit,
            "scanId": git_ctx.scan_id,
        },
        "analysis": json_results,
        "pipelineSummary": pipeline_summary,
        "circularImports": cycles,
        "pyRaceIndicators": race_items,
    });
    fs::write(
        &analysis_json_path,
        serde_json::to_string_pretty(&bundle).map_err(io::Error::other)?,
    )?;
    created.push(format!(
        "./{}",
        analysis_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&analysis_json_path)
            .display()
    ));

    // Generate SARIF report for CI integration
    let all_ranked_dups: Vec<_> = scan_results
        .contexts
        .iter()
        .flat_map(|ctx| ctx.filtered_ranked.clone())
        .collect();
    let high_confidence = parsed.dead_confidence.as_deref() == Some("high");
    let dead_exports = find_dead_exports(
        &scan_results.global_analyses,
        high_confidence,
        None,
        crate::analyzer::dead_parrots::DeadFilterConfig::default(),
    );

    let sarif_content = generate_sarif_string(SarifInputs {
        duplicate_exports: &all_ranked_dups,
        missing_handlers: &global_missing_handlers,
        unused_handlers: &global_unused_handlers,
        dead_exports: &dead_exports,
        circular_imports: &cycles,
        pipeline_summary: &pipeline_summary,
    })
    .map_err(|err| io::Error::other(format!("Failed to serialize SARIF: {err}")))?;
    fs::write(&sarif_path, sarif_content)?;
    created.push(format!(
        "./{}",
        sarif_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&sarif_path)
            .display()
    ));

    // Save dead exports to standalone JSON for easy access
    let dead_json_path = loctree_dir.join("dead.json");
    let dead_json = json!({
        "deadExports": dead_exports.iter().map(|d| {
            json!({
                "file": d.file,
                "symbol": d.symbol,
                "line": d.line,
                "confidence": format!("{:?}", d.confidence),
                "reason": d.reason,
            })
        }).collect::<Vec<_>>(),
        "count": dead_exports.len(),
    });
    fs::write(
        &dead_json_path,
        serde_json::to_string_pretty(&dead_json).map_err(io::Error::other)?,
    )?;
    created.push(format!(
        "./{}",
        dead_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&dead_json_path)
            .display()
    ));

    // Save command handlers coverage to standalone JSON
    let handlers_json_path = loctree_dir.join("handlers.json");
    let handlers_json = json!({
        "missingHandlers": global_missing_handlers.iter().map(|gap| {
            json!({
                "command": gap.name,
                "locations": gap.locations.iter().map(|(path, line)| {
                    json!({ "path": path, "line": line })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "unusedHandlers": global_unused_handlers.iter().map(|gap| {
            json!({
                "command": gap.name,
                "implementationName": gap.implementation_name,
                "locations": gap.locations.iter().map(|(path, line)| {
                    json!({ "path": path, "line": line })
                }).collect::<Vec<_>>(),
                "confidence": gap.confidence.as_ref().map(|c| format!("{:?}", c)),
            })
        }).collect::<Vec<_>>(),
        "unregisteredHandlers": global_unregistered_handlers.iter().map(|gap| {
            json!({
                "handler": gap.name,
                "implementationName": gap.implementation_name,
                "locations": gap.locations.iter().map(|(path, line)| {
                    json!({ "path": path, "line": line })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "missing": global_missing_handlers.len(),
            "unused": global_unused_handlers.len(),
            "unregistered": global_unregistered_handlers.len(),
        },
    });
    fs::write(
        &handlers_json_path,
        serde_json::to_string_pretty(&handlers_json).map_err(io::Error::other)?,
    )?;
    created.push(format!(
        "./{}",
        handlers_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&handlers_json_path)
            .display()
    ));

    Ok(created)
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

    #[test]
    fn test_snapshot_new_creates_valid_metadata() {
        let snapshot = Snapshot::new(vec!["src".to_string()]);
        assert_eq!(snapshot.metadata.schema_version, SNAPSHOT_SCHEMA_VERSION);
        assert_eq!(snapshot.metadata.roots, vec!["src".to_string()]);
        assert!(snapshot.metadata.languages.is_empty());
        assert_eq!(snapshot.metadata.file_count, 0);
        assert!(!snapshot.metadata.generated_at.is_empty());
    }

    #[test]
    fn test_snapshot_path() {
        let path = Snapshot::snapshot_path(Path::new("/some/project"));
        // Accept branch@sha subdir if present, but require .loctree and snapshot.json
        assert!(path.starts_with("/some/project/.loctree"));
        assert!(path.ends_with("snapshot.json"));
    }

    #[test]
    fn test_snapshot_path_prefers_scan_id_when_available() {
        let ctx = Snapshot::current_git_context();
        if let Some(scan) = ctx.scan_id {
            let path = Snapshot::snapshot_path(Path::new("/tmp/loctree"));
            let display = path.display().to_string();
            assert!(
                display.contains(&scan),
                "expected snapshot path to include scan id {} but got {}",
                scan,
                display
            );
            assert!(display.ends_with("/snapshot.json"));
        }
    }

    #[test]
    fn test_artifacts_dir_prefers_scan_id() {
        let ctx = Snapshot::current_git_context();
        let dir = Snapshot::artifacts_dir(Path::new("/tmp/loctree"));
        if let Some(scan) = ctx.scan_id {
            let display = dir.display().to_string();
            assert!(
                display.contains(&scan),
                "expected artifacts dir to include scan id {} but got {}",
                scan,
                display
            );
        } else {
            assert!(dir.ends_with(Path::new(".loctree")));
        }
    }

    #[test]
    fn test_snapshot_exists_false() {
        let tmp = TempDir::new().expect("create temp dir");
        assert!(!Snapshot::exists(tmp.path()));
    }

    #[test]
    fn test_snapshot_exists_true() {
        let tmp = TempDir::new().expect("create temp dir");
        let snapshot = Snapshot::new(vec!["src".to_string()]);
        snapshot.save(tmp.path()).expect("save");
        assert!(Snapshot::exists(tmp.path()));
    }

    #[test]
    fn test_find_loctree_root_none() {
        let tmp = TempDir::new().expect("create temp dir");
        // Create a subdirectory without .loctree
        let subdir = tmp.path().join("sub");
        std::fs::create_dir(&subdir).expect("create subdir");
        assert!(Snapshot::find_loctree_root(&subdir).is_none());
    }

    #[test]
    fn test_find_loctree_root_found() {
        let tmp = TempDir::new().expect("create temp dir");
        // Create .loctree directory at root
        std::fs::create_dir(tmp.path().join(SNAPSHOT_DIR)).expect("create .loctree");
        // Create a nested subdirectory
        let subdir = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&subdir).expect("create nested subdir");
        let found = Snapshot::find_loctree_root(&subdir);
        assert!(found.is_some());
        let found = found.unwrap();
        assert!(found.join(SNAPSHOT_DIR).exists());
    }

    #[test]
    fn test_snapshot_with_files() {
        let tmp = TempDir::new().expect("create temp dir");
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        // Add file analysis
        let file = FileAnalysis::new("src/main.ts".into());
        snapshot.files.push(file);
        snapshot.metadata.file_count = 1;

        snapshot.save(tmp.path()).expect("save");
        let loaded = Snapshot::load(tmp.path()).expect("load");

        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].path, "src/main.ts");
        assert_eq!(loaded.metadata.file_count, 1);
    }

    #[test]
    fn test_snapshot_with_edges() {
        let tmp = TempDir::new().expect("create temp dir");
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        snapshot.edges.push(GraphEdge {
            from: "a.ts".to_string(),
            to: "b.ts".to_string(),
            label: "foo".to_string(),
        });

        snapshot.save(tmp.path()).expect("save");
        let loaded = Snapshot::load(tmp.path()).expect("load");

        assert_eq!(loaded.edges.len(), 1);
        assert_eq!(loaded.edges[0].from, "a.ts");
        assert_eq!(loaded.edges[0].to, "b.ts");
    }

    #[test]
    fn test_snapshot_with_command_bridges() {
        let tmp = TempDir::new().expect("create temp dir");
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        snapshot.command_bridges.push(CommandBridge {
            name: "get_user".to_string(),
            frontend_calls: vec![("app.ts".to_string(), 10)],
            backend_handler: Some(("handlers.rs".to_string(), 20)),
            has_handler: true,
            is_called: true,
        });

        snapshot.save(tmp.path()).expect("save");
        let loaded = Snapshot::load(tmp.path()).expect("load");

        assert_eq!(loaded.command_bridges.len(), 1);
        assert_eq!(loaded.command_bridges[0].name, "get_user");
        assert!(loaded.command_bridges[0].has_handler);
    }

    #[test]
    fn test_snapshot_with_event_bridges() {
        let tmp = TempDir::new().expect("create temp dir");
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        snapshot.event_bridges.push(EventBridge {
            name: "user_updated".to_string(),
            emits: vec![("events.ts".to_string(), 10, "emit".to_string())],
            listens: vec![("listener.ts".to_string(), 20)],
        });

        snapshot.save(tmp.path()).expect("save");
        let loaded = Snapshot::load(tmp.path()).expect("load");

        assert_eq!(loaded.event_bridges.len(), 1);
        assert_eq!(loaded.event_bridges[0].name, "user_updated");
    }

    #[test]
    fn test_snapshot_with_barrels() {
        let tmp = TempDir::new().expect("create temp dir");
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        snapshot.barrels.push(BarrelFile {
            path: "src/index.ts".to_string(),
            module_id: "src".to_string(),
            reexport_count: 5,
            targets: vec!["src/utils.ts".to_string()],
        });

        snapshot.save(tmp.path()).expect("save");
        let loaded = Snapshot::load(tmp.path()).expect("load");

        assert_eq!(loaded.barrels.len(), 1);
        assert_eq!(loaded.barrels[0].reexport_count, 5);
    }

    #[test]
    fn test_snapshot_metadata_serde() {
        let metadata = SnapshotMetadata {
            schema_version: "1.0".to_string(),
            generated_at: "2025-01-01T00:00:00Z".to_string(),
            roots: vec!["src".to_string()],
            languages: HashSet::from(["ts".to_string()]),
            file_count: 10,
            total_loc: 1000,
            scan_duration_ms: 500,
            resolver_config: Some(ResolverConfig {
                ts_paths: HashMap::from([("@/*".to_string(), vec!["src/*".to_string()])]),
                ts_base_url: Some("./src".to_string()),
                py_roots: vec![],
                rust_crate_roots: vec![],
            }),
            git_repo: None,
            git_branch: None,
            git_commit: None,
            git_scan_id: None,
        };

        let json = serde_json::to_string(&metadata).expect("serialize");
        let deser: SnapshotMetadata = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deser.file_count, 10);
        assert!(deser.resolver_config.is_some());
    }

    #[test]
    fn test_graph_edge_serde() {
        let edge = GraphEdge {
            from: "a.ts".to_string(),
            to: "b.ts".to_string(),
            label: "import".to_string(),
        };

        let json = serde_json::to_string(&edge).expect("serialize");
        let deser: GraphEdge = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deser.from, "a.ts");
        assert_eq!(deser.label, "import");
    }

    #[test]
    fn test_resolver_config_default() {
        let config = ResolverConfig::default();
        assert!(config.ts_paths.is_empty());
        assert!(config.ts_base_url.is_none());
        assert!(config.py_roots.is_empty());
        assert!(config.rust_crate_roots.is_empty());
    }

    #[test]
    fn test_snapshot_export_index() {
        let tmp = TempDir::new().expect("create temp dir");
        let mut snapshot = Snapshot::new(vec!["src".to_string()]);

        snapshot
            .export_index
            .insert("Button".to_string(), vec!["src/Button.tsx".to_string()]);

        snapshot.save(tmp.path()).expect("save");
        let loaded = Snapshot::load(tmp.path()).expect("load");

        assert!(loaded.export_index.contains_key("Button"));
        assert_eq!(
            loaded.export_index.get("Button").unwrap(),
            &vec!["src/Button.tsx".to_string()]
        );
    }
}
