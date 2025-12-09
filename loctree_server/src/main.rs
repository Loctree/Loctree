//! # loctree_server
//!
//! MCP (Model Context Protocol) server for loctree.
//! Provides hot codebase analysis for AI agents with instant responses.
//!
//! ## Architecture
//!
//! The server loads the project snapshot into RAM at startup, enabling:
//! - Instant holographic slices (< 10ms)
//! - Fast symbol search across the entire codebase
//! - Real-time dead code and cycle detection
//!
//! ## Usage with rmcp-mux
//!
//! ```bash
//! # Start via mux (recommended for hot-service architecture)
//! rmcp-mux --socket /tmp/loctree.sock --cmd loctree_server -- --project .
//!
//! # Standalone
//! loctree_server --project /path/to/project
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ServerInfo;
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

use loctree::analyzer::crowd::detect_all_crowds_with_edges;
use loctree::analyzer::cycles::find_cycles;
use loctree::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports};
use loctree::analyzer::twins::{build_symbol_registry, detect_exact_twins};
use loctree::query;
use loctree::snapshot::Snapshot;

// ============================================================================
// CLI Arguments
// ============================================================================

#[derive(Parser, Debug)]
#[command(name = "loctree_server")]
#[command(about = "MCP server for loctree - hot codebase analysis for AI agents")]
#[command(version)]
struct Args {
    /// Project root directory (will load .loctree/snapshot.json)
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    /// Force reload snapshot on each request (slower, but always fresh)
    #[arg(long, default_value = "false")]
    no_cache: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
}

