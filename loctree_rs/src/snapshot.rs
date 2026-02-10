//! Snapshot module for persisting the code graph to disk.
//!
//! This module implements the "scan once, slice many" philosophy:
//! - `loctree init` or bare `loctree` scans the project and saves a snapshot
//! - Subsequent queries load the snapshot for instant context slicing

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::args::ParsedArgs;
use crate::types::{FileAnalysis, OutputMode};

/// Current schema version for snapshot format (synced to crate version)
pub const SNAPSHOT_SCHEMA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Extract major.minor from a semver string (e.g. "0.8.10" -> "0.8")
/// Patch bumps don't change the snapshot schema, so we only compare major.minor.
fn schema_major_minor(version: &str) -> &str {
    // Find second dot (end of minor)
    match version
        .find('.')
        .and_then(|i| version[i + 1..].find('.').map(|j| i + 1 + j))
    {
        Some(pos) => &version[..pos],
        None => version, // no patch component, return as-is
    }
}

/// Default snapshot directory name
pub const SNAPSHOT_DIR: &str = ".loctree";

/// Default snapshot file name
pub const SNAPSHOT_FILE: &str = "snapshot.json";

/// Environment variable to override the cache base directory.
const LOCT_CACHE_DIR_ENV: &str = "LOCT_CACHE_DIR";

/// Returns the global cache base directory for loctree artifacts.
///
/// Priority:
/// 1. `LOCT_CACHE_DIR` environment variable
/// 2. Platform default: `~/Library/Caches/loctree` (macOS) or `$XDG_CACHE_HOME/loctree` (Linux)
/// 3. Fallback: OS temp dir (for environments without a home/cache directory)
pub fn cache_base_dir() -> PathBuf {
    if let Ok(custom) = std::env::var(LOCT_CACHE_DIR_ENV) {
        let custom = custom.trim();
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }
    if let Some(cache_dir) = dirs::cache_dir() {
        return cache_dir.join("loctree");
    }
    // Last resort: temp dir (avoid dirtying repos in minimal/container envs)
    std::env::temp_dir().join("loctree")
}

/// Returns the cache directory for a specific project.
///
/// Layout: `<cache_base>/projects/<project_id>/`
/// where `project_id` is the first 16 hex chars of SHA-256(canonical_project_root).
pub fn project_cache_dir(root: &Path) -> PathBuf {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash = hasher.finalize();
    let project_id = format!("{:x}", hash).chars().take(16).collect::<String>();

    // If LOCT_CACHE_DIR is set:
    // - Relative path => interpret relative to the project root (so scans from subdirs still write to root/.loctree)
    // - Absolute path => treat as a multi-project cache base (we still namespace by project_id)
    if let Ok(custom) = std::env::var(LOCT_CACHE_DIR_ENV) {
        let custom = custom.trim();
        if !custom.is_empty() {
            let custom_path = PathBuf::from(custom);
            if custom_path.is_relative() {
                return canonical.join(custom_path);
            }
            return custom_path.join("projects").join(project_id);
        }
    }

    cache_base_dir().join("projects").join(project_id)
}

/// Returns the project-local config directory (for user-editable files).
///
/// Config files (config.toml, suppressions.toml) stay in the project — they are
/// user-editable and may be version-controlled. Only cache artifacts move to global cache.
pub fn project_config_dir(root: &Path) -> PathBuf {
    root.join(SNAPSHOT_DIR)
}

fn write_atomic(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent for atomic write"))?;
    let mut tmp = tempfile::Builder::new()
        .prefix("loctree_tmp")
        .tempfile_in(dir)?;
    tmp.write_all(contents.as_ref())?;
    tmp.flush()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.canonicalize().ok()?;
    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() || git_dir.is_file() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent.to_path_buf(),
            _ => return None,
        }
    }
}

fn normalize_root_dir(root: &Path) -> PathBuf {
    if root.is_file() {
        return root.parent().unwrap_or(root).to_path_buf();
    }
    root.to_path_buf()
}

fn has_project_marker(root: &Path) -> bool {
    const MARKERS: [&str; 11] = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "tsconfig.json",
        "deno.json",
        "deno.jsonc",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "composer.json",
    ];
    MARKERS.iter().any(|marker| root.join(marker).is_file())
}

pub(crate) fn resolve_snapshot_root(root_list: &[PathBuf]) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    let roots: Vec<PathBuf> = if root_list.is_empty() {
        vec![cwd.clone()]
    } else {
        root_list
            .iter()
            .map(|root| normalize_root_dir(root))
            .collect()
    };

    let mut loctree_roots: Vec<PathBuf> = roots
        .iter()
        .filter_map(|root| Snapshot::find_loctree_root(root))
        .collect();
    if let Some(first) = loctree_roots.pop()
        && loctree_roots.iter().all(|root| root == &first)
    {
        return first;
    }

    if roots.len() == 1 && has_project_marker(&roots[0]) {
        return roots[0].clone();
    }

    if let Some(first_git) = roots.first().and_then(|root| find_git_root(root))
        && roots
            .iter()
            .all(|root| find_git_root(root).as_ref() == Some(&first_git))
    {
        return first_git;
    }

    find_git_root(&cwd).unwrap_or(cwd)
}

#[derive(Clone, Debug)]
struct DeclaredEntrypoint {
    source: String,
    path: String,
    exists: bool,
    resolved: bool,
    note: Option<String>,
}

fn normalize_snapshot_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn resolve_declared_path(root: &Path, raw: &str) -> Option<(String, bool)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains('*') || trimmed.contains('?') {
        return None;
    }
    if trimmed.starts_with("node:") {
        return None;
    }
    let cleaned = trimmed.trim_start_matches("./");
    let base = Path::new(cleaned);
    let full = if base.is_absolute() {
        base.to_path_buf()
    } else {
        root.join(cleaned)
    };
    let exists = full.exists();
    let rel = full.strip_prefix(root).unwrap_or(&full);
    Some((normalize_snapshot_path(&rel.to_string_lossy()), exists))
}

fn note_for_declared_path(path: &str, source: &str) -> Option<String> {
    let lowered = path.to_lowercase();
    if source.contains("types") || lowered.ends_with(".d.ts") {
        return Some("types entry (non-runtime)".to_string());
    }
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => {
            Some("js/ts entrypoint markers not detected".to_string())
        }
        _ => None,
    }
}

