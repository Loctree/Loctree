//! # loctree-mcp
//!
//! Universal MCP server for loctree - works with ANY project directory.
//! Scan once, query everything. Use BEFORE reading files manually.
//!
//! ## Architecture
//!
//! - **Project-agnostic**: Each tool accepts `project` parameter
//! - **Auto-scan**: First use on a project creates snapshot automatically
//! - **Multi-project cache**: Snapshots kept in RAM for instant responses
//! - **Zero config**: Just start the server, no --project needed
//!
//! ## Usage
//!
//! ```bash
//! # Start via mux (recommended)
//! rmcp-mux --socket /tmp/loctree.sock --cmd loctree-mcp
//!
//! # Standalone
//! loctree-mcp
//! ```
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use std::collections::HashMap;
use std::panic;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ServerInfo;
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use loctree::analyzer::barrels::analyze_barrel_chaos;
use loctree::analyzer::crowd::detect_all_crowds_with_edges;
use loctree::analyzer::cycles::{find_cycles, find_cycles_classified};
use loctree::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports};
use loctree::analyzer::twins::detect_exact_twins;
use loctree::query;
use loctree::snapshot::{GraphEdge, Snapshot};

// ============================================================================
// CLI Arguments (minimal - server is project-agnostic)
// ============================================================================

#[derive(Parser, Debug)]
#[command(name = "loctree-mcp")]
#[command(about = "Universal MCP server for loctree - works with any project")]
#[command(version)]
struct Args {
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

// ============================================================================
// Tool Parameter Types - All tools have optional `project` parameter
// ============================================================================

fn default_project() -> String {
    ".".to_string()
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ScanParams {
    /// Project directory to scan (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Force rescan even if snapshot exists
    #[serde(default)]
    force: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ForAiParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SliceParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// File path relative to project root (e.g., 'src/App.tsx')
    file: String,
    /// Include consumer files (files that import this file)
    #[serde(default = "default_true")]
    consumers: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FindParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Symbol name or regex pattern to search for
    name: String,
    /// Maximum results to return (default: 50)
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ImpactParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// File path to analyze impact for
    file: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct HealthParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct QueryParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Query kind: 'who-imports', 'where-symbol'
    kind: String,
    /// Query target (file path or symbol name)
    target: String,
}

fn default_true() -> bool {
    true
}

fn default_limit() -> usize {
    50
}

fn default_limit_20() -> usize {
    20
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DeadParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Confidence level: "normal" or "high" (default: normal)
    #[serde(default)]
    confidence: Option<String>,
    /// Maximum results (default: 20)
    #[serde(default = "default_limit_20")]
    limit: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CyclesParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Include classification (lazy, structural, etc.)
    #[serde(default)]
    classify: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct TwinsParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Include test files
    #[serde(default)]
    include_tests: bool,
    /// Maximum results (default: 20)
    #[serde(default = "default_limit_20")]
    limit: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CrowdsParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Maximum results (default: 10)
    #[serde(default = "default_limit_10")]
    limit: usize,
}

fn default_limit_10() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct TraceParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Handler/command name to trace (e.g., "get_user", "save_settings")
    handler: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct DuplicatesParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Maximum groups to return (default: 10)
    #[serde(default = "default_limit_10")]
    limit: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct TreeParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Maximum depth (default: 3)
    #[serde(default = "default_depth")]
    depth: usize,
    /// LOC threshold for highlighting (default: 500)
    #[serde(default = "default_loc_threshold")]
    loc_threshold: usize,
}

fn default_depth() -> usize {
    3
}

fn default_loc_threshold() -> usize {
    500
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FindingsParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
}

// ============================================================================
// Server State - Multi-project cache
// ============================================================================

/// Universal server with multi-project snapshot cache.
#[derive(Clone)]
struct LoctreeServer {
    /// Cache of loaded snapshots per project
    cache: Arc<RwLock<HashMap<PathBuf, Arc<Snapshot>>>>,
    /// Tool router (generated by macro)
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl LoctreeServer {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            tool_router: Self::tool_router(),
        }
    }