// ============================================================================
// Tool Parameter Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct GetSliceParams {
    /// File path relative to project root (e.g., 'src/App.tsx')
    path: String,
    /// Include consumer files (files that import this file)
    #[serde(default = "default_true")]
    include_consumers: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FindSymbolParams {
    /// Symbol name or regex pattern to search for
    pattern: String,
    /// Filter by file path pattern (optional)
    path_filter: Option<String>,
    /// Maximum results to return (default: 50)
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CheckDeadParams {
    /// Confidence filter: 'high', 'medium', or 'all' (default: 'all')
    #[serde(default)]
    confidence: Option<String>,
    /// Maximum results to return (default: 100)
    #[serde(default = "default_dead_limit")]
    limit: usize,
}

fn default_dead_limit() -> usize {
    100
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WhoImportsParams {
    /// File path to find importers for
    file: String,
}

// ============================================================================
// Tool Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct SliceResponse {
    target: String,
    core_loc: usize,
    deps_count: usize,
    consumers_count: usize,
    files: Vec<SliceFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SliceFile {
    path: String,
    layer: String, // "core", "dep", "consumer"
    loc: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct SymbolSearchResult {
    query: String,
    matches: Vec<SymbolMatch>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SymbolMatch {
    file: String,
    symbol: String,
    kind: String,
    line: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeadCodeResult {
    count: usize,
    symbols: Vec<DeadSymbolInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeadSymbolInfo {
    file: String,
    symbol: String,
    confidence: String,
    line: Option<usize>,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CyclesResult {
    count: usize,
    cycles: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WhoImportsResult {
    file: String,
    importers: Vec<ImporterInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ImporterInfo {
    file: String,
    line: Option<usize>,
}

// ============================================================================
// Server State
// ============================================================================

/// Hot server state with snapshot in RAM.
#[derive(Clone)]
struct LoctreeServer {
    /// Project root path
    project_root: PathBuf,
    /// Cached snapshot (loaded once, kept hot)
    snapshot: Arc<RwLock<Snapshot>>,
    /// Disable caching (reload on each request)
    no_cache: bool,
    /// Tool router (generated by macro)
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl LoctreeServer {
    /// Create new server with hot snapshot.
    async fn new(project_root: PathBuf, no_cache: bool) -> Result<Self> {
        info!("Loading snapshot from {:?}", project_root);
        let snapshot = Snapshot::load(&project_root)
            .context("Failed to load snapshot. Run 'loct scan' first.")?;
        info!(
            "Snapshot loaded: {} files, {} edges",
            snapshot.files.len(),
            snapshot.edges.len()
        );

        Ok(Self {
            project_root,
            snapshot: Arc::new(RwLock::new(snapshot)),
            no_cache,
            tool_router: Self::tool_router(),
        })
    }

    /// Reload snapshot if no_cache is enabled.
    async fn maybe_reload(&self) -> Result<()> {
        if self.no_cache {
            let new_snapshot = Snapshot::load(&self.project_root)?;
            let mut snapshot = self.snapshot.write().await;
            *snapshot = new_snapshot;
        }
        Ok(())
    }
}

// ============================================================================
// MCP Tool Implementations
// ============================================================================

#[tool_router]
impl LoctreeServer {
    /// Get a holographic slice for a file (core + deps + consumers).
    #[tool(
        name = "get_slice",
        description = "Extract holographic context for a file. Returns the file, its dependencies, and files that import it."
    )]
    async fn get_slice(&self, Parameters(params): Parameters<GetSliceParams>) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;

        // Find the file in snapshot
        let Some(target) = snapshot
            .files
            .iter()
            .find(|f| f.path.ends_with(&params.path))
        else {
            return format!("File not found: {}", params.path);
        };

        // Build slice using snapshot data
        let mut files = Vec::new();

        // Core layer
        files.push(SliceFile {
            path: target.path.clone(),
            layer: "core".to_string(),
            loc: target.loc,
        });

        // Deps layer (files this file imports)
        let deps: Vec<_> = snapshot
            .edges
            .iter()
            .filter(|e| e.from == target.path)
            .collect();

        for edge in &deps {
            if let Some(dep) = snapshot.files.iter().find(|f| f.path == edge.to) {
                files.push(SliceFile {
                    path: dep.path.clone(),
                    layer: "dep".to_string(),
                    loc: dep.loc,
                });
            }
        }

        // Consumers layer (files that import this file)
        let consumers: Vec<_> = if params.include_consumers {
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
                files.push(SliceFile {
                    path: consumer.path.clone(),
                    layer: "consumer".to_string(),
                    loc: consumer.loc,
                });
            }
        }

        let response = SliceResponse {
            target: params.path,
            core_loc: target.loc,
            deps_count: deps.len(),
            consumers_count: consumers.len(),
            files,
        };

        serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Search for symbols matching a pattern.
    #[tool(
        name = "find_symbol",
        description = "Search for symbols (functions, classes, exports) matching a pattern. Supports regex."
    )]
    async fn find_symbol(&self, Parameters(params): Parameters<FindSymbolParams>) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;

        let regex = match regex::Regex::new(&params.pattern) {
            Ok(r) => r,
            Err(e) => return format!("Invalid regex: {}", e),
        };

        let path_regex = params
            .path_filter
            .as_ref()
            .map(|p| regex::Regex::new(p))
            .transpose();

        let path_regex = match path_regex {
            Ok(r) => r,
            Err(e) => return format!("Invalid path regex: {}", e),
        };

        let mut matches = Vec::new();

        for file in &snapshot.files {
            // Filter by path if specified
            if let Some(ref pr) = path_regex
                && !pr.is_match(&file.path)
            {
                continue;
            }

            // Search exports
            for export in &file.exports {
                if regex.is_match(&export.name) {
                    matches.push(SymbolMatch {
                        file: file.path.clone(),
                        symbol: export.name.clone(),
                        kind: export.kind.clone(),
                        line: export.line,
                    });
                    if matches.len() >= params.limit {
                        break;
                    }
                }
            }

            if matches.len() >= params.limit {
                break;
            }
        }

        let response = SymbolSearchResult {
            query: params.pattern,
            matches,
        };

        serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Find unused exports (dead code).
    #[tool(
        name = "check_dead",
        description = "Detect dead code - exported symbols that are never imported anywhere."
    )]
    async fn check_dead(&self, Parameters(params): Parameters<CheckDeadParams>) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;
        let confidence_filter = params.confidence.as_deref();

        let config = DeadFilterConfig::default();
        let dead_exports = find_dead_exports(&snapshot.files, true, None, config);

        let filtered: Vec<_> = dead_exports
            .into_iter()
            .filter(|d| match confidence_filter {
                Some("high") => d.confidence == "high",
                Some("medium") => d.confidence == "medium" || d.confidence == "high",
                _ => true,
            })
            .take(params.limit)
            .map(|d| DeadSymbolInfo {
                file: d.file,
                symbol: d.symbol,
                confidence: d.confidence,
                line: d.line,
                reason: d.reason,
            })
            .collect();

        let response = DeadCodeResult {
            count: filtered.len(),
            symbols: filtered,
        };

        serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Detect circular imports.
    #[tool(
        name = "check_cycles",
        description = "Find circular import chains in the codebase."
    )]
    async fn check_cycles(&self) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;

        // Build edges for cycle detection
        let edges: Vec<(String, String, String)> = snapshot
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
            .collect();

        let cycles = find_cycles(&edges);

        let response = CyclesResult {
            count: cycles.len(),
            cycles,
        };

        serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Find files that import a given file.
    #[tool(
        name = "who_imports",
        description = "Find all files that import a given file (reverse dependency lookup)."
    )]
    async fn who_imports(&self, Parameters(params): Parameters<WhoImportsParams>) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;
        let result = query::query_who_imports(&snapshot, &params.file);

        let importers: Vec<_> = result
            .results
            .into_iter()
            .map(|m| ImporterInfo {
                file: m.file,
                line: m.line,
            })
            .collect();

        let response = WhoImportsResult {
            file: params.file,
            importers,
        };

        serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Get project overview and health metrics.
    #[tool(
        name = "project_info",
        description = "Get project overview: file count, total LOC, dead code count, cycle count."
    )]
    async fn project_info(&self) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;

        let total_loc: usize = snapshot.files.iter().map(|f| f.loc).sum();
        let file_count = snapshot.files.len();
        let edge_count = snapshot.edges.len();

        // Quick dead code count
        let config = DeadFilterConfig::default();
        let dead_count = find_dead_exports(&snapshot.files, true, None, config).len();

        // Quick cycle count
        let edges: Vec<_> = snapshot
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
            .collect();
        let cycle_count = find_cycles(&edges).len();

        let info = serde_json::json!({
            "project_root": self.project_root.display().to_string(),
            "file_count": file_count,
            "total_loc": total_loc,
            "edge_count": edge_count,
            "dead_exports": dead_count,
            "circular_imports": cycle_count,
        });

        serde_json::to_string_pretty(&info)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Detect twins (dead parrots and exact duplicates).
    #[tool(
        name = "check_twins",
        description = "Find dead parrots (exports with 0 imports) and exact twins (same symbol exported from multiple files)."
    )]
    async fn check_twins(&self) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;
        let registry = build_symbol_registry(&snapshot.files);

        // Dead parrots: symbols with 0 imports
        let dead_parrots: Vec<_> = registry
            .values()
            .filter(|s| s.import_count == 0)
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "kind": s.kind,
                    "file": s.file_path,
                    "line": s.line
                })
            })
            .collect();

        // Exact twins: same name exported from different files
        let exact_twins = detect_exact_twins(&snapshot.files);
        let twins_json: Vec<_> = exact_twins
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "location_count": t.locations.len(),
                    "locations": t.locations.iter().map(|loc| serde_json::json!({
                        "file": loc.file_path,
                        "line": loc.line,
                        "kind": loc.kind,
                        "import_count": loc.import_count,
                        "is_canonical": loc.is_canonical
                    })).collect::<Vec<_>>()
                })
            })
            .collect();

        let result = serde_json::json!({
            "dead_parrots": {
                "count": dead_parrots.len(),
                "symbols": dead_parrots.into_iter().take(50).collect::<Vec<_>>()
            },
            "exact_twins": {
                "count": exact_twins.len(),
                "twins": twins_json.into_iter().take(50).collect::<Vec<_>>()
            }
        });

        serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("Serialization error: {}", e))
    }

    /// Detect functional crowds (clusters of files with similar purpose).
    #[tool(
        name = "check_crowds",
        description = "Find crowds - groups of files that cluster around similar functionality, potentially indicating duplication."
    )]
    async fn check_crowds(&self) -> String {
        if let Err(e) = self.maybe_reload().await {
            return format!("Error reloading snapshot: {}", e);
        }

        let snapshot = self.snapshot.read().await;

        // Build edges for transitive analysis
        let edges: Vec<_> = snapshot.edges.clone();
        let crowds = detect_all_crowds_with_edges(&snapshot.files, &edges);

        let crowds_json: Vec<_> = crowds
            .iter()
            .filter(|c| c.score >= 3.0) // Only show significant crowds
            .take(10)
            .map(|c| {
                serde_json::json!({
                    "pattern": c.pattern,
                    "score": c.score,
                    "member_count": c.members.len(),
                    "members": c.members.iter().take(5).map(|m| serde_json::json!({
                        "file": m.file,
                        "importer_count": m.importer_count
                    })).collect::<Vec<_>>(),
                    "issues": c.issues.iter().map(|i| format!("{:?}", i)).collect::<Vec<_>>()
                })
            })
            .collect();

        let result = serde_json::json!({
            "crowd_count": crowds.len(),
            "significant_crowds": crowds_json.len(),
            "crowds": crowds_json
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
                "Loctree MCP server for AI-oriented codebase analysis. \
                 Provides instant holographic slices, symbol search, dead code detection, \
                 and circular import detection."
                    .into(),
            ),
        }
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging - MUST write to stderr, stdout is for MCP JSON-RPC
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| args.log_level.parse().unwrap_or_default()),
        )
        .init();

    info!("Starting loctree_server v{}", env!("CARGO_PKG_VERSION"));
    info!("Project: {:?}", args.project);

    // Create server with hot snapshot
    let server = LoctreeServer::new(args.project, args.no_cache).await?;

    info!("Server ready. Listening on stdio...");

    // Run MCP server on stdio
    server
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;

    Ok(())
}