fn collect_declared_entrypoints(summary: &ManifestSummary) -> Vec<DeclaredEntrypoint> {
    let mut declared = Vec::new();
    let root = PathBuf::from(&summary.root);

    if let Some(pkg) = &summary.package_json {
        for (label, value) in [
            ("package.json:main", pkg.main.as_ref()),
            ("package.json:module", pkg.module.as_ref()),
            ("package.json:types", pkg.types.as_ref()),
        ] {
            if let Some(raw) = value {
                if let Some((path, exists)) = resolve_declared_path(&root, raw) {
                    declared.push(DeclaredEntrypoint {
                        source: label.to_string(),
                        path,
                        exists,
                        resolved: true,
                        note: note_for_declared_path(raw, label),
                    });
                } else {
                    declared.push(DeclaredEntrypoint {
                        source: label.to_string(),
                        path: raw.to_string(),
                        exists: false,
                        resolved: false,
                        note: Some("unresolved manifest path".to_string()),
                    });
                }
            }
        }
        for entry in &pkg.exports {
            let source = format!("package.json:exports:{}", entry.key);
            if let Some((path, exists)) = resolve_declared_path(&root, &entry.path) {
                declared.push(DeclaredEntrypoint {
                    source,
                    path,
                    exists,
                    resolved: true,
                    note: note_for_declared_path(&entry.path, "package.json:exports"),
                });
            } else {
                declared.push(DeclaredEntrypoint {
                    source,
                    path: entry.path.clone(),
                    exists: false,
                    resolved: false,
                    note: Some("unresolved manifest path".to_string()),
                });
            }
        }
        for entry in &pkg.bin {
            let source = format!("package.json:bin:{}", entry.key);
            if let Some((path, exists)) = resolve_declared_path(&root, &entry.path) {
                declared.push(DeclaredEntrypoint {
                    source,
                    path,
                    exists,
                    resolved: true,
                    note: note_for_declared_path(&entry.path, "package.json:bin"),
                });
            } else {
                declared.push(DeclaredEntrypoint {
                    source,
                    path: entry.path.clone(),
                    exists: false,
                    resolved: false,
                    note: Some("unresolved manifest path".to_string()),
                });
            }
        }
    }

    if let Some(cargo) = &summary.cargo_toml {
        if let Some(lib_path) = &cargo.lib_path {
            if let Some((path, exists)) = resolve_declared_path(&root, lib_path) {
                declared.push(DeclaredEntrypoint {
                    source: "Cargo.toml:lib".to_string(),
                    path,
                    exists,
                    resolved: true,
                    note: None,
                });
            }
        } else {
            let lib_default = root.join("src/lib.rs");
            let rel = normalize_snapshot_path(
                &lib_default
                    .strip_prefix(&root)
                    .unwrap_or(&lib_default)
                    .to_string_lossy(),
            );
            if lib_default.exists() {
                declared.push(DeclaredEntrypoint {
                    source: "Cargo.toml:lib:default".to_string(),
                    path: rel,
                    exists: true,
                    resolved: true,
                    note: None,
                });
            }
        }

        if cargo.bins.is_empty() {
            let main_default = root.join("src/main.rs");
            let rel = normalize_snapshot_path(
                &main_default
                    .strip_prefix(&root)
                    .unwrap_or(&main_default)
                    .to_string_lossy(),
            );
            if main_default.exists() {
                declared.push(DeclaredEntrypoint {
                    source: "Cargo.toml:bin:default".to_string(),
                    path: rel,
                    exists: true,
                    resolved: true,
                    note: None,
                });
            }
        } else {
            for bin in &cargo.bins {
                let source = format!("Cargo.toml:bin:{}", bin.name);
                let path_value = bin
                    .path
                    .clone()
                    .unwrap_or_else(|| format!("src/bin/{}.rs", bin.name));
                if let Some((path, exists)) = resolve_declared_path(&root, &path_value) {
                    declared.push(DeclaredEntrypoint {
                        source,
                        path,
                        exists,
                        resolved: true,
                        note: None,
                    });
                }
            }
        }

        for member in &cargo.workspace_members {
            let member_root = root.join(member);
            if !member_root.join("Cargo.toml").exists() {
                continue;
            }
            let member_lib = member_root.join("src/lib.rs");
            if member_lib.exists() {
                let rel = normalize_snapshot_path(
                    &member_lib
                        .strip_prefix(&root)
                        .unwrap_or(&member_lib)
                        .to_string_lossy(),
                );
                declared.push(DeclaredEntrypoint {
                    source: format!("Cargo.toml:member:{}:lib", member),
                    path: rel,
                    exists: true,
                    resolved: true,
                    note: None,
                });
            }
            let member_main = member_root.join("src/main.rs");
            if member_main.exists() {
                let rel = normalize_snapshot_path(
                    &member_main
                        .strip_prefix(&root)
                        .unwrap_or(&member_main)
                        .to_string_lossy(),
                );
                declared.push(DeclaredEntrypoint {
                    source: format!("Cargo.toml:member:{}:bin", member),
                    path: rel,
                    exists: true,
                    resolved: true,
                    note: None,
                });
            }
        }
    }

    if let Some(py) = &summary.pyproject_toml {
        for script in &py.scripts {
            declared.push(DeclaredEntrypoint {
                source: "pyproject.toml:scripts".to_string(),
                path: script.clone(),
                exists: false,
                resolved: false,
                note: Some("script entry (no path mapping)".to_string()),
            });
        }
        for entry in &py.entry_points {
            declared.push(DeclaredEntrypoint {
                source: "pyproject.toml:entry-points".to_string(),
                path: entry.clone(),
                exists: false,
                resolved: false,
                note: Some("entry-point group (no path mapping)".to_string()),
            });
        }
    }

    declared
}

fn compute_entrypoint_drift(
    manifest_summary: &[ManifestSummary],
    entrypoints: &[EntrypointSummary],
) -> EntrypointDriftSummary {
    let mut drift = EntrypointDriftSummary::default();

    let mut declared_paths: HashSet<String> = HashSet::new();
    let entrypoint_paths: HashSet<String> = entrypoints
        .iter()
        .map(|e| normalize_snapshot_path(&e.path))
        .collect();

    for summary in manifest_summary {
        for declared in collect_declared_entrypoints(summary) {
            if !declared.resolved {
                drift.declared_unresolved.push(EntrypointDriftItem {
                    source: declared.source,
                    path: declared.path,
                    note: declared.note,
                });
                continue;
            }
            let path = normalize_snapshot_path(&declared.path);
            declared_paths.insert(path.clone());
            if !declared.exists {
                drift.declared_missing.push(EntrypointDriftItem {
                    source: declared.source,
                    path,
                    note: declared.note,
                });
            } else if !entrypoint_paths.contains(&path) {
                drift.declared_without_marker.push(EntrypointDriftItem {
                    source: declared.source,
                    path,
                    note: declared.note,
                });
            }
        }
    }

    for entry in entrypoints {
        let path = normalize_snapshot_path(&entry.path);
        if !declared_paths.contains(&path) {
            drift.code_only_entrypoints.push(EntrypointSummary {
                path,
                kinds: entry.kinds.clone(),
            });
        }
    }

    drift
}

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
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    /// Manifest summaries (package.json, Cargo.toml, pyproject.toml)
    #[serde(default)]
    pub manifest_summary: Vec<ManifestSummary>,
    /// Detected entrypoints across files
    #[serde(default)]
    pub entrypoints: Vec<EntrypointSummary>,
    /// Drift between declared manifest roots and code entrypoints
    #[serde(default)]
    pub entrypoint_drift: EntrypointDriftSummary,
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

