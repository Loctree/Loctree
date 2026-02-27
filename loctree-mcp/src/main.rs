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
//! # Standalone
//! loctree-mcp
//! ```
//!
//! VibeCrafted with AI Agents (c)2026 Loctree Team

use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Write as _;
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

use loctree::analyzer::cycles::find_cycles;
use loctree::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports};
use loctree::analyzer::search::run_search;
use loctree::analyzer::twins::detect_exact_twins;
use loctree::snapshot::Snapshot;

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

/// Deserialize usize from either a number or a string (Claude Code sends strings).
mod deserialize_usize_lenient {
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<usize, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrNum {
            Num(usize),
            Str(String),
        }
        match StringOrNum::deserialize(deserializer)? {
            StringOrNum::Num(n) => Ok(n),
            StringOrNum::Str(s) => s
                .trim()
                .parse()
                .map_err(|_| serde::de::Error::custom(format!("invalid number: {s}"))),
        }
    }
}

fn default_project() -> String {
    // Normalize "." to absolute path - MCP server cwd may differ from agent cwd
    std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string())
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
    #[serde(
        default = "default_limit",
        deserialize_with = "deserialize_usize_lenient::deserialize"
    )]
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

fn default_true() -> bool {
    true
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct TreeParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Maximum depth (default: 3)
    #[serde(
        default = "default_depth",
        deserialize_with = "deserialize_usize_lenient::deserialize"
    )]
    depth: usize,
    /// LOC threshold for highlighting (default: 500)
    #[serde(default = "default_loc_threshold")]
    loc_threshold: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FocusParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// Directory to focus on (e.g., 'src/components')
    directory: String,
}

fn default_follow_scope() -> String {
    "all".to_string()
}

fn default_follow_limit() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FollowParams {
    /// Project directory (default: current directory)
    #[serde(default = "default_project")]
    project: String,
    /// What to follow: "dead", "cycles", "twins", "hotspots", or "all"
    #[serde(default = "default_follow_scope")]
    scope: String,
    /// Max trails to return per scope (default: 10)
    #[serde(
        default = "default_follow_limit",
        deserialize_with = "deserialize_usize_lenient::deserialize"
    )]
    limit: usize,
}

fn default_depth() -> usize {
    3
}

fn default_loc_threshold() -> usize {
    500
}

fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .take_while(|(left, right)| left == right)
        .count()
}