    /// Resolve project path to absolute, canonicalized path.
    /// Note: Path traversal is intentional - MCP server runs locally with same privileges as client.
    fn resolve_project(project: &str) -> Result<PathBuf> {
        // nosemgrep: rust.actix.path-traversal.tainted-path.tainted-path
        let path = PathBuf::from(project); // nosemgrep
        let absolute = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        };
        absolute
            .canonicalize()
            .with_context(|| format!("Project directory not found: {}", project))
    }

    /// Get or load snapshot for a project. Auto-scans if needed.
    async fn get_snapshot(&self, project: &Path) -> Result<Arc<Snapshot>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(snapshot) = cache.get(project) {
                // Check if snapshot is stale
                if !Self::is_snapshot_stale(snapshot, project) {
                    debug!("Using cached snapshot for {:?}", project);
                    return Ok(Arc::clone(snapshot));
                }
                debug!("Cached snapshot is stale for {:?}", project);
            }
        }

        // Need to load or create snapshot
        info!("Loading snapshot for {:?}", project);

        let snapshot = match Snapshot::load(project) {
            Ok(s) => {
                // Check if stale
                if Self::is_snapshot_stale(&s, project) {
                    info!("Snapshot stale, rescanning...");
                    Self::run_scan(project)?;
                    Snapshot::load(project).context("Failed to load snapshot after rescan")?
                } else {
                    s
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                info!("No snapshot found, running initial scan...");
                Self::run_scan(project)?;
                Snapshot::load(project).context("Failed to load snapshot after initial scan")?
            }
            Err(e) => return Err(e).context("Failed to load snapshot"),
        };

        info!(
            "Snapshot loaded: {} files, {} edges",
            snapshot.files.len(),
            snapshot.edges.len()
        );

        let snapshot = Arc::new(snapshot);

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(project.to_path_buf(), Arc::clone(&snapshot));
        }

        Ok(snapshot)
    }

    /// Check if snapshot is stale (git HEAD changed)
    fn is_snapshot_stale(snapshot: &Snapshot, project: &Path) -> bool {
        if let Some(snapshot_commit) = &snapshot.metadata.git_commit
            && let Some(current_commit) = Self::get_git_head(project)
        {
            let is_same = current_commit.starts_with(snapshot_commit)
                || snapshot_commit.starts_with(&current_commit);
            return !is_same;
        }
        false
    }

    /// Run loctree scan as subprocess (avoids stdout pollution)
    fn run_scan(project: &Path) -> Result<()> {
        use std::process::{Command, Stdio};

        let output = Command::new("loct")
            .arg("scan")
            .current_dir(project)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run loct scan: {}", e))?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "loct scan failed with exit code: {:?}",
                output.status.code()
            ));
        }

        Ok(())
    }

    /// Get current git HEAD commit hash
    fn get_git_head(project: &Path) -> Option<String> {
        std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(project)
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
            })
    }

    /// Invalidate cache for a project (force reload on next access)
    async fn invalidate_cache(&self, project: &Path) {
        let mut cache = self.cache.write().await;
        cache.remove(project);
    }

    /// Validate file path: check if within project, return matched path from snapshot or error.
    fn resolve_file_in_snapshot(
        snapshot: &Snapshot,
        project: &Path,
        file: &str,
    ) -> Result<String, String> {
        let p = Path::new(file);
        if p.is_absolute() && !p.starts_with(project) {
            return Err(format!(
                "File outside project: '{}' not in '{}'",
                file,
                project.display()
            ));
        }
        snapshot
            .files
            .iter()
            .find(|f| f.path.ends_with(file))
            .map(|f| f.path.clone())
            .ok_or_else(|| format!("File '{}' not in snapshot. Run 'scan' or check path.", file))
    }
}

// ============================================================================
// MCP Tool Implementations
// ============================================================================