/// Single manifest entry (key -> path).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ManifestEntry {
    pub key: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PackageJsonSummary {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub package_type: Option<String>,
    #[serde(default)]
    pub main: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub types: Option<String>,
    #[serde(default)]
    pub exports: Vec<ManifestEntry>,
    #[serde(default)]
    pub bin: Vec<ManifestEntry>,
    #[serde(default)]
    pub workspaces: Vec<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub package_manager: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CargoBinSummary {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CargoTomlSummary {
    #[serde(default)]
    pub package_name: Option<String>,
    #[serde(default)]
    pub workspace_members: Vec<String>,
    #[serde(default)]
    pub workspace_default_members: Vec<String>,
    #[serde(default)]
    pub lib_path: Option<String>,
    #[serde(default)]
    pub bins: Vec<CargoBinSummary>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub crate_roots: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PyProjectSummary {
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub poetry_name: Option<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub entry_points: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ManifestSummary {
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub package_json: Option<PackageJsonSummary>,
    #[serde(default)]
    pub cargo_toml: Option<CargoTomlSummary>,
    #[serde(default)]
    pub pyproject_toml: Option<PyProjectSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EntrypointSummary {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub kinds: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EntrypointDriftItem {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EntrypointDriftSummary {
    #[serde(default)]
    pub declared_missing: Vec<EntrypointDriftItem>,
    #[serde(default)]
    pub declared_without_marker: Vec<EntrypointDriftItem>,
    #[serde(default)]
    pub code_only_entrypoints: Vec<EntrypointSummary>,
    #[serde(default)]
    pub declared_unresolved: Vec<EntrypointDriftItem>,
}

impl EntrypointDriftSummary {
    pub fn is_empty(&self) -> bool {
        self.declared_missing.is_empty()
            && self.declared_without_marker.is_empty()
            && self.code_only_entrypoints.is_empty()
            && self.declared_unresolved.is_empty()
    }
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
    /// True if this is a FE↔FE sync pattern (emit and listen both in frontend)
    #[serde(default)]
    pub is_fe_sync: bool,
    /// True if emit and listen are in the same file (strongest FE↔FE indicator)
    #[serde(default)]
    pub same_file_sync: bool,
}

/// Export index entry (used by VS2 slice module)
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
                manifest_summary: Vec::new(),
                entrypoints: Vec::new(),
                entrypoint_drift: EntrypointDriftSummary::default(),
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

    /// Get git repository info (repo name, branch, commit) for given root.
    ///
    /// Uses libgit2's repository discovery to properly find the git root,
    /// even when called from a deeply nested subdirectory. This fixes issues
    /// where git commands would fail if `root` wasn't directly inside a git repo.
    fn get_git_info(root: &Path) -> (Option<String>, Option<String>, Option<String>) {
        use std::process::{Command, Stdio};

        // Find the actual git root (searches upward from root)
        let git_root = match crate::git::find_git_root(root) {
            Some(r) => r,
            None => return (None, None, None), // Not a git repository
        };

        let repo = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(&git_root)
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .and_then(|url| {
                url.trim()
                    .rsplit('/')
                    .next()
                    .or_else(|| url.trim().rsplit(':').next())
                    .map(|s| s.trim_end_matches(".git").to_string())
            });
        let branch = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&git_root)
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });
        let commit = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(&git_root)
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
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

    /// Build git context for given root (repo, branch, commit, scan_id)
    pub fn git_context_for(root: &Path) -> GitContext {
        let (repo, branch, commit) = Self::get_git_info(root);
        let scan_id = branch.as_ref().map(|b| {
            let base = Self::sanitize_ref(b);
            commit.as_ref().map_or(base.clone(), |c| {
                format!("{}@{}", base, Self::sanitize_ref(c))
            })
        });
        GitContext {
            repo,
            branch,
            commit,
            scan_id,
        }
    }
    /// Build git context for CWD (backwards compat)
    pub fn current_git_context() -> GitContext {
        Self::git_context_for(&std::env::current_dir().unwrap_or_default())
    }

    fn candidate_snapshot_paths(root: &Path) -> Vec<PathBuf> {
        let cache_dir = project_cache_dir(root);
        let mut paths = Vec::new();
        if let Some(seg) = Self::git_context_for(root).scan_id {
            // Primary: global cache with branch@sha
            paths.push(cache_dir.join(&seg).join(SNAPSHOT_FILE));
            // Legacy: project-local with branch@sha
            paths.push(root.join(SNAPSHOT_DIR).join(seg).join(SNAPSHOT_FILE));
        }
        // Legacy: project-local flat
        paths.push(root.join(SNAPSHOT_DIR).join(SNAPSHOT_FILE));
        // Global cache flat (no git context)
        paths.push(cache_dir.join(SNAPSHOT_FILE));
        paths
    }

    /// Get the snapshot file path for a given root (writes go here).
    ///
    /// Returns a path under the global cache directory.
    pub fn snapshot_path(root: &Path) -> PathBuf {
        let cache_dir = project_cache_dir(root);
        if let Some(seg) = Self::git_context_for(root).scan_id {
            cache_dir.join(seg).join(SNAPSHOT_FILE)
        } else {
            cache_dir.join(SNAPSHOT_FILE)
        }
    }

    /// Directory where snapshot and artifacts should be stored for the current scan.
    ///
    /// Returns a path under the global cache directory.
    pub fn artifacts_dir(root: &Path) -> PathBuf {
        let path = Self::snapshot_path(root);
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| project_cache_dir(root))
    }

    fn remove_any_path(path: &Path) {
        let meta = fs::symlink_metadata(path);
        let Ok(meta) = meta else {
            return;
        };
        if meta.is_dir() {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
    }

    fn refresh_latest_artifacts(root: &Path) -> io::Result<()> {
        let Some(scan_id) = Self::git_context_for(root).scan_id else {
            return Ok(());
        };

        let base_dir = project_cache_dir(root);
        let scan_dir = base_dir.join(&scan_id);
        if !scan_dir.exists() {
            return Ok(());
        }

        // Validate scan_dir is contained within base_dir (prevent path traversal via crafted git refs)
        let canon_base = base_dir.canonicalize().unwrap_or_else(|_| base_dir.clone());
        let canon_scan = scan_dir.canonicalize().unwrap_or_else(|_| scan_dir.clone());
        if !canon_scan.starts_with(&canon_base) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "scan directory escapes cache base: {}",
                    canon_scan.display()
                ),
            ));
        }

        let latest_dir = base_dir.join("latest");
        Self::remove_any_path(&latest_dir);
        fs::create_dir_all(&latest_dir)?;

        // Keep this list small and stable; these are the key artifacts CI and tooling depend on.
        // Other ad-hoc outputs (e.g. report.html, circular.json) can still be found in the scan_id dir.
        const POINTER_FILES: &[&str] = &[
            "snapshot.json",
            "analysis.json",
            "findings.json",
            "agent.json",
            "manifest.json",
            "report.sarif",
            "dead.json",
            "handlers.json",
            "circular.json",
            "py_races.json",
            "report.html",
        ];

        for name in POINTER_FILES {
            let src = canon_scan.join(name);
            if !src.exists() {
                continue;
            }
            // nosemgrep:rust.actix.path-traversal.tainted-path.tainted-path - SAFETY: canon_scan is canonicalized and bounded to canon_base via starts_with guard above; name is from hardcoded POINTER_FILES const
            let bytes = fs::read(&src)?;

            // Stable pointers at base dir: `.loctree/agent.json`, `.loctree/findings.json`, ...
            // These are ignored by default gitignore (config.toml/suppressions.toml are explicitly unignored).
            let dst_flat = base_dir.join(name);
            write_atomic(&dst_flat, &bytes)?;

            // Mirror into `.loctree/latest/` for snapshot-as-proof workflows.
            let dst_latest = latest_dir.join(name);
            write_atomic(&dst_latest, &bytes)?;
        }

        Ok(())
    }

    /// Check if a snapshot exists for the given root (checks both cache and legacy paths).
    pub fn exists(root: &Path) -> bool {
        Self::candidate_snapshot_paths(root)
            .iter()
            .any(|p| p.exists())
    }

    /// Walk upward from `start` looking for a loctree project root.
    ///
    /// A directory is considered a root if it has a `.loctree/` config dir
    /// (user-editable files like config.toml, suppressions.toml) OR if its
    /// global cache directory contains at least one snapshot.
    pub fn find_loctree_root(start: &Path) -> Option<PathBuf> {
        let mut current = start.canonicalize().ok()?;
        loop {
            // Check for .loctree config dir (config.toml, suppressions.toml, .loctreeignore)
            if current.join(SNAPSHOT_DIR).exists() {
                return Some(current);
            }
            // Check global cache — require an actual snapshot, not just an empty dir
            let cache = project_cache_dir(&current);
            if cache.is_dir() && Self::cache_has_snapshot(&cache) {
                return Some(current);
            }
            match current.parent() {
                Some(parent) if parent != current => current = parent.to_path_buf(),
                _ => return None,
            }
        }
    }

    /// Returns true if a cache directory contains at least one snapshot.json.
    fn cache_has_snapshot(cache_dir: &Path) -> bool {
        if cache_dir.join(SNAPSHOT_FILE).exists() {
            return true;
        }
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                if entry.path().join(SNAPSHOT_FILE).exists() {
                    return true;
                }
            }
        }
        false
    }

    /// Normalize a path to be relative to snapshot roots
    ///
    /// Handles:
    /// - Absolute paths: strips snapshot root prefix
    /// - Relative paths with ./: removes the prefix
    /// - Windows paths: normalizes backslashes to forward slashes
    ///
    /// # Examples
    /// ```ignore
    /// // Given snapshot with root "/Users/foo/project"
    /// snapshot.normalize_path("/Users/foo/project/src/main.rs") // => "src/main.rs"
    /// snapshot.normalize_path("./src/main.rs") // => "src/main.rs"
    /// snapshot.normalize_path("src\\main.rs") // => "src/main.rs"
    /// ```
    pub fn normalize_path(&self, path: &str) -> String {
        let path = path.trim_start_matches("./").replace('\\', "/");

        // If path is absolute, try to strip snapshot root prefixes
        if path.starts_with('/') {
            for root in &self.metadata.roots {
                let root_normalized = root.trim_end_matches('/');
                if let Some(relative) = path.strip_prefix(root_normalized) {
                    // Remove leading slash from relative path
                    return relative.trim_start_matches('/').to_string();
                }
            }
        }

        path
    }

    /// Find the most recent snapshot in .loctree/*/snapshot.json
    ///
    /// This function is useful for query mode where we want to automatically
    /// discover the latest snapshot without requiring explicit path specification.
    ///
    /// # Arguments
    /// * `explicit_path` - If provided, use this path directly instead of searching
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - Path to the snapshot file
    /// * `Err(String)` - Helpful error message if no snapshot found
    ///
    /// # Example
    /// ```ignore
    /// // Auto-discover latest snapshot
    /// let path = Snapshot::find_latest_snapshot(None)?;
    ///
    /// // Use explicit path
    /// let path = Snapshot::find_latest_snapshot(Some(Path::new(".loctree/main@abc123/snapshot.json")))?;
    /// ```
    pub fn find_latest_snapshot(explicit_path: Option<&Path>) -> Result<PathBuf, String> {
        // If explicit path provided, validate and return it
        if let Some(path) = explicit_path {
            if path.exists() {
                return Ok(path.to_path_buf());
            } else {
                return Err(format!(
                    "Snapshot not found at '{}'. Run `loct scan` first.",
                    path.display()
                ));
            }
        }

        // Search for .loctree directory starting from current directory
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;

        Self::find_latest_snapshot_in(&cwd)
    }

    /// Find latest snapshot starting from a given root directory.
    /// Searches both global cache and legacy project-local `.loctree/` directory.
    pub fn find_latest_snapshot_in(root: &Path) -> Result<PathBuf, String> {
        let mut snapshots: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        let effective_root =
            Self::find_loctree_root(root).unwrap_or_else(|| normalize_root_dir(root));

        // Search global cache directory for this project (effective root, not CWD)
        let cache_dir = project_cache_dir(&effective_root);
        Self::collect_snapshots_from_dir(&cache_dir, &mut snapshots);

        // Search legacy project-local .loctree/ directory
        let legacy_dir = effective_root.join(SNAPSHOT_DIR);
        Self::collect_snapshots_from_dir(&legacy_dir, &mut snapshots);

        if snapshots.is_empty() {
            return Err("No snapshot found. Run `loct scan` first to create one.".to_string());
        }

        // Sort by mtime (newest first) and return the most recent
        snapshots.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(snapshots.into_iter().next().unwrap().0)
    }

    /// Collect all snapshot.json files from a directory (flat + subdirs).
    fn collect_snapshots_from_dir(
        dir: &Path,
        snapshots: &mut Vec<(PathBuf, std::time::SystemTime)>,
    ) {
        // Check flat: dir/snapshot.json
        let flat_path = dir.join(SNAPSHOT_FILE);
        if let Ok(meta) = fs::metadata(&flat_path)
            && let Ok(mtime) = meta.modified()
        {
            snapshots.push((flat_path, mtime));
        }
        // Check subdirs: dir/*/snapshot.json
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let snapshot_path = path.join(SNAPSHOT_FILE);
                    if let Ok(meta) = fs::metadata(&snapshot_path)
                        && let Ok(mtime) = meta.modified()
                    {
                        snapshots.push((snapshot_path, mtime));
                    }
                }
            }
        }
    }

    /// Save snapshot to disk
    pub fn save(&self, root: &Path) -> io::Result<()> {
        // If a snapshot already exists for the same branch/commit AND worktree is clean,
        // skip rewriting (no changes since last commit = same code).
        // But if worktree is dirty, ALWAYS save fresh snapshot (user is actively working).
        if let (Some(commit), Some(branch), Ok(existing)) = (
            self.metadata.git_commit.as_ref(),
            self.metadata.git_branch.as_ref(),
            Self::load(root),
        ) && existing.metadata.git_commit.as_ref() == Some(commit)
            && existing.metadata.git_branch.as_ref() == Some(branch)
        {
            let mut existing_roots = existing.metadata.roots.clone();
            let mut current_roots = self.metadata.roots.clone();
            existing_roots.sort();
            current_roots.sort();
            let same_roots = existing_roots == current_roots;
            let dirty = is_git_dirty(root).unwrap_or(false);
            if !dirty && same_roots {
                // Clean worktree + same commit = skip (nothing changed)
                crate::progress::info(&format!(
                    "Snapshot {}@{} up-to-date, skipping",
                    branch,
                    &commit[..7.min(commit.len())]
                ));
                return Ok(());
            }
            // Dirty worktree = continue and save fresh snapshot
        }

        let snapshot_path = Self::snapshot_path(root);
        if let Some(dir) = snapshot_path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        write_atomic(&snapshot_path, json)?;

        // Refresh stable pointers (base_dir/*.json + base_dir/latest/) for CI and human workflows.
        // This is a no-op for non-git dirs (no scan_id).
        let _ = Self::refresh_latest_artifacts(root);

        Ok(())
    }

    /// Load snapshot from disk (used by VS2 slice module)
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

        // Check schema version compatibility (major.minor only - patch bumps don't change schema)
        if schema_major_minor(&snapshot.metadata.schema_version)
            != schema_major_minor(SNAPSHOT_SCHEMA_VERSION)
        {
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
        let snapshot_path = Self::snapshot_path(root);
        let pretty_path = snapshot_path
            .strip_prefix(root)
            .map(|p| format!("./{}", p.display()))
            .unwrap_or_else(|_| snapshot_path.display().to_string());
        crate::progress::info(&format!("Saved to {}", pretty_path));

        let languages: Vec<_> = self.metadata.languages.iter().collect();
        if !languages.is_empty() {
            eprintln!(
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
            eprint!("Commands: {} handlers", handler_count);
            if missing_handlers > 0 {
                eprint!(", {} missing", missing_handlers);
            }
            if unused_handlers > 0 {
                eprint!(", {} unused", unused_handlers);
            }
            eprintln!();
        }

        let event_count = self.event_bridges.len();
        if event_count > 0 {
            eprintln!("Events: {} tracked", event_count);
        }

        // Check for cycles or issues
        let barrel_count = self.barrels.len();
        if barrel_count > 0 {
            eprintln!("Barrels: {} detected", barrel_count);
        }

        // Count duplicate exports (symbols exported from multiple files)
        let duplicate_count = self
            .export_index
            .values()
            .filter(|files| files.len() > 1)
            .count();
        if duplicate_count > 0 {
            eprintln!("Duplicates: {} export groups", duplicate_count);
        }

        // Count indexed parameters (NEW in 0.8.4)
        let param_count: usize = self
            .files
            .iter()
            .flat_map(|f| f.exports.iter())
            .map(|e| e.params.len())
            .sum();
        if param_count > 0 {
            let func_with_params = self
                .files
                .iter()
                .flat_map(|f| f.exports.iter())
                .filter(|e| !e.params.is_empty())
                .count();
            eprintln!(
                "Params: {} indexed ({} functions)",
                param_count, func_with_params
            );
        }

        eprintln!("Status: OK");
        eprintln!();
        eprintln!("Next steps:");
        eprintln!("  loct --for-ai                # Project overview for AI agents");
        eprintln!("  loct slice <file> --json     # Extract context with dependencies");
        eprintln!("  loct twins                   # Dead parrots + duplicates + barrel chaos");
        eprintln!("  loct '.files | length'       # jq-style queries on snapshot");
        eprintln!("  loct query who-imports <f>   # Quick graph queries");
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
///
/// # Arguments
/// * `root_list` - List of root directories to scan
/// * `parsed` - Parsed command-line arguments
/// * `quiet_summary` - If true, skip printing the summary (useful for internal scans like dist mode)
pub fn run_init_with_options(
    root_list: &[PathBuf],
    parsed: &ParsedArgs,
    quiet_summary: bool,
) -> io::Result<()> {
    use crate::analyzer::coverage::{compute_command_gaps, normalize_cmd_name};
    use crate::analyzer::root_scan::{ScanConfig, scan_roots};
    use crate::analyzer::runner::default_analyzer_exts;
    use crate::analyzer::scan::{opt_globset, python_stdlib};
    use crate::config::LoctreeConfig;

    let start_time = Instant::now();
    let mut parsed = parsed.clone();

    // Snapshot root defaults to the first provided root (common UX: keep artifacts near target),
    // falling back to CWD if multiple roots are provided.
    let snapshot_root = resolve_snapshot_root(root_list);

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

    // Show spinner during scan (Black-style feedback)
    let spinner = crate::progress::Spinner::new(&format!("Scanning ({})...", scan_mode));

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
    parsed.library_mode = parsed.library_mode || loctree_config.library_mode;
    if parsed.library_mode && parsed.library_example_globs.is_empty() {
        parsed.library_example_globs = loctree_config.library_example_globs.clone();
    }
    let custom_command_macros = loctree_config.tauri.command_macros;
    let command_detection = crate::analyzer::ast_js::CommandDetectionConfig::new(
        &loctree_config.tauri.dom_exclusions,
        &loctree_config.tauri.non_invoke_exclusions,
        &loctree_config.tauri.invalid_command_names,
    );

    let scan_config = ScanConfig {
        roots: root_list,
        parsed: &parsed,
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

    // Finish spinner with file count
    let file_count: usize = scan_results.contexts.iter().map(|c| c.analyses.len()).sum();
    spinner.finish_success(&format!(
        "Scanned {} in {:.2}s",
        crate::progress::format_count(file_count, "file", "files"),
        start_time.elapsed().as_secs_f64()
    ));

    // Second spinner for building snapshot (can take a while for large codebases)
    let build_spinner = crate::progress::Spinner::new("Building snapshot...");

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

    // Summarize manifests and derive crate roots (ScanOnce -> SliceMany)
    let mut manifest_summary = Vec::new();
    let mut rust_crate_roots = Vec::new();
    for root in root_list {
        let summary = crate::analyzer::manifests::summarize_manifests(root);
        if let Some(cargo) = &summary.cargo_toml {
            rust_crate_roots.extend(cargo.crate_roots.clone());
        }
        manifest_summary.push(summary);
    }
    rust_crate_roots.sort();
    rust_crate_roots.dedup();
    snapshot.metadata.manifest_summary = manifest_summary;

    // Aggregate detected entrypoints for fast lookup
    let entrypoints = crate::analyzer::entrypoints::find_entrypoints(&snapshot.files)
        .into_iter()
        .map(|(path, kinds)| EntrypointSummary { path, kinds })
        .collect();
    snapshot.metadata.entrypoints = entrypoints;
    snapshot.metadata.entrypoint_drift = compute_entrypoint_drift(
        &snapshot.metadata.manifest_summary,
        &snapshot.metadata.entrypoints,
    );

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

    // Helper to check if a file is frontend code (TypeScript/JavaScript)
    let is_frontend_file = |path: &str| {
        snapshot
            .files
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.language == "typescript" || f.language == "javascript")
            .unwrap_or(false)
    };

    for event_name in all_events {
        let emits = event_emits_map
            .get(&event_name)
            .cloned()
            .unwrap_or_default();
        let listens = event_listens_map
            .get(&event_name)
            .cloned()
            .unwrap_or_default();

        // Detect FE↔FE sync pattern:
        // 1. Has both emits and listens
        // 2. All emits are from frontend files
        // 3. All listens are from frontend files
        // 4. No Rust involvement (Rust files would have "rust" language)
        let has_emit = !emits.is_empty();
        let has_listen = !listens.is_empty();
        let all_emits_fe = emits.iter().all(|(path, _, _)| is_frontend_file(path));
        let all_listens_fe = listens.iter().all(|(path, _)| is_frontend_file(path));
        let is_fe_sync = has_emit && has_listen && all_emits_fe && all_listens_fe;

        // Check if emit and listen are in the same file (strongest indicator)
        let same_file_sync = if is_fe_sync {
            let emit_files: HashSet<&str> =
                emits.iter().map(|(path, _, _)| path.as_str()).collect();
            let listen_files: HashSet<&str> =
                listens.iter().map(|(path, _)| path.as_str()).collect();
            !emit_files.is_disjoint(&listen_files)
        } else {
            false
        };

        snapshot.event_bridges.push(EventBridge {
            name: event_name.clone(),
            emits,
            listens,
            is_fe_sync,
            same_file_sync,
        });
    }

    // Store resolver configuration from scan results for caching
    if scan_results.ts_resolver_config.is_some()
        || !scan_results.py_roots.is_empty()
        || !rust_crate_roots.is_empty()
    {
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
            rust_crate_roots,
        });
    }

    // Finalize metadata
    let duration_ms = start_time.elapsed().as_millis() as u64;
    snapshot.finalize_metadata(duration_ms);

    // Finish build spinner
    build_spinner.finish_clear();

    // Save snapshot
    snapshot.save(&snapshot_root)?;

    // Print summary (unless quiet mode)
    if !quiet_summary {
        snapshot.print_summary(&snapshot_root);
    }

    // Auto mode: emit full artifact set into the artifact directory (global cache by default)
    if parsed.auto_outputs {
        let artifacts_spinner = crate::progress::Spinner::new("Generating artifacts...");
        match write_auto_artifacts(
            &snapshot_root,
            root_list,
            &scan_results,
            &parsed,
            Some(&snapshot.metadata),
        ) {
            Ok(paths) => {
                artifacts_spinner.finish_clear();
                if !paths.is_empty() {
                    eprintln!(
                        "Artifacts saved under {}:",
                        Snapshot::artifacts_dir(&snapshot_root).display()
                    );
                    for p in paths {
                        eprintln!("  - {}", p);
                    }
                }
            }
            Err(err) => {
                artifacts_spinner.finish_error("Failed to generate artifacts");
                eprintln!("[loctree][warn] failed to write auto artifacts: {}", err);
            }
        }
    }

    Ok(())
}