fn suggest_directories(snapshot: &Snapshot, query: &str, max: usize) -> Vec<String> {
    if max == 0 {
        return Vec::new();
    }

    let mut dirs = BTreeSet::new();
    for file in &snapshot.files {
        if let Some(parent) = Path::new(&file.path).parent() {
            let dir = parent.to_string_lossy().replace('\\', "/");
            if !dir.is_empty() && dir != "." {
                dirs.insert(dir);
            }
        }
    }

    if dirs.is_empty() {
        return Vec::new();
    }

    let normalized_query = query.trim().trim_matches('/');
    let query_lower = normalized_query.to_ascii_lowercase();
    let query_last = normalized_query
        .split('/')
        .rfind(|part| !part.is_empty())
        .unwrap_or(normalized_query);
    let query_last_lower = query_last.to_ascii_lowercase();
    let query_tokens: Vec<_> = query_lower
        .split(['/', '_', '-', '.'])
        .filter(|token| token.len() >= 2)
        .collect();

    let mut scored: Vec<(String, usize)> = dirs
        .iter()
        .map(|dir| {
            let dir_lower = dir.to_ascii_lowercase();
            let mut score = 0usize;

            if !query_last_lower.is_empty() && dir.contains(query_last) {
                score += 100;
            }
            if query_last_lower.len() > 2 && dir_lower.contains(&query_last_lower) {
                score += 50;
            }
            if !query_lower.is_empty() {
                score += common_prefix_len(&dir_lower, &query_lower) * 10;
            }
            for token in &query_tokens {
                if dir_lower.contains(token) {
                    score += 3;
                }
            }

            (dir.clone(), score)
        })
        .filter(|(_, score)| *score > 0)
        .collect();

    if scored.is_empty() {
        return dirs.into_iter().take(max).collect();
    }

    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scored.into_iter().take(max).map(|(dir, _)| dir).collect()
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
    /// The caller specifies the project root explicitly via --project,
    /// so we trust it as-is — no upward walk.  If the path doesn't have
    /// a snapshot yet, `get_snapshot()` will auto-scan it.
    /// Note: Path traversal is intentional - MCP server runs locally with same privileges as client.
    fn resolve_project(project: &str) -> Result<PathBuf> {
        // nosemgrep: rust.actix.path-traversal.tainted-path.tainted-path
        let path = PathBuf::from(project); // nosemgrep
        let absolute = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        };
        let canonical = absolute
            .canonicalize()
            .with_context(|| format!("Project directory not found: {}", project))?;

        Ok(canonical)
    }

    /// Get or load snapshot for a project. Auto-scans if needed.
    async fn get_snapshot(&self, project: &Path) -> Result<Arc<Snapshot>> {
        let cached_snapshot = {
            let cache = self.cache.read().await;
            cache.get(project).map(Arc::clone)
        };

        // Check cache first
        if let Some(snapshot) = cached_snapshot {
            // Check if snapshot is stale
            if !Self::is_snapshot_stale(&snapshot, project) {
                debug!("Using cached snapshot for {:?}", project);
                return Ok(snapshot);
            }
            debug!("Cached snapshot is stale for {:?}", project);
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

    /// Check if snapshot is stale (git HEAD changed OR dirty worktree).
    /// Delegates to `Snapshot::is_stale()` — single source of truth shared
    /// with CLI and LSP, covers both commit mismatch and uncommitted changes.
    fn is_snapshot_stale(snapshot: &Snapshot, project: &Path) -> bool {
        snapshot.is_stale(project)
    }

    /// Run loctree scan in-process using library API.
    /// Respects `.loctignore` patterns from the project root.
    fn run_scan(project: &Path) -> Result<()> {
        use loctree::args::ParsedArgs;
        let roots = vec![project.to_path_buf()];
        let parsed = ParsedArgs {
            ignore_patterns: loctree::fs_utils::load_loctreeignore(project),
            ..ParsedArgs::default()
        };
        loctree::snapshot::run_init_with_options(&roots, &parsed, true)
            .map_err(|e| anyhow::anyhow!("Scan failed: {}", e))
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
    /// Get repository overview for AI agents
    #[tool(
        name = "repo-view",
        description = "Get repository overview: file count, LOC, languages, health summary, top hubs. USE THIS FIRST at the start of any AI session to understand the codebase."
    )]
    async fn repo_view(&self, Parameters(params): Parameters<ForAiParams>) -> String {
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
                "follow(all) - pursue signals before commits"
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
        let target = match snapshot.files.iter().find(|f| f.path == target_path) {
            Some(target) => target,
            None => {
                return format!(
                    "Error: Internal snapshot inconsistency for '{}'. Run a fresh scan and retry.",
                    params.file
                );
            }
        };

        let mut files = vec![serde_json::json!({
            "path": target.path,
            "layer": "core",
            "loc": target.loc,
            "language": target.language
        })];

        let files_by_path: HashMap<_, _> = snapshot
            .files
            .iter()
            .map(|f| (f.path.as_str(), f))
            .collect();

        // Dependencies - dedup by path and exclude self-references.
        let mut dep_paths: HashSet<&str> = HashSet::new();
        let mut dep_import_types: HashMap<&str, &str> = HashMap::new();
        for edge in snapshot
            .edges
            .iter()
            .filter(|e| e.from == target.path && e.to != target.path)
        {
            dep_paths.insert(edge.to.as_str());
            dep_import_types
                .entry(edge.to.as_str())
                .or_insert(edge.label.as_str());
        }
        let dep_count = dep_paths.len();

        let mut dep_paths: Vec<_> = dep_paths.into_iter().collect();
        dep_paths.sort_unstable();

        for dep_path in dep_paths {
            if let Some(dep) = files_by_path.get(dep_path) {
                files.push(serde_json::json!({
                    "path": dep.path,
                    "layer": "dependency",
                    "loc": dep.loc,
                    "import_type": dep_import_types.get(dep_path).copied().unwrap_or("unknown")
                }));
            }
        }

        // Consumers - dedup by path and exclude self-references.
        let consumer_paths: HashSet<&str> = if params.consumers {
            snapshot
                .edges
                .iter()
                .filter(|e| e.to == target.path && e.from != target.path)
                .map(|e| e.from.as_str())
                .collect()
        } else {
            HashSet::new()
        };
        let consumer_count = consumer_paths.len();

        let mut consumer_paths: Vec<_> = consumer_paths.into_iter().collect();
        consumer_paths.sort_unstable();

        for consumer_path in consumer_paths {
            if let Some(consumer) = files_by_path.get(consumer_path) {
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
            "dependencies": dep_count,
            "consumers": consumer_count,
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

        // Normalize query: split by whitespace and join with | for OR matching (like CLI)
        let query = if params.name.contains('|') {
            // Already has pipe - use as-is
            params.name.clone()
        } else {
            // Split by whitespace, filter short tokens, join with |
            let tokens: Vec<&str> = params
                .name
                .split_whitespace()
                .filter(|t| t.len() >= 2)
                .collect();
            if tokens.is_empty() {
                params.name.clone()
            } else {
                tokens.join("|")
            }
        };

        // Use the same search infrastructure as CLI
        let search_results = run_search(&query, &snapshot.files);

        // Convert symbol matches to JSON format (with limit)
        let symbol_matches: Vec<_> = search_results
            .symbol_matches
            .files
            .iter()
            .flat_map(|f| {
                f.matches.iter().map(move |m| {
                    serde_json::json!({
                        "file": f.file,
                        "symbol": m.context.split_whitespace().last().unwrap_or(&m.context),
                        "kind": if m.is_definition { "definition" } else { "usage" },
                        "line": m.line,
                        "context": m.context
                    })
                })
            })
            .take(params.limit)
            .collect();

        // Convert param matches to JSON format
        let param_matches: Vec<_> = search_results
            .param_matches
            .iter()
            .take(params.limit.saturating_sub(symbol_matches.len()))
            .map(|pm| {
                serde_json::json!({
                    "file": pm.file,
                    "function": pm.function,
                    "param": pm.param_name,
                    "type": pm.param_type,
                    "line": pm.line
                })
            })
            .collect();

        // Convert semantic matches to JSON format
        let semantic_matches: Vec<_> = search_results
            .semantic_matches
            .iter()
            .take(20)
            .map(|sm| {
                serde_json::json!({
                    "symbol": sm.symbol,
                    "file": sm.file,
                    "score": sm.score
                })
            })
            .collect();

        // Convert cross-match files to JSON format (files with 2+ query terms)
        let cross_matches: Vec<_> = search_results
            .cross_matches
            .iter()
            .take(20)
            .map(|cm| {
                let terms: Vec<_> = cm
                    .matched_terms
                    .iter()
                    .map(|t| {
                        let type_tag = match &t.match_type {
                            loctree::analyzer::search::MatchType::Export { kind } => {
                                format!("EXPORT:{}", kind)
                            }
                            loctree::analyzer::search::MatchType::Import { source } => {
                                format!("IMPORT:{}", source)
                            }
                            loctree::analyzer::search::MatchType::Parameter {
                                function, ..
                            } => {
                                format!("PARAM:{}", function)
                            }
                        };
                        serde_json::json!({
                            "term": t.term,
                            "line": t.line,
                            "type": type_tag,
                            "context": t.context
                        })
                    })
                    .collect();
                serde_json::json!({
                    "file": cm.file,
                    "matched_terms": terms
                })
            })
            .collect();

        // Convert suppression matches to JSON format
        let suppression_matches: Vec<_> = search_results
            .suppression_matches
            .iter()
            .take(20)
            .map(|sm| {
                serde_json::json!({
                    "file": sm.file,
                    "line": sm.line,
                    "type": sm.suppression_type,
                    "lint": sm.lint_name,
                    "context": sm.context
                })
            })
            .collect();

        let result = serde_json::json!({
            "query": query,
            "project": project.display().to_string(),
            "symbol_matches": {
                "count": symbol_matches.len(),
                "matches": symbol_matches
            },
            "param_matches": {
                "count": param_matches.len(),
                "matches": param_matches
            },
            "semantic_matches": {
                "count": semantic_matches.len(),
                "matches": semantic_matches
            },
            "cross_matches": {
                "count": cross_matches.len(),
                "matches": cross_matches
            },
            "suppression_matches": {
                "count": suppression_matches.len(),
                "matches": suppression_matches
            },
            "dead_status": {
                "is_exported": search_results.dead_status.is_exported,
                "is_dead": search_results.dead_status.is_dead
            }
        });

        let no_primary_matches =
            symbol_matches.is_empty() && param_matches.is_empty() && semantic_matches.is_empty();
        let mut result = result;
        if no_primary_matches && let Some(obj) = result.as_object_mut() {
            obj.insert(
                "suggestions".to_string(),
                serde_json::json!([
                    "Try a broader pattern or check spelling.",
                    "Browse available exports with repo-view()."
                ]),
            );
        }

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

    /// Focus on a specific directory
    #[tool(
        name = "focus",
        description = "Focus on a specific directory: list files, their LOC, exports, and dependencies within that directory. Great for understanding a module or subsystem."
    )]
    async fn focus(&self, Parameters(params): Parameters<FocusParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        // Filter files in the target directory
        let dir_prefix = if params.directory.ends_with('/') {
            params.directory.clone()
        } else {
            format!("{}/", params.directory)
        };

        let files_in_dir: Vec<_> = snapshot
            .files
            .iter()
            .filter(|f| {
                f.path.starts_with(&dir_prefix) || f.path == params.directory.trim_end_matches('/')
            })
            .collect();

        if files_in_dir.is_empty() {
            let suggestions = suggest_directories(&snapshot, &params.directory, 3);
            return serde_json::json!({
                "directory": params.directory,
                "project": project.display().to_string(),
                "error": "No files found in this directory. Check the path.",
                "suggestions": suggestions
            })
            .to_string();
        }

        let total_loc: usize = files_in_dir.iter().map(|f| f.loc).sum();
        let total_exports: usize = files_in_dir.iter().map(|f| f.exports.len()).sum();

        // Find internal edges (within this directory)
        let internal_edges: Vec<_> = snapshot
            .edges
            .iter()
            .filter(|e| {
                (e.from.starts_with(&dir_prefix)
                    || e.from == params.directory.trim_end_matches('/'))
                    && (e.to.starts_with(&dir_prefix)
                        || e.to == params.directory.trim_end_matches('/'))
            })
            .collect();

        // Find external dependencies (imports from outside)
        let external_deps: Vec<_> = snapshot
            .edges
            .iter()
            .filter(|e| {
                (e.from.starts_with(&dir_prefix)
                    || e.from == params.directory.trim_end_matches('/'))
                    && !e.to.starts_with(&dir_prefix)
            })
            .map(|e| e.to.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        // Find external consumers (files outside that import from this dir)
        let external_consumers: Vec<_> = snapshot
            .edges
            .iter()
            .filter(|e| {
                !e.from.starts_with(&dir_prefix)
                    && (e.to.starts_with(&dir_prefix)
                        || e.to == params.directory.trim_end_matches('/'))
            })
            .map(|e| e.from.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let result = serde_json::json!({
            "directory": params.directory,
            "project": project.display().to_string(),
            "summary": {
                "files": files_in_dir.len(),
                "total_loc": total_loc,
                "total_exports": total_exports,
                "internal_edges": internal_edges.len(),
            },
            "files": files_in_dir.iter().map(|f| serde_json::json!({
                "path": f.path,
                "loc": f.loc,
                "language": f.language,
                "exports": f.exports.len()
            })).collect::<Vec<_>>(),
            "external_dependencies": external_deps.iter().take(20).collect::<Vec<_>>(),
            "external_consumers": external_consumers.iter().take(20).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Follow signals flagged by repo-view at field level
    #[tool(
        name = "follow",
        description = "Pursue structural signals at field level. repo-view flags problems (dead exports, cycles, twins, hotspots) — follow gives you the details with actionable recommendations. Scopes: dead, cycles, twins, hotspots, all."
    )]
    async fn follow(&self, Parameters(params): Parameters<FollowParams>) -> String {
        let project = match Self::resolve_project(&params.project) {
            Ok(p) => p,
            Err(e) => return format!("Error: {}", e),
        };

        let snapshot = match self.get_snapshot(&project).await {
            Ok(s) => s,
            Err(e) => return format!("Error loading project: {}", e),
        };

        let scope = params.scope.to_lowercase();
        let limit = params.limit;
        let mut trails = serde_json::Map::new();

        // Dead exports trail
        if scope == "dead" || scope == "all" {
            let config = DeadFilterConfig::default();
            let dead = find_dead_exports(&snapshot.files, true, None, config);

            // Find nearest candidate consumers for each dead export
            let signals: Vec<_> = dead
                .iter()
                .take(limit)
                .map(|d| {
                    // Find files that import from the same directory (potential wiring candidates)
                    let dir = Path::new(&d.file)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let candidates: Vec<_> = snapshot
                        .edges
                        .iter()
                        .filter(|e| e.to.starts_with(&dir) && e.from != d.file)
                        .map(|e| e.from.clone())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .take(3)
                        .collect();

                    let loc = snapshot
                        .files
                        .iter()
                        .find(|f| f.path == d.file)
                        .map(|f| f.loc)
                        .unwrap_or(0);

                    serde_json::json!({
                        "file": d.file,
                        "symbol": d.symbol,
                        "confidence": d.confidence,
                        "reason": d.reason,
                        "loc": loc,
                        "nearest_candidates": candidates,
                        "action": "remove or wire into candidate consumers"
                    })
                })
                .collect();

            trails.insert(
                "dead_exports".to_string(),
                serde_json::json!({
                    "total": dead.len(),
                    "shown": signals.len(),
                    "signals": signals
                }),
            );
        }

        // Cycles trail
        if scope == "cycles" || scope == "all" {
            let edges: Vec<_> = snapshot
                .edges
                .iter()
                .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
                .collect();
            let cycles = find_cycles(&edges);

            let signals: Vec<_> = cycles
                .iter()
                .take(limit)
                .map(|chain| {
                    // Calculate total LOC in cycle
                    let total_loc: usize = chain
                        .iter()
                        .filter_map(|f| snapshot.files.iter().find(|a| a.path == *f))
                        .map(|a| a.loc)
                        .sum();

                    // Find weakest link (edge with fewest symbols crossing)
                    let mut weakest = ("", "", usize::MAX);
                    for i in 0..chain.len() {
                        let from = &chain[i];
                        let to = &chain[(i + 1) % chain.len()];
                        let symbols_crossed = snapshot
                            .edges
                            .iter()
                            .filter(|e| e.from == *from && e.to == *to)
                            .count();
                        if symbols_crossed < weakest.2 {
                            weakest = (from, to, symbols_crossed);
                        }
                    }

                    serde_json::json!({
                        "chain": chain,
                        "length": chain.len(),
                        "total_loc": total_loc,
                        "weakest_link": {
                            "from": weakest.0,
                            "to": weakest.1,
                            "symbols_crossed": weakest.2
                        },
                        "action": "break at weakest link"
                    })
                })
                .collect();

            trails.insert(
                "cycles".to_string(),
                serde_json::json!({
                    "total": cycles.len(),
                    "shown": signals.len(),
                    "signals": signals
                }),
            );
        }

        // Twins trail
        if scope == "twins" || scope == "all" {
            let twins = detect_exact_twins(&snapshot.files, false);

            let signals: Vec<_> = twins
                .iter()
                .take(limit)
                .map(|twin| {
                    let files: Vec<_> = twin.locations.iter().map(|l| &l.file_path).collect();
                    serde_json::json!({
                        "symbol": twin.name,
                        "files": files,
                        "locations": twin.locations.iter().map(|l| serde_json::json!({
                            "file": l.file_path,
                            "line": l.line,
                            "kind": l.kind,
                            "importers": l.import_count
                        })).collect::<Vec<_>>(),
                        "signature_similarity": twin.signature_similarity,
                        "action": "consolidate into single module"
                    })
                })
                .collect();

            trails.insert(
                "twins".to_string(),
                serde_json::json!({
                    "total": twins.len(),
                    "shown": signals.len(),
                    "signals": signals
                }),
            );
        }

        // Hotspots trail (files with most importers)
        if scope == "hotspots" || scope == "all" {
            let mut import_counts: HashMap<&str, usize> = HashMap::new();
            for edge in &snapshot.edges {
                *import_counts.entry(&edge.to).or_default() += 1;
            }
            let mut hubs: Vec<_> = import_counts.into_iter().collect();
            hubs.sort_by(|a, b| b.1.cmp(&a.1));

            let signals: Vec<_> = hubs
                .iter()
                .take(limit)
                .map(|(file, importers)| {
                    let loc = snapshot
                        .files
                        .iter()
                        .find(|f| f.path == *file)
                        .map(|f| f.loc)
                        .unwrap_or(0);

                    let risk = if *importers > 30 {
                        "high — changes here ripple everywhere"
                    } else if *importers > 10 {
                        "medium — significant blast radius"
                    } else {
                        "low"
                    };

                    serde_json::json!({
                        "file": file,
                        "importers": importers,
                        "loc": loc,
                        "risk": risk,
                        "action": if *importers > 20 { "split or freeze interface" } else { "monitor" }
                    })
                })
                .collect();

            trails.insert(
                "hotspots".to_string(),
                serde_json::json!({
                    "total": hubs.len(),
                    "shown": signals.len(),
                    "signals": signals
                }),
            );
        }

        let result = serde_json::json!({
            "project": project.display().to_string(),
            "scope": params.scope,
            "trails": trails
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
                description: Some("Structural code intelligence for AI agents".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://github.com/Loctree/Loctree".to_string()),
            },
            instructions: Some(
                "Loctree MCP provides structural code intelligence. Use these tools for codebase awareness.\n\n\
                 BASELINE TOOLS:\n\
                 - repo-view(project) - Start here. Overview: files, LOC, languages, health, top hubs.\n\
                 - slice(file) - Before modifying. File + dependencies + consumers in one call.\n\
                 - find(name) - Before creating. Symbol search with regex support.\n\
                 - impact(file) - Before deleting. Direct + transitive consumers (blast radius).\n\
                 - focus(directory) - Understand a module. Files, internal edges, external deps.\n\
                 - tree(project) - Directory structure with LOC counts.\n\
                 - follow(scope) - Pursue signals: dead exports, cycles, twins, hotspots. Field-level detail.\n\n\
                 All tools accept 'project' parameter (default: current dir).\n\
                 First use auto-scans if no snapshot exists."
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
fn safe_stderr_log(line: &str) {
    // Never panic while reporting errors from a panic hook.
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(line.as_bytes());
    let _ = stderr.write_all(b"\n");
    let _ = stderr.flush();
}

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
            safe_stderr_log("[loctree-mcp] Client disconnected (broken pipe), shutting down");
            std::process::exit(0);
        } else {
            // Log other panics with location info
            let location = panic_info
                .location()
                .map(|loc| format!(" at {}:{}:{}", loc.file(), loc.line(), loc.column()))
                .unwrap_or_default();
            safe_stderr_log(&format!("[loctree-mcp] Panic{}: {}", location, msg));
        }
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
        // Prevent tracing from recursively writing fallback errors to stderr when stderr is closed.
        .log_internal_errors(false)
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