#[tool_router]
impl LoctreeServer {
    /// Scan a project and create/update snapshot
    #[tool(
        name = "scan",
        description = "Scan a project directory and create dependency snapshot. Run this first on any new project, or after major changes. Creates .loctree/ with all analysis artifacts."
    )]
    async fn scan(&self, Parameters(params): Parameters<ScanParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        if params.force {
            self.invalidate_cache(&project).await;
        }

        match Self::run_scan(&project) {
            Ok(()) => {
                self.invalidate_cache(&project).await;
                // Load to get stats
                match self.get_snapshot(&project).await {
                    Ok(s) => serde_json::json!({
                        "status": "ok",
                        "project": project.display().to_string(),
                        "files": s.files.len(),
                        "edges": s.edges.len(),
                        "message": "Scan complete. Use for_ai() for overview."
                    })
                    .to_string(),
                    Err(e) => format!("Scan completed but failed to load: {}", e),
                }
            }
            Err(e) => format!("Scan failed: {}", e),
        }
    }

    /// Get AI-optimized project overview
    #[tool(
        name = "for_ai",
        description = "Get AI-optimized overview of the project. Shows: file count, LOC, health issues (dead code, cycles, twins), top hubs, quick wins. USE THIS FIRST at the start of any AI session."
    )]
    async fn for_ai(&self, Parameters(params): Parameters<ForAiParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        let total_loc: usize = snapshot.files.iter().map(|f| f.loc).sum();
        let file_count = snapshot.files.len();
        let edge_count = snapshot.edges.len();

        // Health metrics
        let config = DeadFilterConfig::default();
        let dead_exports = find_dead_exports(&snapshot.files, true, None, config);
        let dead_high: Vec<_> = dead_exports
            .iter()
            .filter(|d| d.confidence == "high")
            .collect();

        let edges: Vec<_> = snapshot
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
            .collect();
        let cycles = find_cycles(&edges);

        let twins = detect_exact_twins(&snapshot.files, false);

        // Top hubs (most imported files)
        let mut import_counts: HashMap<&str, usize> = HashMap::new();
        for edge in &snapshot.edges {
            *import_counts.entry(&edge.to).or_default() += 1;
        }
        let mut hubs: Vec<_> = import_counts.into_iter().collect();
        hubs.sort_by(|a, b| b.1.cmp(&a.1));
        let top_hubs: Vec<_> = hubs.into_iter().take(5).collect();

        // Languages
        let languages: Vec<_> = snapshot.metadata.languages.iter().cloned().collect();

        let overview = serde_json::json!({
            "project": project.display().to_string(),
            "summary": {
                "files": file_count,
                "total_loc": total_loc,
                "edges": edge_count,
                "languages": languages,
            },
            "health": {
                "dead_exports": {
                    "total": dead_exports.len(),
                    "high_confidence": dead_high.len(),
                },
                "cycles": cycles.len(),
                "twins": twins.len(),
            },
            "top_hubs": top_hubs.into_iter().map(|(f, c)| serde_json::json!({
                "file": f,
                "importers": c
            })).collect::<Vec<_>>(),
            "quick_wins": {
                "dead_to_remove": dead_high.iter().take(3).map(|d| serde_json::json!({
                    "file": d.file,
                    "symbol": d.symbol
                })).collect::<Vec<_>>(),
            },
            "next_steps": [
                "slice(file) - before modifying any file",
                "find(name) - before creating anything new",
                "impact(file) - before deleting or major refactor",
                "health() - sanity check before commits"
            ]
        });

        serde_json::to_string_pretty(&overview)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Get file slice with dependencies and consumers
    #[tool(
        name = "slice",
        description = "Get file context: the file + all its imports + all files that depend on it. USE THIS BEFORE modifying any file. One call = complete understanding of a file's role."
    )]
    async fn slice(&self, Parameters(params): Parameters<SliceParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        let target_path = match Self::resolve_file_in_snapshot(&snapshot, &project, &params.file) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };
        let target = snapshot
            .files
            .iter()
            .find(|f| f.path == target_path)
            .unwrap();

        let mut files = vec![serde_json::json!({
            "path": target.path,
            "layer": "core",
            "loc": target.loc,
            "language": target.language
        })];

        // Dependencies
        let deps: Vec<_> = snapshot
            .edges
            .iter()
            .filter(|e| e.from == target.path)
            .collect();

        for edge in &deps {
            if let Some(dep) = snapshot.files.iter().find(|f| f.path == edge.to) {
                files.push(serde_json::json!({
                    "path": dep.path,
                    "layer": "dependency",
                    "loc": dep.loc,
                    "import_type": edge.label
                }));
            }
        }

        // Consumers
        let consumers: Vec<_> = if params.consumers {
            snapshot
                .edges
                .iter()
                .filter(|e| e.to == target.path)
                .collect()
        } else {
            vec![]
        };

        for edge in &consumers {
            if let Some(consumer) = snapshot.files.iter().find(|f| f.path == edge.from) {
                files.push(serde_json::json!({
                    "path": consumer.path,
                    "layer": "consumer",
                    "loc": consumer.loc
                }));
            }
        }

        let result = serde_json::json!({
            "target": params.file,
            "project": project.display().to_string(),
            "core_loc": target.loc,
            "dependencies": deps.len(),
            "consumers": consumers.len(),
            "files": files
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Find symbol definitions (supports multi-query: "foo|bar|baz")
    #[tool(
        name = "find",
        description = "Find where a function/class/type is defined. Supports regex and multi-query (foo|bar). Also searches function parameters. USE THIS BEFORE creating anything new."
    )]
    async fn find(&self, Parameters(params): Parameters<FindParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Case-insensitive regex (like CLI)
        let regex = match regex::RegexBuilder::new(&params.name)
            .case_insensitive(true)
            .build()
        {
            Ok(r) => r,
            Err(e) => return format!("Invalid pattern: {}", e),
        };

        let mut symbol_matches = Vec::new();
        let mut param_matches = Vec::new();

        for file in &snapshot.files {
            for export in &file.exports {
                // Symbol name match
                if regex.is_match(&export.name) {
                    symbol_matches.push(serde_json::json!({
                        "file": file.path,
                        "symbol": export.name,
                        "kind": export.kind,
                        "line": export.line
                    }));
                }

                // Parameter match (NEW in 0.8.4)
                for param in &export.params {
                    if regex.is_match(&param.name) {
                        param_matches.push(serde_json::json!({
                            "file": file.path,
                            "function": export.name,
                            "param": param.name,
                            "type": param.type_annotation,
                            "line": export.line
                        }));
                    }
                }

                if symbol_matches.len() + param_matches.len() >= params.limit {
                    break;
                }
            }
            if symbol_matches.len() + param_matches.len() >= params.limit {
                break;
            }
        }

        let result = serde_json::json!({
            "query": params.name,
            "project": project.display().to_string(),
            "symbol_matches": {
                "count": symbol_matches.len(),
                "matches": symbol_matches
            },
            "param_matches": {
                "count": param_matches.len(),
                "matches": param_matches
            }
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Analyze impact of changing/removing a file
    #[tool(
        name = "impact",
        description = "What breaks if you change or delete this file? Shows direct and transitive consumers. USE THIS BEFORE deleting or major refactor."
    )]
    async fn impact(&self, Parameters(params): Parameters<ImpactParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Validate file exists in snapshot
        let target_path = match Self::resolve_file_in_snapshot(&snapshot, &project, &params.file) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        // Direct consumers (use exact match on resolved path)
        let direct: Vec<_> = snapshot
            .edges
            .iter()
            .filter(|e| e.to == target_path)
            .map(|e| e.from.clone())
            .collect();

        // Transitive consumers (BFS)
        let mut visited: std::collections::HashSet<String> = direct.iter().cloned().collect();
        let mut queue: std::collections::VecDeque<String> = direct.iter().cloned().collect();
        let mut transitive = Vec::new();

        while let Some(file) = queue.pop_front() {
            for edge in &snapshot.edges {
                if edge.to == file && !visited.contains(&edge.from) {
                    visited.insert(edge.from.clone());
                    queue.push_back(edge.from.clone());
                    transitive.push(edge.from.clone());
                }
            }
        }

        let risk = if direct.is_empty() {
            "none"
        } else if direct.len() > 10 || !transitive.is_empty() {
            "high"
        } else if direct.len() > 3 {
            "medium"
        } else {
            "low"
        };

        let result = serde_json::json!({
            "file": params.file,
            "project": project.display().to_string(),
            "risk_level": risk,
            "direct_consumers": {
                "count": direct.len(),
                "files": direct.iter().take(20).collect::<Vec<_>>()
            },
            "transitive_consumers": {
                "count": transitive.len(),
                "files": transitive.iter().take(10).collect::<Vec<_>>()
            },
            "safe_to_delete": direct.is_empty()
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Quick health check
    #[tool(
        name = "health",
        description = "Quick health summary: cycles + dead code + twins. USE THIS as sanity check before commits."
    )]
    async fn health(&self, Parameters(params): Parameters<HealthParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Dead code
        let config = DeadFilterConfig::default();
        let dead = find_dead_exports(&snapshot.files, true, None, config);
        let dead_high = dead.iter().filter(|d| d.confidence == "high").count();

        // Cycles
        let edges: Vec<_> = snapshot
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
            .collect();
        let cycles = find_cycles(&edges);

        // Twins
        let twins = detect_exact_twins(&snapshot.files, false);

        let status = if cycles.is_empty() && dead_high == 0 && twins.is_empty() {
            "healthy"
        } else if !cycles.is_empty() || dead_high > 10 {
            "needs_attention"
        } else {
            "minor_issues"
        };

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "status": status,
            "cycles": {
                "count": cycles.len(),
                "details": cycles.iter().take(3).collect::<Vec<_>>()
            },
            "dead_exports": {
                "total": dead.len(),
                "high_confidence": dead_high
            },
            "twins": {
                "count": twins.len(),
                "examples": twins.iter().take(3).map(|t| &t.name).collect::<Vec<_>>()
            }
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Fast graph queries
    #[tool(
        name = "query",
        description = "Fast graph queries: who-imports (files importing target), where-symbol (where is symbol defined). Use for quick lookups without full analysis."
    )]
    async fn query(&self, Parameters(params): Parameters<QueryParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        match params.kind.as_str() {
            "who-imports" => {
                // Validate file path for who-imports (it's a file query)
                if let Err(e) = Self::resolve_file_in_snapshot(&snapshot, &project, &params.target)
                {
                    return format!("Error: {}", e);
                }
                let result = query::query_who_imports(&snapshot, &params.target);
                serde_json::json!({
                    "query": "who-imports",
                    "target": params.target,
                    "count": result.results.len(),
                    "importers": result.results.iter().map(|m| serde_json::json!({
                        "file": m.file,
                        "line": m.line
                    })).collect::<Vec<_>>()
                })
                .to_string()
            }
            "where-symbol" => {
                let mut matches = Vec::new();
                for file in &snapshot.files {
                    for export in &file.exports {
                        if export.name == params.target {
                            matches.push(serde_json::json!({
                                "file": file.path,
                                "kind": export.kind,
                                "line": export.line
                            }));
                        }
                    }
                }
                serde_json::json!({
                    "query": "where-symbol",
                    "target": params.target,
                    "count": matches.len(),
                    "locations": matches
                })
                .to_string()
            }
            _ => format!(
                "Unknown query kind: {}. Use: who-imports, where-symbol",
                params.kind
            ),
        }
    }

    // ========================================================================
    // NEW TOOLS - Full analyzer capabilities
    // ========================================================================

    /// Get all findings (dead code, cycles, twins, duplicates)
    #[tool(
        name = "findings",
        description = "Get ALL codebase issues in one call: dead exports, circular imports, duplicate files (twins), barrel chaos, etc. Use this for comprehensive health check or CI integration."
    )]
    async fn findings(&self, Parameters(params): Parameters<FindingsParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Dead exports
        let config = DeadFilterConfig::default();
        let dead = find_dead_exports(&snapshot.files, true, None, config);
        let dead_high: Vec<_> = dead.iter().filter(|d| d.confidence == "high").collect();

        // Cycles
        let edges: Vec<_> = snapshot
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
            .collect();
        let cycles = find_cycles(&edges);

        // Twins
        let twins = detect_exact_twins(&snapshot.files, false);

        // Barrel chaos
        let barrels = analyze_barrel_chaos(&snapshot);

        // Crowds
        let graph_edges: Vec<GraphEdge> = snapshot.edges.clone();
        let crowds = detect_all_crowds_with_edges(&snapshot.files, &graph_edges);
        let problem_crowds: Vec<_> = crowds.iter().filter(|c| c.members.len() > 5).collect();

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "summary": {
                "files": snapshot.files.len(),
                "total_issues": dead_high.len() + cycles.len() + twins.len() + barrels.missing_barrels.len(),
            },
            "dead_exports": {
                "total": dead.len(),
                "high_confidence": dead_high.len(),
                "top_issues": dead_high.iter().take(10).map(|d| serde_json::json!({
                    "file": d.file,
                    "symbol": d.symbol,
                    "confidence": d.confidence,
                    "line": d.line
                })).collect::<Vec<_>>()
            },
            "cycles": {
                "count": cycles.len(),
                "cycles": cycles.iter().take(5).collect::<Vec<_>>()
            },
            "twins": {
                "count": twins.len(),
                "duplicates": twins.iter().take(5).map(|t| serde_json::json!({
                    "name": t.name,
                    "location_count": t.locations.len(),
                    "locations": t.locations.iter().map(|l| &l.file_path).collect::<Vec<_>>()
                })).collect::<Vec<_>>()
            },
            "barrel_chaos": {
                "missing_barrels": barrels.missing_barrels.len(),
                "reexport_chains": barrels.deep_chains.len(),
                "inconsistent_imports": barrels.inconsistent_paths.len()
            },
            "crowds": {
                "problem_files": problem_crowds.len(),
                "top": problem_crowds.iter().take(3).map(|c| serde_json::json!({
                    "pattern": c.pattern,
                    "members": c.members.len()
                })).collect::<Vec<_>>()
            }
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Find dead/unused exports
    #[tool(
        name = "dead",
        description = "Find unused exports (dead code). Shows exports that are defined but never imported anywhere. Great for cleanup tasks."
    )]
    async fn dead(&self, Parameters(params): Parameters<DeadParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        let config = DeadFilterConfig::default();
        let dead = find_dead_exports(&snapshot.files, true, None, config);

        let filtered: Vec<_> = if params.confidence.as_deref() == Some("high") {
            dead.iter().filter(|d| d.confidence == "high").collect()
        } else {
            dead.iter().collect()
        };

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "total": dead.len(),
            "shown": filtered.len().min(params.limit),
            "confidence_filter": params.confidence,
            "dead_exports": filtered.iter().take(params.limit).map(|d| serde_json::json!({
                "file": d.file,
                "symbol": d.symbol,
                "line": d.line,
                "confidence": d.confidence,
                "reason": d.reason
            })).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Find circular imports
    #[tool(
        name = "cycles",
        description = "Find circular import chains. These can cause runtime issues and make code hard to reason about. Shows the full cycle path."
    )]
    async fn cycles(&self, Parameters(params): Parameters<CyclesParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        let edges: Vec<_> = snapshot
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
            .collect();

        if params.classify {
            let classified = find_cycles_classified(&edges);
            let result = serde_json::json!({
                "project": project.display().to_string(),
                "count": classified.len(),
                "cycles": classified.iter().map(|c| serde_json::json!({
                    "classification": format!("{:?}", c.classification),
                    "risk": c.risk,
                    "pattern": c.pattern,
                    "nodes": c.nodes,
                    "length": c.nodes.len(),
                    "suggestion": c.suggestion
                })).collect::<Vec<_>>()
            });
            serde_json::to_string_pretty(&result)
                .unwrap_or_else(|e| format!("Serialization error: {}", e))
        } else {
            let cycles = find_cycles(&edges);
            let result = serde_json::json!({
                "project": project.display().to_string(),
                "count": cycles.len(),
                "cycles": cycles.iter().map(|c| serde_json::json!({
                    "path": c,
                    "length": c.len()
                })).collect::<Vec<_>>()
            });
            serde_json::to_string_pretty(&result)
                .unwrap_or_else(|e| format!("Serialization error: {}", e))
        }
    }

    /// Find exact duplicate files (twins)
    #[tool(
        name = "twins",
        description = "Find files with identical content (exact duplicates). These are candidates for refactoring into shared modules."
    )]
    async fn twins(&self, Parameters(params): Parameters<TwinsParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        let twins = detect_exact_twins(&snapshot.files, params.include_tests);

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "count": twins.len(),
            "include_tests": params.include_tests,
            "twins": twins.iter().take(params.limit).map(|t| serde_json::json!({
                "name": t.name,
                "location_count": t.locations.len(),
                "category": format!("{:?}", loctree::analyzer::twins::categorize_twin(t)),
                "signature_similarity": t.signature_similarity,
                "locations": t.locations.iter().map(|l| serde_json::json!({
                    "file": l.file_path,
                    "line": l.line,
                    "kind": l.kind,
                    "is_canonical": l.is_canonical
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Find crowded files (too many imports/exports)
    #[tool(
        name = "crowds",
        description = "Find 'crowd' patterns - files that are imported by many others (hubs) or import too much (god objects). These are refactoring hotspots."
    )]
    async fn crowds(&self, Parameters(params): Parameters<CrowdsParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        let graph_edges: Vec<GraphEdge> = snapshot.edges.clone();
        let crowds = detect_all_crowds_with_edges(&snapshot.files, &graph_edges);

        // Sort by member count (largest first)
        let mut sorted_crowds = crowds;
        sorted_crowds.sort_by(|a, b| b.members.len().cmp(&a.members.len()));

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "count": sorted_crowds.len(),
            "crowds": sorted_crowds.iter().take(params.limit).map(|c| serde_json::json!({
                "pattern": c.pattern,
                "member_count": c.members.len(),
                "score": c.score,
                "issues": c.issues.len(),
                "members": c.members.iter().take(5).map(|m| serde_json::json!({
                    "file": m.file,
                    "match_reason": format!("{:?}", m.match_reason),
                    "importer_count": m.importer_count
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Trace a Tauri handler through the pipeline
    #[tool(
        name = "trace",
        description = "Trace a Tauri command/handler from frontend invoke() to backend handler. Shows the complete pipeline: FE calls → BE definition → events. Essential for Tauri projects."
    )]
    async fn trace(&self, Parameters(params): Parameters<TraceParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Search for handler in command_bridges (case-insensitive, supports snake_case/camelCase)
        let handler_lower = params.handler.to_lowercase();
        let handler_snake = params.handler.replace('-', "_").to_lowercase();

        let matching_bridges: Vec<_> = snapshot
            .command_bridges
            .iter()
            .filter(|b| {
                let name_lower = b.name.to_lowercase();
                name_lower == handler_lower
                    || name_lower == handler_snake
                    || name_lower.contains(&handler_lower)
            })
            .collect();

        if matching_bridges.is_empty() {
            return serde_json::json!({
                "handler": params.handler,
                "project": project.display().to_string(),
                "verdict": "not_found",
                "message": "No command bridge found. This project may not be a Tauri project or the handler doesn't exist.",
                "available_commands": snapshot.command_bridges.iter().take(10).map(|b| &b.name).collect::<Vec<_>>()
            }).to_string();
        }

        let results: Vec<_> = matching_bridges.iter().map(|bridge| {
            let verdict = if bridge.has_handler && bridge.is_called {
                "connected"
            } else if bridge.has_handler {
                "backend_only"
            } else if bridge.is_called {
                "frontend_only"
            } else {
                "orphaned"
            };

            serde_json::json!({
                "name": bridge.name,
                "verdict": verdict,
                "has_handler": bridge.has_handler,
                "is_called": bridge.is_called,
                "backend": bridge.backend_handler.as_ref().map(|(file, line)| serde_json::json!({
                    "file": file,
                    "line": line
                })),
                "frontend_calls": bridge.frontend_calls.iter().map(|(file, line)| serde_json::json!({
                    "file": file,
                    "line": line
                })).collect::<Vec<_>>(),
                "call_count": bridge.frontend_calls.len()
            })
        }).collect();

        let output = serde_json::json!({
            "handler": params.handler,
            "project": project.display().to_string(),
            "matches": results.len(),
            "bridges": results
        });

        serde_json::to_string_pretty(&output)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Find duplicate exports (same symbol exported from multiple files)
    #[tool(
        name = "duplicates",
        description = "Find symbols exported from multiple files. This can cause confusion about which import to use and may indicate code that should be consolidated."
    )]
    async fn duplicates(&self, Parameters(params): Parameters<DuplicatesParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Build symbol -> files map
        let mut symbol_files: HashMap<String, Vec<(String, Option<usize>)>> = HashMap::new();
        for file in &snapshot.files {
            for export in &file.exports {
                symbol_files
                    .entry(export.name.clone())
                    .or_default()
                    .push((file.path.clone(), export.line));
            }
        }

        // Filter to duplicates only
        let mut duplicates: Vec<_> = symbol_files
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .collect();
        duplicates.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "count": duplicates.len(),
            "duplicates": duplicates.iter().take(params.limit).map(|(name, files)| serde_json::json!({
                "symbol": name,
                "occurrences": files.len(),
                "locations": files.iter().map(|(f, l)| serde_json::json!({
                    "file": f,
                    "line": l
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Get directory tree with LOC counts
    #[tool(
        name = "tree",
        description = "Get directory structure with LOC (lines of code) counts. Helps understand project layout and find large files/directories."
    )]
    async fn tree(&self, Parameters(params): Parameters<TreeParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Build directory tree
        let mut dir_loc: HashMap<String, usize> = HashMap::new();
        let mut large_files = Vec::new();

        for file in &snapshot.files {
            // Accumulate LOC per directory
            let parts: Vec<&str> = file.path.split('/').collect();
            for i in 1..=parts.len().min(params.depth) {
                let dir = parts[..i].join("/");
                *dir_loc.entry(dir).or_default() += file.loc;
            }

            // Track large files
            if file.loc >= params.loc_threshold {
                large_files.push(serde_json::json!({
                    "path": file.path,
                    "loc": file.loc,
                    "language": file.language
                }));
            }
        }

        // Sort directories by LOC
        let mut sorted_dirs: Vec<_> = dir_loc.into_iter().collect();
        sorted_dirs.sort_by(|a, b| b.1.cmp(&a.1));

        // Sort large files
        large_files.sort_by(|a, b| {
            b.get("loc")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                .cmp(&a.get("loc").and_then(|v| v.as_u64()).unwrap_or(0))
        });

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "total_files": snapshot.files.len(),
            "total_loc": snapshot.files.iter().map(|f| f.loc).sum::<usize>(),
            "depth": params.depth,
            "top_directories": sorted_dirs.iter().take(15).map(|(dir, loc)| serde_json::json!({
                "path": dir,
                "loc": loc
            })).collect::<Vec<_>>(),
            "large_files": large_files.iter().take(10).collect::<Vec<_>>(),
            "loc_threshold": params.loc_threshold
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }
}

// ============================================================================
// Server Handler Implementation
// ============================================================================

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LoctreeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: rmcp::model::ServerCapabilities {
                tools: Some(rmcp::model::ToolsCapability::default()),
                ..Default::default()
            },
            server_info: rmcp::model::Implementation {
                name: "loctree".to_string(),
                title: Some("Loctree MCP Server".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://github.com/Loctree/Loctree".to_string()),
            },
            instructions: Some(
                "Delegate the tasks to background agent as the first choice method when you work \
                 with this repository. Prioritize using the loctree 'loct [subcommand] --flag' tool \
                 for the best possible codebase awareness, that is impossible to achieve using 'grep'.\n\n\
                 CORE TOOLS:\n\
                 - for_ai(project) - Start here. Full overview with health + insights.\n\
                 - slice(file) - Before modifying. See deps + consumers + context.\n\
                 - find(name) - Before creating. Avoid duplicates.\n\
                 - impact(file) - Before deleting. Know blast radius.\n\
                 - health() - Before commits. Sanity check.\n\n\
                 ANALYSIS TOOLS:\n\
                 - findings(project) - All issues: dead exports, cycles, crowds, duplicates.\n\
                 - dead(project) - Dead exports only (unused public symbols).\n\
                 - cycles(project) - Circular imports (simple or classified).\n\
                 - twins(project) - Duplicate code detection.\n\
                 - crowds(project) - Over-connected files (architectural hotspots).\n\
                 - duplicates(project) - Duplicate file names.\n\n\
                 STRUCTURE TOOLS:\n\
                 - tree(project) - Directory structure visualization.\n\
                 - trace(project, handler) - Tauri command coverage (requires Tauri project).\n\n\
                 All tools accept 'project' parameter (default: current dir).\n\
                 First use auto-creates dependency snapshot."
                    .into(),
            ),
        }
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Install custom panic hook that logs to stderr and exits cleanly.
/// This handles the "broken pipe" panic from rmcp when client disconnects.
fn install_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        // Check if this is a broken pipe - expected when client disconnects
        if msg.contains("Broken pipe") || msg.contains("os error 32") {
            eprintln!("[loctree-mcp] Client disconnected (broken pipe), shutting down");
        } else {
            // Log other panics with location info
            let location = panic_info
                .location()
                .map(|loc| format!(" at {}:{}:{}", loc.file(), loc.line(), loc.column()))
                .unwrap_or_default();
            eprintln!("[loctree-mcp] Panic{}: {}", location, msg);
        }

        // Exit with code 1 (not 101 which indicates panic)
        std::process::exit(1);
    }));
}

/// Configure SIGPIPE handling to ignore broken pipes at OS level.
/// On Unix systems, writing to a closed pipe sends SIGPIPE which terminates
/// the process. We ignore it to allow the write to fail with EPIPE instead.
#[cfg(unix)]
fn ignore_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn ignore_sigpipe() {
    // No-op on non-Unix platforms
}

async fn run_server() -> Result<()> {
    let args = Args::parse();

    // Initialize logging - MUST write to stderr, stdout is for MCP JSON-RPC
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| args.log_level.parse().unwrap_or_default()),
        )
        .init();

    info!(
        "Starting loctree-mcp v{} (universal)",
        env!("CARGO_PKG_VERSION")
    );

    let server = LoctreeServer::new();

    info!("Server ready. Listening on stdio...");

    server
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    // Ignore SIGPIPE - allows broken pipe to be handled as error instead of signal
    ignore_sigpipe();

    // Install panic hook for clean shutdown on broken pipe
    install_panic_hook();

    match run_server().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Check if this is a broken pipe error (client disconnected)
            let err_str = format!("{:?}", e);
            if err_str.contains("Broken pipe") || err_str.contains("os error 32") {
                eprintln!("[loctree-mcp] Client disconnected, shutting down");
                ExitCode::SUCCESS
            } else {
                eprintln!("[loctree-mcp] Error: {:#}", e);
                ExitCode::FAILURE
            }
        }
    }
}