/// Run the init command: scan the project and save snapshot
///
/// This is a convenience wrapper around `run_init_with_options` with default behavior
/// (prints summary). For internal scans that should be quiet, use `run_init_with_options` directly.
pub fn run_init(root_list: &[PathBuf], parsed: &ParsedArgs) -> io::Result<()> {
    run_init_with_options(root_list, parsed, false)
}

/// In auto mode, generate the full set of analysis artifacts in the artifact directory.
pub(crate) fn write_auto_artifacts(
    snapshot_root: &Path,
    roots: &[PathBuf],
    scan_results: &crate::analyzer::root_scan::ScanResults,
    parsed: &ParsedArgs,
    metadata_override: Option<&SnapshotMetadata>,
) -> io::Result<Vec<String>> {
    use crate::analyzer::coverage::{
        CommandUsage, compute_command_gaps_with_confidence, compute_unregistered_handlers,
    };
    use crate::analyzer::cycles::find_cycles_with_lazy;
    use crate::analyzer::dead_parrots::find_dead_exports;
    use crate::analyzer::output::{RootArtifacts, process_root_context, write_report};
    use crate::analyzer::pipelines::build_pipeline_summary;
    use crate::analyzer::sarif::{SarifInputs, generate_sarif_string};
    use crate::analyzer::scan::opt_globset;
    use serde_json::json;

    const DEFAULT_EXCLUDE_REPORT_PATTERNS: &[&str] =
        &["**/__tests__/**", "scripts/semgrep-fixtures/**"];
    const SCHEMA_NAME: &str = "loctree-json";
    const SCHEMA_VERSION: &str = SNAPSHOT_SCHEMA_VERSION;

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
    let (cycles, lazy_cycles) = find_cycles_with_lazy(&all_graph_edges);
    write_atomic(
        &circular_json_path,
        serde_json::to_string_pretty(&json!({
            "circularImports": cycles,
            "lazyCircularImports": lazy_cycles
        }))
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
    write_atomic(
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

    let mut languages: Vec<String> = scan_results
        .contexts
        .iter()
        .flat_map(|ctx| ctx.languages.iter().cloned())
        .collect();
    languages.sort();
    languages.dedup();
    let total_loc: usize = scan_results.global_analyses.iter().map(|a| a.loc).sum();
    let file_count = scan_results.global_analyses.len();

    let entrypoint_drift = if let Some(meta) = metadata_override {
        meta.entrypoint_drift.clone()
    } else {
        let manifest_summary: Vec<ManifestSummary> = roots
            .iter()
            .map(|root| crate::analyzer::manifests::summarize_manifests(root))
            .collect();
        let entrypoints =
            crate::analyzer::entrypoints::find_entrypoints(&scan_results.global_analyses)
                .into_iter()
                .map(|(path, kinds)| EntrypointSummary { path, kinds })
                .collect::<Vec<_>>();
        compute_entrypoint_drift(&manifest_summary, &entrypoints)
    };

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
        "stats": {
            "files": file_count,
            "loc": total_loc,
            "languages": languages,
        },
        "analysis": json_results,
        "pipelineSummary": pipeline_summary,
        "circularImports": cycles,
        "pyRaceIndicators": race_items,
        "entrypointDrift": entrypoint_drift,
    });
    write_atomic(
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
    let mut dead_ok_globs = crate::fs_utils::load_loctignore_dead_ok_globs(snapshot_root);
    dead_ok_globs.sort();
    dead_ok_globs.dedup();
    let dead_exports = find_dead_exports(
        &scan_results.global_analyses,
        high_confidence,
        None,
        crate::analyzer::dead_parrots::DeadFilterConfig {
            include_tests: false,
            include_helpers: false,
            library_mode: parsed.library_mode,
            example_globs: parsed.library_example_globs.clone(),
            python_library_mode: parsed.python_library,
            include_ambient: false,
            include_dynamic: false,
            dead_ok_globs,
        },
    );

    // Build minimal snapshot for SARIF enrichment and findings analysis
    let minimal_snapshot = Snapshot {
        metadata: metadata_override
            .cloned()
            .unwrap_or_else(|| SnapshotMetadata {
                roots: vec![snapshot_root.to_string_lossy().to_string()],
                entrypoint_drift: entrypoint_drift.clone(),
                ..Default::default()
            }),
        files: scan_results.global_analyses.clone(),
        edges: all_graph_edges
            .iter()
            .map(|(from, to, label)| GraphEdge {
                from: from.clone(),
                to: to.clone(),
                label: label.clone(),
            })
            .collect(),
        export_index: Default::default(),
        command_bridges: vec![],
        event_bridges: vec![],
        barrels: vec![],
    };

    let sarif_content = generate_sarif_string(SarifInputs {
        duplicate_exports: &all_ranked_dups,
        missing_handlers: &global_missing_handlers,
        unused_handlers: &global_unused_handlers,
        dead_exports: &dead_exports,
        circular_imports: &cycles,
        pipeline_summary: &pipeline_summary,
        snapshot: Some(&minimal_snapshot),
    })
    .map_err(|err| io::Error::other(format!("Failed to serialize SARIF: {err}")))?;
    write_atomic(&sarif_path, sarif_content)?;
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
    write_atomic(
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
                "why": format!("Frontend calls invoke('{}') but no #[tauri::command] handler found", gap.name),
                "impact": "Runtime error: 'command {} not found' when invoked from frontend",
                "suggestedFix": "Create handler with #[tauri::command] and register in invoke_handler![]",
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
                "why": format!("Handler '{}' is registered but no invoke() calls found in frontend", gap.name),
                "impact": "Dead code - handler exists but is never called",
                "suggestedFix": "If intentionally unused (e.g., for tests), ignore. Otherwise, remove handler.",
            })
        }).collect::<Vec<_>>(),
        "unregisteredHandlers": global_unregistered_handlers.iter().map(|gap| {
            json!({
                "handler": gap.name,
                "implementationName": gap.implementation_name,
                "locations": gap.locations.iter().map(|(path, line)| {
                    json!({ "path": path, "line": line })
                }).collect::<Vec<_>>(),
                "why": format!("#[tauri::command] fn {}() found but NOT in invoke_handler![] macro", gap.name),
                "impact": "Command exists but is unreachable from frontend - invoke() calls will fail",
                "suggestedFix": "Add to invoke_handler![] in main.rs or lib.rs, or remove if unused",
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "missing": global_missing_handlers.len(),
            "unused": global_unused_handlers.len(),
            "unregistered": global_unregistered_handlers.len(),
        },
    });
    write_atomic(
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

    // Save findings.json - consolidated issue report
    let findings_json_path = loctree_dir.join("findings.json");
    let findings_config = crate::analyzer::findings::FindingsConfig {
        high_confidence,
        library_mode: parsed.library_mode,
        python_library: parsed.python_library,
        example_globs: parsed.library_example_globs.clone(),
    };
    let findings = crate::analyzer::findings::Findings::produce(
        scan_results,
        &minimal_snapshot,
        findings_config,
    );
    let findings_json = findings.to_json().map_err(io::Error::other)?;
    write_atomic(&findings_json_path, &findings_json)?;
    created.push(format!(
        "./{}",
        findings_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&findings_json_path)
            .display()
    ));

    // Save agent.json - AI-optimized bundle (used by CI and agent tooling)
    let agent_json_path = loctree_dir.join("agent.json");
    let agent_report = crate::analyzer::for_ai::generate_for_ai_report(
        &snapshot_root.to_string_lossy(),
        &report_sections,
        &scan_results.global_analyses,
        Some(&minimal_snapshot),
    );
    let agent_json = serde_json::to_vec_pretty(&agent_report).map_err(io::Error::other)?;
    write_atomic(&agent_json_path, &agent_json)?;
    created.push(format!(
        "./{}",
        agent_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&agent_json_path)
            .display()
    ));

    // Save manifest.json - index of artifacts for AI agents
    let manifest_json_path = loctree_dir.join("manifest.json");
    let findings_size_kb = findings_json.len() / 1024;
    let agent_size_kb = agent_json.len() / 1024;
    let manifest = crate::analyzer::findings::Manifest::produce(
        &minimal_snapshot,
        findings_size_kb,
        agent_size_kb,
    );
    let manifest_json = manifest.to_json().map_err(io::Error::other)?;
    write_atomic(&manifest_json_path, &manifest_json)?;
    created.push(format!(
        "./{}",
        manifest_json_path
            .strip_prefix(snapshot_root)
            .unwrap_or(&manifest_json_path)
            .display()
    ));

    // Now that the full artifact set exists, refresh stable pointers (base_dir/*.json + base_dir/latest/).
    // Snapshot::save() runs this before auto artifacts are generated, so we do it again here.
    if let Err(e) = Snapshot::refresh_latest_artifacts(snapshot_root) {
        eprintln!("[loctree][warn] failed to refresh latest pointers: {}", e);
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct DirGuard {
        path: PathBuf,
    }

    impl DirGuard {
        fn new(path: PathBuf) -> Self {
            Self { path }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

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
        // Snapshot path should be under the global cache, not project-local
        assert!(path.ends_with("snapshot.json"));
        // Should NOT be under the project directory anymore
        assert!(
            !path.starts_with("/some/project/.loctree"),
            "snapshot should go to global cache, not project-local .loctree"
        );
    }

    #[test]
    fn test_snapshot_path_uses_root_git_context() {
        // Non-git directory should use legacy path (no scan_id)
        let path = Snapshot::snapshot_path(Path::new("/tmp/loctree"));
        assert!(
            path.ends_with("snapshot.json"),
            "should end with snapshot.json"
        );
        // Git directory (cwd) should include scan_id
        let cwd = std::env::current_dir().unwrap();
        let ctx = Snapshot::git_context_for(&cwd);
        if let Some(scan) = ctx.scan_id {
            let path = Snapshot::snapshot_path(&cwd);
            assert!(
                path.display().to_string().contains(&scan),
                "git dir should include scan_id"
            );
        }
    }

    #[test]
    fn test_artifacts_dir_uses_root_git_context() {
        // Non-git directory should use global cache
        let dir = Snapshot::artifacts_dir(Path::new("/tmp/loctree"));
        assert!(
            !dir.starts_with("/tmp/loctree/.loctree"),
            "artifacts should go to global cache, not project-local"
        );
        // Git directory should include scan_id
        let cwd = std::env::current_dir().unwrap();
        let ctx = Snapshot::git_context_for(&cwd);
        if let Some(scan) = ctx.scan_id {
            let dir = Snapshot::artifacts_dir(&cwd);
            assert!(
                dir.display().to_string().contains(&scan),
                "git dir should include scan_id"
            );
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
            is_fe_sync: false,
            same_file_sync: false,
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
            schema_version: SNAPSHOT_SCHEMA_VERSION.to_string(),
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
            manifest_summary: Vec::new(),
            entrypoints: Vec::new(),
            entrypoint_drift: EntrypointDriftSummary::default(),
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

    #[test]
    fn test_find_latest_snapshot_explicit_path_exists() {
        let tmp = TempDir::new().expect("create temp dir");
        let snapshot_path = tmp.path().join(SNAPSHOT_DIR).join(SNAPSHOT_FILE);

        // Create snapshot directory and file
        std::fs::create_dir_all(snapshot_path.parent().unwrap()).expect("create dir");
        let snapshot = Snapshot::new(vec!["src".to_string()]);
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        std::fs::write(&snapshot_path, json).expect("write snapshot");

        // Should return the explicit path
        let result = Snapshot::find_latest_snapshot(Some(&snapshot_path));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), snapshot_path);
    }

    #[test]
    fn test_find_latest_snapshot_explicit_path_not_exists() {
        let result =
            Snapshot::find_latest_snapshot(Some(Path::new("/nonexistent/path/snapshot.json")));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Snapshot not found"));
        assert!(err.contains("Run `loct scan` first"));
    }

    #[test]
    fn test_find_latest_snapshot_picks_newest_by_mtime() {
        let tmp = TempDir::new().expect("create temp dir");
        let loctree_dir = tmp.path().join(SNAPSHOT_DIR);

        // Create two branch@sha subdirectories with snapshots
        let old_dir = loctree_dir.join("main@old123");
        let new_dir = loctree_dir.join("main@new456");
        std::fs::create_dir_all(&old_dir).expect("create old dir");
        std::fs::create_dir_all(&new_dir).expect("create new dir");

        let old_snapshot_path = old_dir.join(SNAPSHOT_FILE);
        let new_snapshot_path = new_dir.join(SNAPSHOT_FILE);

        // Write old snapshot first
        let snapshot = Snapshot::new(vec!["src".to_string()]);
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        std::fs::write(&old_snapshot_path, &json).expect("write old snapshot");

        // Wait a tiny bit to ensure mtime difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Write new snapshot
        std::fs::write(&new_snapshot_path, &json).expect("write new snapshot");

        // Use find_latest_snapshot_in to avoid changing global cwd (thread-safe)
        let result = Snapshot::find_latest_snapshot_in(tmp.path());

        assert!(result.is_ok());
        let found_path = result.unwrap();
        // Should find the newer snapshot
        assert!(
            found_path.to_string_lossy().contains("new456"),
            "Expected newest snapshot, got: {}",
            found_path.display()
        );
    }

    #[test]
    fn test_find_latest_snapshot_no_loctree_dir() {
        let tmp = TempDir::new().expect("create temp dir");
        // No .loctree directory and no cache entry

        // Use find_latest_snapshot_in to avoid changing global cwd (thread-safe)
        let result = Snapshot::find_latest_snapshot_in(tmp.path());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("No snapshot found"),
            "Expected 'No snapshot found' in error: {}",
            err
        );
    }

    #[test]
    fn test_find_latest_snapshot_empty_loctree_dir() {
        let tmp = TempDir::new().expect("create temp dir");
        // Create empty .loctree directory (no snapshots)
        std::fs::create_dir(tmp.path().join(SNAPSHOT_DIR)).expect("create .loctree");

        // Use find_latest_snapshot_in to avoid changing global cwd (thread-safe)
        let result = Snapshot::find_latest_snapshot_in(tmp.path());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("No snapshot found"),
            "Expected 'No snapshot found' in error: {}",
            err
        );
    }

    #[test]
    fn test_find_latest_snapshot_legacy_path() {
        let tmp = TempDir::new().expect("create temp dir");
        let loctree_dir = tmp.path().join(SNAPSHOT_DIR);

        // Create legacy snapshot at .loctree/snapshot.json (not in subdirectory)
        std::fs::create_dir_all(&loctree_dir).expect("create .loctree dir");
        let legacy_path = loctree_dir.join(SNAPSHOT_FILE);

        let snapshot = Snapshot::new(vec!["src".to_string()]);
        let json = serde_json::to_string_pretty(&snapshot).unwrap();
        std::fs::write(&legacy_path, json).expect("write legacy snapshot");

        // Use find_latest_snapshot_in to avoid changing global cwd (thread-safe)
        let result = Snapshot::find_latest_snapshot_in(tmp.path());

        assert!(result.is_ok());
        // Compare canonicalized paths to handle /private/var vs /var on macOS
        let found = result.unwrap().canonicalize().unwrap_or_default();
        let expected = legacy_path.canonicalize().unwrap_or_default();
        assert_eq!(found, expected);
    }

    #[test]
    fn test_find_latest_snapshot_global_cache_from_subdir() {
        let project = TempDir::new().expect("create temp project dir");
        let nested = project.path().join("a/b/c");
        std::fs::create_dir_all(&nested).expect("create nested dirs");

        // Ensure we don't leave cache artifacts around after this test.
        let _cleanup = DirGuard::new(project_cache_dir(project.path()));

        // Save snapshot for project root -> goes to global cache (or temp dir fallback)
        let snapshot = Snapshot::new(vec!["src".to_string()]);
        snapshot.save(project.path()).expect("save snapshot");

        // Discover from nested subdir: should resolve effective root via global cache
        let found = Snapshot::find_latest_snapshot_in(&nested).expect("find snapshot");
        let expected = Snapshot::snapshot_path(project.path());

        // Compare canonicalized paths to handle /private/var vs /var on macOS
        let found = found.canonicalize().unwrap_or(found);
        let expected = expected.canonicalize().unwrap_or(expected);
        assert_eq!(found, expected);
    }
}
