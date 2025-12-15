//! Command enum and related types for the new CLI interface.
//!
//! This module defines the canonical `loct <command> [options]` interface.
//! The Command enum is the source of truth for all CLI commands and backs
//! both the parser and help output.
//!
//! # Design Philosophy
//!
//! - **Agent-first**: The tool is primarily for AI agents, with humans having
//!   a minimal CLI interface (5 core commands).
//! - **Minimal commands, exclusive flags**: Flags modify/exclude default behavior,
//!   they don't add functionality.
//! - **Regex on metadata**: Agents can filter using regex on symbol names, paths,
//!   namespaces - but never on raw source code.

use std::path::PathBuf;

use crate::types::ColorMode;

// ============================================================================
// Global Options (shared across all commands)
// ============================================================================

/// Global options that apply to all commands.
///
/// These flags can be used with any command and control output format,
/// verbosity, and other cross-cutting concerns.
#[derive(Debug, Clone, Default)]
pub struct GlobalOptions {
    /// Output as JSON (stdout is JSON only, warnings go to stderr)
    pub json: bool,

    /// Suppress all non-essential output including deprecation warnings
    pub quiet: bool,

    /// Color mode for terminal output
    pub color: ColorMode,

    /// Verbose output with progress information
    pub verbose: bool,

    /// Library/framework mode (tunes dead-code heuristics, ignores examples)
    pub library_mode: bool,

    /// Python library mode (treat __all__ exports as public API, skip dunder methods)
    pub python_library: bool,

    /// Additional Python package roots for import resolution
    pub py_roots: Vec<PathBuf>,
}

// ============================================================================
// Command Enum - Source of Truth
// ============================================================================

/// The canonical command enum for the `loct <command>` interface.
///
/// Each variant maps to a handler module. This enum is the single source
/// of truth for CLI commands and backs both parser and help output.
#[derive(Debug, Clone)]
pub enum Command {
    /// Automatic full scan with stack detection (default when no command given).
    ///
    /// Performs: stack detection → path mapping → full analysis → save to .loctree/
    /// This is the "one-shot scan" for humans and the recommended starting point.
    Auto(AutoOptions),

    /// Build/update snapshot for current HEAD.
    ///
    /// Explicit scan command for when you want to update the snapshot
    /// without running analysis.
    Scan(ScanOptions),

    /// Display LOC tree / structural overview.
    ///
    /// Replacement for legacy `--tree` flag.
    Tree(TreeOptions),

    /// Produce 3-layer holographic context for a path.
    ///
    /// Extracts: Core (target) + Deps (imports) + Consumers (importers).
    /// Primary interface for AI agents needing file context.
    Slice(SliceOptions),

    /// Search symbols/files/impact/similar.
    ///
    /// Consolidates search functionality with regex support on metadata.
    Find(FindOptions),

    /// Detect unused exports / dead code.
    ///
    /// Replacement for legacy `-A --dead`.
    Dead(DeadOptions),

    /// Detect circular imports / structural cycles.
    ///
    /// Replacement for legacy `-A --circular`.
    Cycles(CyclesOptions),

    /// Show Tauri command bridges (FE ↔ BE mappings).
    ///
    /// Lists all Tauri commands with their frontend invocations
    /// and backend handlers.
    Commands(CommandsOptions),

    /// Show backend/web routes (FastAPI/Flask/etc.)
    ///
    /// Lists route decorators detected in Python backends.
    Routes(RoutesOptions),

    /// Show event flow (ghost events, orphan handlers, races).
    ///
    /// Analyzes event-driven patterns and detects issues.
    Events(EventsOptions),

    /// Snapshot metadata and project info.
    ///
    /// Quick sanity check: snapshot exists, schema version, file counts, timestamps.
    Info(InfoOptions),

    /// Structural lint/policy checks.
    ///
    /// Handles entrypoint validation, SARIF output for CI.
    Lint(LintOptions),

    /// Generate HTML/JSON reports.
    ///
    /// Interactive reports with dependency graphs.
    Report(ReportOptions),

    /// Show help for commands.
    Help(HelpOptions),

    /// Show version.
    Version,

    /// Query snapshot data (who-imports, where-symbol, component-of).
    ///
    /// Interactive queries against the cached snapshot for fast lookups.
    Query(QueryOptions),

    /// Compare two snapshots and show delta.
    ///
    /// Analyzes semantic changes between commits: files changed, imports added/removed,
    /// exports changed, and impact on consumers.
    Diff(DiffOptions),

    /// Index analysis into AI memory (vector database).
    ///
    /// Converts loctree analysis (dead code, duplications) into semantic
    /// vectors and stores them in LanceDB for AI agent queries.
    Memex(MemexOptions),

    /// Detect functional crowds (similar files clustering).
    ///
    /// Identifies groups of files that cluster around the same functionality,
    /// suggesting potential consolidation or refactoring opportunities.
    Crowd(CrowdOptions),

    /// Unified search around a keyword - files, crowds, and dead exports.
    ///
    /// Aggregates files matching the keyword, crowd analysis, and dead code
    /// into a single comprehensive view.
    Tagmap(TagmapOptions),

    /// Show symbol registry and dead parrots (semantic duplicate detection).
    ///
    /// Builds a registry of all exported symbols and their import counts.
    /// Dead parrots are symbols with 0 imports - candidates for removal.
    Twins(TwinsOptions),

    /// Manage false positive suppressions.
    ///
    /// Mark findings as "reviewed and OK" so they don't appear in subsequent runs.
    /// Suppressions are stored in `.loctree/suppressions.toml`.
    Suppress(SuppressOptions),

    /// Analyze bundle distribution using source maps.
    ///
    /// Compares source exports with bundled code to find truly dead exports
    /// that were tree-shaken out by the bundler.
    Dist(DistOptions),

    /// Analyze test coverage gaps.
    ///
    /// Cross-references production usage (FE invoke/emit) with test imports
    /// to find handlers and events without test coverage.
    Coverage(CoverageOptions),

    /// Sniff for code smells (twins + dead parrots + crowds).
    ///
    /// Aggregates all smell-level findings worth checking:
    /// - Twins: same symbol name in multiple files
    /// - Dead parrots: exports with 0 imports
    /// - Crowds: files with similar dependency patterns
    ///
    /// Output is friendly and non-judgmental - these are hints, not verdicts.
    Sniff(SniffOptions),

    /// Query snapshot with jq-style filters (loct '.metadata').
    ///
    /// Runs jq-style filter expressions on the latest snapshot JSON.
    /// Supports standard jq flags: -r (raw), -c (compact), -e (exit status),
    /// --arg (string vars), --argjson (JSON vars), --snapshot (explicit path).
    JqQuery(JqQueryOptions),

    /// Analyze impact of modifying/removing a file.
    ///
    /// Shows which files would break if you modify or remove the target file.
    /// Traverses the reverse dependency graph to find direct and transitive consumers.
    Impact(ImpactCommandOptions),

    /// Focus on a directory - extract holographic context for all files in a directory.
    ///
    /// Like `slice` but for directories. Shows core files, internal edges,
    /// external dependencies, and consumers of the directory as a unit.
    Focus(FocusOptions),

    /// Show import frequency heatmap - which files are core vs peripheral.
    ///
    /// Ranks files by how often they are imported (in-degree) to identify
    /// hub modules (core infrastructure) vs leaf modules (feature endpoints).
    Hotspots(HotspotsOptions),

    /// Analyze CSS layout properties (z-index, position, display).
    ///
    /// Extracts layout structure from CSS/SCSS files to help understand
    /// UI layering, sticky/fixed elements, and grid/flex usage.
    Layoutmap(LayoutmapOptions),

    /// Find zombie code (dead exports + orphan files + shadow exports).
    ///
    /// Combines three sources of dead code into one actionable report:
    /// - Dead exports (from dead code analysis)
    /// - Orphan files (files with 0 importers)
    /// - Shadow exports (symbols exported by multiple files where some have 0 imports)
    Zombie(ZombieOptions),
}

impl Default for Command {
    fn default() -> Self {
        Command::Auto(AutoOptions::default())
    }
}

// ============================================================================
// Per-Command Options
// ============================================================================

/// Options for the `auto` command (default behavior).
#[derive(Debug, Clone, Default)]
pub struct AutoOptions {
    /// Root directories to scan (defaults to current directory)
    pub roots: Vec<PathBuf>,

    /// Force full rescan ignoring mtime cache
    pub full_scan: bool,

    /// Include normally-ignored directories (node_modules, target, .venv)
    pub scan_all: bool,

    /// Generate AI agent feed report (ForAi mode)
    pub for_agent_feed: bool,

    /// Emit single-shot agent JSON bundle (vs JSONL stream)
    pub agent_json: bool,

    /// Suppress duplicate export output (noise reduction)
    pub suppress_duplicates: bool,

    /// Suppress dynamic imports output (noise reduction)
    pub suppress_dynamic: bool,
}

/// Options for the `scan` command.
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Root directories to scan
    pub roots: Vec<PathBuf>,

    /// Force full rescan ignoring mtime cache
    pub full_scan: bool,

    /// Include normally-ignored directories
    pub scan_all: bool,

    /// Watch for file changes and re-scan automatically
    pub watch: bool,
}

/// Options for the `tree` command.
#[derive(Debug, Clone, Default)]
pub struct TreeOptions {
    /// Root directories to display
    pub roots: Vec<PathBuf>,

    /// Maximum depth of tree recursion
    pub depth: Option<usize>,

    /// Show summary with top N large files
    pub summary: Option<usize>,

    /// Suppress full tree output, show top list only
    pub summary_only: bool,

    /// LOC threshold for highlighting large files
    pub loc_threshold: Option<usize>,

    /// Include hidden files (dotfiles)
    pub show_hidden: bool,

    /// Find build artifacts (node_modules, target, .venv)
    pub find_artifacts: bool,

    /// Show gitignored files
    pub show_ignored: bool,
}

/// Options for the `slice` command.
#[derive(Debug, Clone, Default)]
pub struct SliceOptions {
    /// Target file path for the slice
    pub target: String,

    /// Root directory (defaults to current directory)
    pub root: Option<PathBuf>,

    /// Include consumer files (files that import the target)
    pub consumers: bool,

    /// Maximum depth for dependency traversal
    pub depth: Option<usize>,

    /// Force rescan before slicing (includes uncommitted files)
    pub rescan: bool,
}

/// Options for the `find` command.
///
/// Supports regex filtering on metadata fields for AI agent queries.
#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    /// Search query (can be regex pattern)
    pub query: Option<String>,

    /// Filter by symbol name (regex supported)
    pub symbol: Option<String>,

    /// Filter by file path (regex supported)
    pub file: Option<String>,

    /// Find files impacted by changes to this file
    pub impact: Option<String>,

    /// Find similar symbols (fuzzy matching)
    pub similar: Option<String>,

    /// Filter to dead code only
    pub dead_only: bool,

    /// Filter to exported symbols only
    pub exported_only: bool,

    /// Programming language filter
    pub lang: Option<String>,

    /// Maximum results to return (default: 200)
    pub limit: Option<usize>,
}

/// Options for the `dead` command.
#[derive(Debug, Clone, Default)]
pub struct DeadOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Confidence level filter (high, medium, low)
    pub confidence: Option<String>,

    /// Maximum number of dead symbols to report
    pub top: Option<usize>,

    /// Show full list (no top limit)
    pub full: bool,

    /// Filter by file path pattern (regex)
    pub path_filter: Option<String>,

    /// Include tests in dead-export detection (default: false)
    pub with_tests: bool,

    /// Include helper/scripts/docs files (default: false)
    pub with_helpers: bool,

    /// Detect shadow exports (same symbol exported by multiple files, only one used)
    pub with_shadows: bool,
}

/// Options for the `cycles` command.
#[derive(Debug, Clone, Default)]
pub struct CyclesOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Filter by file path pattern (regex)
    pub path_filter: Option<String>,
}

/// Options for the `commands` command (Tauri command bridges).
#[derive(Debug, Clone, Default)]
pub struct CommandsOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Filter by command name (regex)
    pub name_filter: Option<String>,

    /// Show only commands with missing handlers
    pub missing_only: bool,

    /// Show only commands with missing frontend invocations
    pub unused_only: bool,

    /// Suppress duplicate sections (noise reduction)
    pub suppress_duplicates: bool,

    /// Suppress dynamic import sections (noise reduction)
    pub suppress_dynamic: bool,

    /// Maximum number of results to show (for limiting large outputs)
    pub limit: Option<usize>,
}

/// Options for the `coverage` command (test coverage analysis).
#[derive(Debug, Clone, Default)]
pub struct CoverageOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Show only handler coverage gaps
    pub handlers_only: bool,

    /// Show only event coverage gaps
    pub events_only: bool,

    /// Filter by minimum severity (critical/high/medium/low)
    pub min_severity: Option<String>,
}

/// Options for the `routes` command.
#[derive(Debug, Clone, Default)]
pub struct RoutesOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Filter by framework label (fastapi/flask)
    pub framework: Option<String>,

    /// Filter by route path substring
    pub path_filter: Option<String>,
}

/// Options for the `events` command.
#[derive(Debug, Clone, Default)]
pub struct EventsOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Show ghost events (emitted but not handled)
    pub ghost: bool,

    /// Show orphan handlers (handlers without emitters)
    pub orphan: bool,

    /// Show potential race conditions
    pub races: bool,

    /// Show only FE↔FE sync events (window sync pattern)
    pub fe_sync: bool,

    /// Suppress duplicate sections (noise reduction)
    pub suppress_duplicates: bool,

    /// Suppress dynamic import sections (noise reduction)
    pub suppress_dynamic: bool,
}

/// Options for the `info` command.
#[derive(Debug, Clone, Default)]
pub struct InfoOptions {
    /// Root directory to check
    pub root: Option<PathBuf>,
}

/// Options for the `lint` command.
#[derive(Debug, Clone, Default)]
pub struct LintOptions {
    /// Root directories to lint
    pub roots: Vec<PathBuf>,

    /// Check entrypoint coverage
    pub entrypoints: bool,

    /// Fail with non-zero exit code on issues
    pub fail: bool,

    /// Output in SARIF format for CI integration
    pub sarif: bool,

    /// Enable Tauri-specific checks
    pub tauri: bool,

    /// Suppress duplicate sections (noise reduction)
    pub suppress_duplicates: bool,

    /// Suppress dynamic import sections (noise reduction)
    pub suppress_dynamic: bool,
}

/// Options for the `report` command.
#[derive(Debug, Clone, Default)]
pub struct ReportOptions {
    /// Root directories to report on
    pub roots: Vec<PathBuf>,

    /// Output format (html, json)
    pub format: Option<String>,

    /// Output file path
    pub output: Option<PathBuf>,

    /// Start a local server to view the report
    pub serve: bool,

    /// Server port
    pub port: Option<u16>,

    /// Editor integration (code, cursor, windsurf, jetbrains)
    pub editor: Option<String>,
}

/// Options for the `diff` command.
#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    /// Snapshot ID or path to compare against (from)
    pub since: Option<String>,

    /// Second snapshot ID or path (to). If omitted, compare against current state
    pub to: Option<String>,

    /// Output as JSONL (one line per change)
    pub jsonl: bool,

    /// Show only new problems (added dead exports, new cycles, new missing handlers)
    pub problems_only: bool,

    /// Automatically scan target branch using git worktree (zero-friction diff)
    pub auto_scan_base: bool,
}

/// Options for the `memex` command.
/// Indexes loctree analysis into AI memory (vector database).
#[derive(Debug, Clone)]
pub struct MemexOptions {
    /// Path to the .loctree directory or analysis.json file
    pub report_path: PathBuf,

    /// Unique project identifier (e.g., "github.com/org/repo")
    pub project_id: Option<String>,

    /// Namespace for the memory index (default: "loctree")
    pub namespace: String,

    /// Path to the LanceDB storage directory
    pub db_path: Option<String>,
}

impl Default for MemexOptions {
    fn default() -> Self {
        Self {
            report_path: PathBuf::from(".loctree"),
            project_id: None,
            namespace: "loctree".to_string(),
            db_path: None,
        }
    }
}

/// Options for the `crowd` command.
/// Detects functional crowds - multiple files clustering around same functionality.
#[derive(Debug, Clone, Default)]
pub struct CrowdOptions {
    /// Pattern to detect crowd around (e.g., "message", "patient", "auth")
    pub pattern: Option<String>,

    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Detect all crowds automatically (if no pattern specified)
    pub auto_detect: bool,

    /// Minimum crowd size to report (default: 2)
    pub min_size: Option<usize>,

    /// Maximum crowds to show in auto-detect mode (default: 10)
    pub limit: Option<usize>,

    /// Include test files in crowd detection (default: false)
    /// Tests are entry points by design - they have 0 importers and create noise
    pub include_tests: bool,
}

/// Options for the `tagmap` command.
/// Unified search aggregating files, crowds, and dead code around a keyword.
#[derive(Debug, Clone, Default)]
pub struct TagmapOptions {
    /// Keyword to search for (in paths and names)
    pub keyword: String,

    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Include test files in analysis (default: false)
    pub include_tests: bool,

    /// Maximum results to show per section
    pub limit: Option<usize>,
}

/// Options for the `twins` command.
/// Shows symbol registry and dead parrots (0 import count).
#[derive(Debug, Clone, Default)]
pub struct TwinsOptions {
    /// Root directory to analyze (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Show only dead parrots (symbols with 0 imports)
    pub dead_only: bool,

    /// Include suppressed findings in output
    pub include_suppressed: bool,

    /// Include test files in analysis (default: false)
    pub include_tests: bool,
}

/// Options for the `suppress` command.
/// Manage false positive suppressions.
#[derive(Debug, Clone, Default)]
pub struct SuppressOptions {
    /// Root directory (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Type of finding to suppress: twins, dead_parrot, dead_export, circular
    pub suppression_type: Option<String>,

    /// Symbol name to suppress
    pub symbol: Option<String>,

    /// Optional: specific file path
    pub file: Option<String>,

    /// Reason for suppression
    pub reason: Option<String>,

    /// List all current suppressions
    pub list: bool,

    /// Clear all suppressions
    pub clear: bool,

    /// Remove a specific suppression
    pub remove: bool,
}

/// Options for the `dist` command.
/// Analyzes bundle distribution using source maps.
#[derive(Debug, Clone, Default)]
pub struct DistOptions {
    /// Path to source map file (.js.map)
    pub source_map: Option<PathBuf>,

    /// Source directory to scan for exports
    pub src: Option<PathBuf>,
}

/// Options for the `sniff` command.
/// Aggregates code smell findings (twins, dead parrots, crowds).
#[derive(Debug, Clone, Default)]
pub struct SniffOptions {
    /// Root directory to analyze (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Show only dead parrots (skip twins and crowds)
    pub dead_only: bool,

    /// Show only twins (skip dead parrots and crowds)
    pub twins_only: bool,

    /// Show only crowds (skip twins and dead parrots)
    pub crowds_only: bool,

    /// Include test files in analysis (default: false)
    pub include_tests: bool,

    /// Minimum crowd size to report (default: 2)
    pub min_crowd_size: Option<usize>,
}

/// Options for jq-style query mode (loct '.filter')
#[derive(Debug, Clone, Default)]
pub struct JqQueryOptions {
    /// The jq filter expression
    pub filter: String,
    /// Raw string output (-r)
    pub raw_output: bool,
    /// Compact JSON output (-c)
    pub compact_output: bool,
    /// Exit status mode (-e)
    pub exit_status: bool,
    /// String variable bindings: (name, value)
    pub string_args: Vec<(String, String)>,
    /// JSON variable bindings: (name, json_string)
    pub json_args: Vec<(String, String)>,
    /// Explicit snapshot path
    pub snapshot_path: Option<std::path::PathBuf>,
}

/// Options for the `impact` command.
#[derive(Debug, Clone, Default)]
pub struct ImpactCommandOptions {
    /// Target file path to analyze
    pub target: String,

    /// Maximum traversal depth (None = unlimited)
    pub depth: Option<usize>,

    /// Root directory (defaults to current directory)
    pub root: Option<PathBuf>,
}

/// Options for the `focus` command.
/// Focus on a directory - like slice but for directories.
#[derive(Debug, Clone, Default)]
pub struct FocusOptions {
    /// Target directory path
    pub target: String,

    /// Root directory (defaults to current directory)
    pub root: Option<PathBuf>,

    /// Include consumer files (files outside the directory that import it)
    pub consumers: bool,

    /// Maximum depth for external dependency traversal
    pub depth: Option<usize>,
}

/// Options for the `hotspots` command.
/// Shows import frequency heatmap - which files are core vs peripheral.
#[derive(Debug, Clone, Default)]
pub struct HotspotsOptions {
    /// Root directory (defaults to current directory)
    pub root: Option<PathBuf>,

    /// Minimum import count to show (default: 1)
    pub min_imports: Option<usize>,

    /// Maximum files to show (default: 50)
    pub limit: Option<usize>,

    /// Show only files with zero importers (leaf nodes)
    pub leaves_only: bool,

    /// Show coupling score (out-degree / files that import many others)
    pub coupling: bool,
}

/// Options for the `layoutmap` command.
/// Analyze CSS layout properties (z-index, position, display).
#[derive(Debug, Clone, Default)]
pub struct LayoutmapOptions {
    /// Root directory (defaults to current directory)
    pub root: Option<PathBuf>,

    /// Show only z-index values
    pub zindex_only: bool,

    /// Show only sticky/fixed position elements
    pub sticky_only: bool,

    /// Show only grid/flex layouts
    pub grid_only: bool,

    /// Minimum z-index threshold to report (default: 1)
    pub min_zindex: Option<i32>,

    /// Glob patterns to exclude (e.g., "**/prototype/**", "**/.obsidian/**")
    pub exclude: Vec<String>,
}

/// Options for the `zombie` command.
/// Find all zombie code (dead exports + orphan files + shadows).
#[derive(Debug, Clone, Default)]
pub struct ZombieOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Include test files in zombie detection (default: false)
    pub include_tests: bool,
}

/// Options for the `help` command.
#[derive(Debug, Clone, Default)]
pub struct HelpOptions {
    /// Show help for a specific command
    pub command: Option<String>,

    /// Show legacy flag documentation
    pub legacy: bool,

    /// Show full help (new + legacy)
    pub full: bool,
}

/// Query kind for the `query` command.
#[derive(Debug, Clone)]
pub enum QueryKind {
    /// Find files that import a given file
    WhoImports,
    /// Find where a symbol is defined
    WhereSymbol,
    /// Show what component a file belongs to
    ComponentOf,
}

/// Options for the `query` command.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Query kind
    pub kind: QueryKind,

    /// Target (file path or symbol name)
    pub target: String,
}

// ============================================================================
// Command Parsing Result
// ============================================================================

/// Result of parsing command-line arguments.
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// The parsed command
    pub command: Command,

    /// Global options
    pub global: GlobalOptions,

    /// Whether this was parsed from legacy flags (triggers deprecation warning)
    pub from_legacy: bool,

    /// If from legacy, the original invocation for the warning message
    pub legacy_invocation: Option<String>,

    /// If from legacy, the suggested new invocation
    pub suggested_invocation: Option<String>,
}

impl ParsedCommand {
    /// Create a new ParsedCommand for a modern invocation.
    pub fn new(command: Command, global: GlobalOptions) -> Self {
        Self {
            command,
            global,
            from_legacy: false,
            legacy_invocation: None,
            suggested_invocation: None,
        }
    }

    /// Create a new ParsedCommand for a legacy invocation.
    pub fn from_legacy(
        command: Command,
        global: GlobalOptions,
        legacy_invocation: String,
        suggested_invocation: String,
    ) -> Self {
        Self {
            command,
            global,
            from_legacy: true,
            legacy_invocation: Some(legacy_invocation),
            suggested_invocation: Some(suggested_invocation),
        }
    }

    /// Emit deprecation warning to stderr if this is a legacy invocation.
    ///
    /// Respects the `--quiet` flag by not emitting if quiet is set.
    pub fn emit_deprecation_warning(&self) {
        if self.from_legacy
            && !self.global.quiet
            && let (Some(old), Some(new)) = (&self.legacy_invocation, &self.suggested_invocation)
        {
            eprintln!(
                "[loct][deprecated] '{}' → '{}'. This alias will be removed in v1.0.",
                old, new
            );
        }
    }
}

// ============================================================================
// Help Text Generation
// ============================================================================

impl Command {
    /// Get the command name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Command::Auto(_) => "auto",
            Command::Scan(_) => "scan",
            Command::Tree(_) => "tree",
            Command::Slice(_) => "slice",
            Command::Find(_) => "find",
            Command::Dead(_) => "dead",
            Command::Cycles(_) => "cycles",
            Command::Commands(_) => "commands",
            Command::Routes(_) => "routes",
            Command::Events(_) => "events",
            Command::Info(_) => "info",
            Command::Lint(_) => "lint",
            Command::Report(_) => "report",
            Command::Help(_) => "help",
            Command::Version => "version",
            Command::Query(_) => "query",
            Command::Diff(_) => "diff",
            Command::Memex(_) => "memex",
            Command::Crowd(_) => "crowd",
            Command::Tagmap(_) => "tagmap",
            Command::Twins(_) => "twins",
            Command::Suppress(_) => "suppress",
            Command::Dist(_) => "dist",
            Command::Coverage(_) => "coverage",
            Command::Sniff(_) => "sniff",
            Command::JqQuery(_) => "jq",
            Command::Impact(_) => "impact",
            Command::Focus(_) => "focus",
            Command::Hotspots(_) => "hotspots",
            Command::Layoutmap(_) => "layoutmap",
            Command::Zombie(_) => "zombie",
        }
    }

    /// Get a short description of the command.
    pub fn description(&self) -> &'static str {
        match self {
            Command::Auto(_) => "Full auto-scan with stack detection (default)",
            Command::Scan(_) => "Build/update snapshot for current HEAD (supports --watch)",
            Command::Tree(_) => "Display LOC tree / structural overview",
            Command::Slice(_) => "Extract holographic context for a file",
            Command::Find(_) => "Search symbols/files with regex filters",
            Command::Dead(_) => "Detect unused exports / dead code",
            Command::Cycles(_) => "Detect circular imports",
            Command::Commands(_) => "Show Tauri command bridges (FE ↔ BE)",
            Command::Events(_) => "Show event flow and issues",
            Command::Info(_) => "Show snapshot metadata and project info",
            Command::Lint(_) => "Structural lint/policy checks",
            Command::Report(_) => "Generate HTML/JSON reports",
            Command::Help(_) => "Show help for commands",
            Command::Version => "Show version information",
            Command::Query(_) => "Query snapshot data (who-imports, where-symbol, component-of)",
            Command::Diff(_) => "Compare snapshots and show semantic delta",
            Command::Memex(_) => "Index analysis into AI memory (vector DB)",
            Command::Crowd(_) => "Detect functional crowds (similar files clustering)",
            Command::Tagmap(_) => "Unified search: files + crowd + dead around a keyword",
            Command::Twins(_) => "Show symbol registry and dead parrots (0 imports)",
            Command::Suppress(_) => "Manage false positive suppressions",
            Command::Routes(_) => "List backend/web routes (FastAPI/Flask)",
            Command::Dist(_) => "Analyze bundle distribution using source maps",
            Command::Coverage(_) => "Analyze test coverage gaps (structural coverage)",
            Command::Sniff(_) => "Sniff for code smells (twins + dead parrots + crowds)",
            Command::JqQuery(_) => "Query snapshot with jq-style filters (loct '.filter')",
            Command::Impact(_) => "Analyze impact of modifying/removing a file",
            Command::Focus(_) => "Extract holographic context for a directory",
            Command::Hotspots(_) => "Show import frequency heatmap (core vs peripheral)",
            Command::Layoutmap(_) => "Analyze CSS layout (z-index, position, grid/flex)",
            Command::Zombie(_) => "Find zombie code (dead exports + orphan files + shadows)",
        }
    }

    /// Generate the main help text listing all commands.
    pub fn format_help() -> String {
        let commands = [
            ("auto", "Full auto-scan with stack detection (default)"),
            ("agent", "Agent bundle JSON (alias for auto --agent-json)"),
            (
                "scan",
                "Build/update snapshot for current HEAD (supports --watch)",
            ),
            ("tree", "Display LOC tree / structural overview"),
            ("slice <path>", "Extract holographic context for a file"),
            ("find", "Search symbols/files with regex filters"),
            ("dead", "Detect unused exports / dead code"),
            ("cycles", "Detect circular imports"),
            ("commands", "Show Tauri command bridges (FE ↔ BE)"),
            ("routes", "List backend/web routes (FastAPI/Flask)"),
            ("events", "Show event flow and issues"),
            ("info", "Show snapshot metadata and project info"),
            ("lint", "Structural lint/policy checks"),
            ("report", "Generate HTML/JSON reports"),
            (
                "query <kind> <target>",
                "Query snapshot (who-imports, where-symbol, component-of)",
            ),
            (
                "impact <file>",
                "Analyze impact of modifying/removing a file",
            ),
            ("memex", "Index analysis into AI memory (vector DB)"),
            (
                "crowd [pattern]",
                "Detect functional crowds around a pattern",
            ),
            (
                "tagmap <keyword>",
                "Unified search: files + crowd + dead around keyword",
            ),
            ("twins", "Dead parrots, exact twins, barrel chaos analysis"),
            ("sniff", "Code smells (twins + dead parrots + crowds)"),
            (
                "dist <map> <src>",
                "Analyze bundle distribution using source maps",
            ),
            ("coverage", "Analyze test coverage gaps (structural)"),
            (
                "suppress <type> <sym>",
                "Mark findings as false positive (reviewed OK)",
            ),
            ("focus <dir>", "Extract holographic context for a directory"),
            ("hotspots", "Import frequency heatmap (core vs peripheral)"),
            (
                "layoutmap",
                "Analyze CSS layout (z-index, position, grid/flex)",
            ),
            (
                "zombie",
                "Find zombie code (dead exports + orphan files + shadows)",
            ),
        ];

        let mut help = String::new();
        help.push_str("loctree - AI-oriented Project Analyzer\n\n");
        help.push_str("USAGE:\n");
        help.push_str("    loct [COMMAND] [OPTIONS]\n");
        help.push_str("    loct                      # Auto-scan (default)\n");
        help.push_str("    loct <command> --help     # Command-specific help\n\n");
        help.push_str("COMMANDS:\n");

        for (name, desc) in commands {
            help.push_str(&format!("    {:<18} {}\n", name, desc));
        }

        help.push_str("\nGLOBAL OPTIONS:\n");
        help.push_str("    --json           Output as JSON (stdout only)\n");
        help.push_str("    --quiet          Suppress non-essential output\n");
        help.push_str("    --verbose        Show detailed progress\n");
        help.push_str("    --color <mode>   Color mode: auto|always|never\n");
        help.push_str("    --py-root <path> Additional Python package root for imports\n");
        help.push_str("    --help           Show this help\n");
        help.push_str("    --version        Show version\n");
        help.push_str("\nFor deprecated flags, run: loct --help-legacy\n");

        help
    }

    /// Generate help text for a specific subcommand (new CLI).
    pub fn format_command_help(command: &str) -> Option<&'static str> {
        match command {
            "auto" => Some(AUTO_HELP),
            "agent" => Some(AGENT_HELP),
            "scan" => Some(SCAN_HELP),
            "tree" => Some(TREE_HELP),
            "slice" => Some(SLICE_HELP),
            "find" => Some(FIND_HELP),
            "dead" | "unused" => Some(DEAD_HELP),
            "cycles" => Some(CYCLES_HELP),
            "commands" => Some(COMMANDS_HELP),
            "events" => Some(EVENTS_HELP),
            "info" => Some(INFO_HELP),
            "lint" => Some(LINT_HELP),
            "report" => Some(REPORT_HELP),
            "query" => Some(QUERY_HELP),
            "impact" => Some(IMPACT_HELP),
            "diff" => Some(DIFF_HELP),
            "memex" => Some(MEMEX_HELP),
            "crowd" => Some(CROWD_HELP),
            "tagmap" => Some(TAGMAP_HELP),
            "twins" => Some(TWINS_HELP),
            "routes" => Some(ROUTES_HELP),
            "dist" => Some(DIST_HELP),
            "coverage" => Some(COVERAGE_HELP),
            "sniff" => Some(SNIFF_HELP),
            "suppress" => Some(SUPPRESS_HELP),
            "focus" => Some(FOCUS_HELP),
            "hotspots" => Some(HOTSPOTS_HELP),
            "layoutmap" => Some(LAYOUTMAP_HELP),
            "zombie" => Some(ZOMBIE_HELP),
            _ => None,
        }
    }

    /// Generate legacy help text with migration hints.
    pub fn format_legacy_help() -> String {
        let mut help = String::new();
        help.push_str("loctree - Legacy Flag Reference\n\n");
        help.push_str("These flags are deprecated and will be removed in v1.0.\n");
        help.push_str("Please migrate to the new subcommand interface.\n\n");

        help.push_str("LEGACY FLAG              → NEW COMMAND\n");
        help.push_str("─────────────────────────────────────────────────\n");
        help.push_str("loct                     → loct auto (unchanged)\n");
        help.push_str("loct --tree              → loct tree\n");
        help.push_str("loct -A                  → loct report\n");
        help.push_str("loct -A --dead           → loct dead\n");
        help.push_str("loct -A --circular       → loct cycles\n");
        help.push_str("loct -A --entrypoints    → loct lint --entrypoints\n");
        help.push_str("loct -A --symbol NAME    → loct find --symbol NAME\n");
        help.push_str("loct -A --impact FILE    → loct find --impact FILE\n");
        help.push_str("loct --for-ai PATH       → loct slice PATH --json\n");
        help.push_str("loct slice PATH          → loct slice PATH (unchanged)\n");

        help.push_str("\nFor the new command reference, run: loct --help\n");

        help
    }
}

// Static help text per subcommand (kept concise but actionable)
const AUTO_HELP: &str = "loct auto - Full auto-scan with stack detection (default command)

USAGE:
    loct auto [OPTIONS] [PATHS...]
    loct [OPTIONS] [PATHS...]    # 'auto' is the default command

DESCRIPTION:
    Performs a comprehensive analysis of your codebase:
    - Detects project type and language stack automatically
    - Builds dependency graph and import relationships
    - Analyzes code structure and exports
    - Identifies potential issues (dead code, cycles, etc.)

OPTIONS:
    --full-scan          Force full rescan (ignore cache)
    --scan-all           Scan all files including hidden/ignored
    --for-agent-feed     Output optimized format for AI agents (JSONL stream)
    --agent-json         Emit a single agent bundle JSON (alias: loct agent)
    --no-duplicates      Hide duplicate export sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h           Show this help message

ARGUMENTS:
    [PATHS...]           Root directories to scan (default: current directory)

EXAMPLES:
    loct                         # Auto-scan current directory
    loct auto                    # Explicit auto command
    loct auto --full-scan        # Force full rescan
    loct auto src/ lib/          # Scan specific directories
    loct --for-agent-feed        # AI-optimized output (JSONL stream)
    loct --agent-json            # One-shot agent bundle JSON";

const AGENT_HELP: &str = "loct agent - Agent bundle JSON (shortcut for auto --agent-json)

USAGE:
    loct agent [PATHS...]

DESCRIPTION:
    Runs the auto scan and emits a single JSON tuned for AI agents:
    handlers, duplicates, dead exports, dynamic imports, cycles, and top files.
    The bundle is also saved to ./.loctree/agent.json for reuse.

OPTIONS:
    --full-scan          Force full rescan (ignore cache)
    --scan-all           Scan all files including hidden/ignored
    --help, -h           Show this help message

ARGUMENTS:
    [PATHS...]           Root directories to scan (default: current directory)

EXAMPLES:
    loct agent                   # Agent bundle for current directory
    loct agent src/              # Agent bundle for src/";

const SCAN_HELP: &str = "loct scan - Build/update snapshot for current HEAD

USAGE:
    loct scan [OPTIONS] [PATHS...]

DESCRIPTION:
    Scans the codebase and updates the internal snapshot database.
    Builds the dependency graph and prepares data for other commands.
    Unlike 'auto', it only builds the snapshot without extra analysis.

OPTIONS:
    --full-scan       Force full rescan, ignore cached data
    --scan-all        Include hidden and ignored files
    --watch           Watch for changes and re-scan automatically
    --help, -h        Show this help message

ARGUMENTS:
    [PATHS...]        Root directories to scan (default: current directory)

EXAMPLES:
    loct scan                    # Scan current directory
    loct scan --full-scan        # Force complete rescan
    loct scan src/ lib/          # Scan specific directories
    loct scan --scan-all         # Include all files (even hidden)
    loct scan --watch            # Watch mode with live refresh";

const TREE_HELP: &str = "loct tree - Display LOC tree / structural overview

USAGE:
    loct tree [OPTIONS] [PATHS...]

DESCRIPTION:
    Hierarchical tree of the codebase with LOC metrics.
    Similar to 'tree' but with LOC and gitignore handling.

OPTIONS:
    --depth <N>, -L <N>    Maximum depth (default: unlimited)
    --summary [N]          Show top N largest items (default: 5)
    --top [N]              Only show top N largest items (default: 50)
    --loc <N>              Only show items with LOC >= N
    --min-loc <N>          Alias for --loc
    --show-hidden, -H      Include hidden files/directories
    --find-artifacts       Highlight build/generated artifacts
    --show-ignored         Show gitignored files
    --help, -h             Show this help message

ARGUMENTS:
    [PATHS...]             Roots to analyze (default: current directory)

EXAMPLES:
    loct tree                       # Full tree
    loct tree --depth 3             # Limit depth
    loct tree --summary 10          # Top 10 largest
    loct tree --loc 100             # LOC threshold
    loct tree src/ --show-hidden    # Include dotfiles";

const SLICE_HELP: &str = "loct slice - Extract file + dependencies for AI context

USAGE:
    loct slice <TARGET_PATH> [OPTIONS]

DESCRIPTION:
    Extracts a 'holographic slice' - the target file plus all its dependencies.
    Perfect for feeding focused context to AI assistants.

    Shows what the file USES, not what USES it.
    For reverse dependencies, use --consumers or 'loct query who-imports'.

OPTIONS:
    --consumers, -c    Include reverse dependencies (files that import this)
    --depth <N>        Maximum dependency depth to traverse
    --root <PATH>      Project root for resolving imports
    --rescan           Force snapshot update (includes new/uncommitted files)
    --help, -h         Show this help message

EXAMPLES:
    loct slice src/main.rs              # File + its dependencies
    loct slice src/utils.ts --consumers # Include reverse deps
    loct slice lib/api.ts --depth 2     # Limit to 2 levels
    loct slice src/app.tsx --json       # JSON output for AI tools
    loct slice src/new-file.ts --rescan # Slice a newly created file

RELATED COMMANDS:
    loct query who-imports <file>    Find all importers
    loct auto --for-agent-feed       Full codebase context
    loct focus <dir>                 Slice for a directory";

const FIND_HELP: &str = "loct find - Semantic search for symbols by name pattern

USAGE:
    loct find [QUERY] [OPTIONS]

DESCRIPTION:
    Semantic search for symbols (functions, classes, types) matching a name pattern.
    Uses regex patterns to match symbol names in your codebase.

    NOT impact analysis - for dependency impact, use your editor's LSP or 'loct impact'.
    NOT dead code detection - use 'loct dead' or 'loct twins' for that.

OPTIONS:
    --symbol <PATTERN>   Search for symbols matching regex
    --file <PATTERN>     Search for files matching regex
    --similar <SYMBOL>   Find symbols with similar names (fuzzy)
    --dead               Only show dead/unused symbols
    --exported           Only show exported symbols
    --lang <LANG>        Filter by language (ts, rs, js, py, etc.)
    --limit <N>          Maximum results to show
    --help, -h           Show this help message

EXAMPLES:
    loct find Patient              # Find symbols containing \"Patient\"
    loct find --symbol \".*Config$\" # Regex: symbols ending with Config
    loct find --file \"utils\"       # Files containing \"utils\" in path
    loct find --dead --exported    # Dead exported symbols

RELATED COMMANDS:
    loct dead              Find unused exports / dead code
    loct twins             Find duplicate exports and dead parrots
    loct slice <file>      Extract file dependencies
    loct query who-imports Show what imports a file";

const DEAD_HELP: &str = "loct dead - Detect unused exports / dead code

USAGE:
    loct dead [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects unused exports with confidence levels and optional
    inclusion of tests/helpers. Integrates with quick wins.

OPTIONS:
    --confidence <lvl>   normal|high (default: normal)
    --top <N>            Limit results to top N (default: 20)
    --full, --all        Show all results (ignore --top limit)
    --with-tests         Include test files
    --with-helpers       Include helper files
    --with-shadows       Detect shadow exports (same symbol, multiple files)
    --help, -h           Show this help message

EXAMPLES:
    loct dead
    loct dead --confidence high
    loct dead --with-tests
    loct dead --with-shadows";

const CYCLES_HELP: &str = "loct cycles - Detect circular import chains

USAGE:
    loct cycles [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects circular dependencies in your import graph.
    Example: A → B → C → A

    Circular imports cause:
    - Runtime initialization errors
    - Build/bundling failures
    - Flaky test behavior

OPTIONS:
    --path <PATTERN>     Filter to files matching pattern
    --help, -h           Show this help message

EXAMPLES:
    loct cycles                # Detect all cycles
    loct cycles src/           # Only analyze src/
    loct cycles --json         # JSON output for CI

RELATED COMMANDS:
    loct slice <file>       See what a file depends on
    loct query who-imports  Find reverse dependencies
    loct lint --fail        Run as CI check";

const COMMANDS_HELP: &str = "loct commands - Tauri FE↔BE handler coverage analysis

USAGE:
    loct commands [OPTIONS] [PATHS...]

DESCRIPTION:
    Analyzes Tauri command bridge contracts between frontend and backend.

    Detects:
    - Missing handlers: FE calls invoke('cmd') but no BE #[tauri::command]
    - Unused handlers: BE has #[tauri::command] but FE never calls it
    - Matched handlers: Both FE and BE exist (healthy)

OPTIONS:
    --name <PATTERN>     Filter to commands matching pattern
    --missing-only       Show only missing handlers
    --unused-only        Show only unused handlers
    --limit <N>          Maximum results to show
    --help, -h           Show this help message

EXAMPLES:
    loct commands                    # Full coverage report
    loct commands --missing-only     # Only missing handlers
    loct commands --json --missing   # JSON for CI

RELATED COMMANDS:
    loct events        Analyze Tauri event flow
    loct lint --tauri  Full Tauri contract validation";

const EVENTS_HELP: &str = "loct events - Show event flow and issues

USAGE:
    loct events [OPTIONS] [PATHS...]

DESCRIPTION:
    Analyzes event emit/listen pairs, ghost events, and race conditions.

OPTIONS:
    --races             Enable race detection (async/await gaps)
    --no-duplicates     Hide duplicate sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h          Show this help message

EXAMPLES:
    loct events
    loct events --races";

const ROUTES_HELP: &str = "loct routes - List backend/web routes (FastAPI/Flask)

USAGE:
    loct routes [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects Python web routes based on common decorators:
    - FastAPI: @app.get/post/put/delete/patch, @router.*, @api_router.*
    - Flask:   @app.route, @blueprint.route, .route(...)

OPTIONS:
    --framework <NAME>   Filter by framework label (fastapi, flask)
    --path <PATTERN>     Filter by route path substring
    --help, -h           Show this help message

EXAMPLES:
    loct routes
    loct routes --framework fastapi
    loct routes --path /patients";

const INFO_HELP: &str = "loct info - Show snapshot metadata and project info

USAGE:
    loct info

DESCRIPTION:
    Prints snapshot metadata, detected stack, and analysis summary.

OPTIONS:
    --help, -h          Show this help message";

const LINT_HELP: &str = "loct lint - Structural lint and policy checks

USAGE:
    loct lint [OPTIONS] [PATHS...]

DESCRIPTION:
    Runs structural linting: entrypoints, handlers, ghost events, races.

OPTIONS:
    --entrypoints        List entrypoints
    --sarif              Emit SARIF
    --tauri              Apply Tauri presets
    --fail               Exit non-zero on findings
    --no-duplicates      Hide duplicate sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h           Show this help message

EXAMPLES:
    loct lint
    loct lint --fail --sarif";

const REPORT_HELP: &str = "loct report - Generate HTML/JSON reports

USAGE:
    loct report [OPTIONS] [PATHS...]

DESCRIPTION:
    Runs full analysis and emits HTML/JSON/SARIF reports.

OPTIONS:
    --output <FILE>      Output HTML path
    --serve              Serve report locally
    --port <N>           Port for --serve
    --editor <NAME>      Editor integration (code/cursor/jetbrains)
    --help, -h           Show this help message

EXAMPLES:
    loct report --output report.html
    loct report --serve --port 4173";

const QUERY_HELP: &str = "loct query - Graph queries (who-imports, who-exports, etc.)

USAGE:
    loct query <KIND> <TARGET>

DESCRIPTION:
    Query the import graph and symbol index for specific relationships.
    Targeted queries against the dependency graph built by 'loct scan'.

QUERY KINDS:
    who-imports <FILE>      Find all files that import the file (reverse deps)
    where-symbol <SYMBOL>   Find where a symbol is defined/exported
    component-of <FILE>     Show which component/module contains the file

OPTIONS:
    --help, -h              Show this help message

EXAMPLES:
    loct query who-imports src/utils.ts       # What imports utils.ts?
    loct query where-symbol PatientRecord     # Where is it defined?
    loct query component-of src/ui/Button.tsx # What owns Button?

RELATED COMMANDS:
    loct slice <file>           Show what a file depends on
    loct find --symbol <name>   Search for symbols by pattern
    loct dead                   Find symbols with 0 imports";

const IMPACT_HELP: &str = "loct impact - Analyze impact of modifying/removing a file

USAGE:
    loct impact <FILE> [OPTIONS]

DESCRIPTION:
    Shows \"what breaks if you modify or remove this file\" by traversing
    the reverse dependency graph. Finds all direct and transitive consumers.

    This is different from 'query who-imports':
    - who-imports: Finds direct importers only
    - impact: Finds ALL affected files (direct + transitive)

    Useful for:
    - Understanding change impact before refactoring
    - Identifying critical files (high fan-out)
    - Safe deletion analysis

OPTIONS:
    --depth <N>          Limit traversal depth (default: unlimited)
    --root <PATH>        Project root (default: current directory)
    --json               Output as JSON for agent consumption
    --help, -h           Show this help message

ARGUMENTS:
    <FILE>               Path to the file to analyze (required)

EXAMPLES:
    loct impact src/utils.ts                # Full impact analysis
    loct impact src/api.ts --depth 2        # Limit to 2 levels deep
    loct impact lib/helpers.ts --json       # JSON output
    loct impact src/core.ts --root ./       # Specify project root

OUTPUT FORMAT:
    Direct consumers (5 files):
      src/app.ts (import)
      src/lib.ts (import)
      ...

    Transitive impact (23 files total):
      [depth 2] src/page.tsx (import)
      ...

    ⚠ Removing this file would break 28 files (max depth: 3)";

const DIFF_HELP: &str = "loct diff - Compare snapshots between branches/commits

USAGE:
    loct diff --since <SNAPSHOT> [--to <SNAPSHOT>] [OPTIONS]

DESCRIPTION:
    Compares two code snapshots and shows semantic differences.

    Unlike git diff (line changes), this shows structural changes:
    - New/removed files and symbols
    - Import graph changes
    - New dead code introduced (regressions)
    - New circular dependencies

OPTIONS:
    --since <SNAPSHOT>   Base snapshot to compare from (required)
    --to <SNAPSHOT>      Target snapshot (default: current working tree)
    --auto-scan-base     Auto-create git worktree and scan target branch
    --problems-only      Show only regressions (new dead code, new cycles)
    --help, -h           Show this help message

EXAMPLES:
    loct diff --since main                    # Compare main to working tree
    loct diff --since HEAD~1                  # Compare to previous commit
    loct diff --since main --auto-scan-base   # Auto-scan main branch
    loct diff --since v1.0.0 --to v2.0.0      # Compare two tags

RELATED COMMANDS:
    loct scan             Run scan to create snapshot
    loct auto --full-scan Force full rescan";

const MEMEX_HELP: &str = "loct memex - Index analysis into AI memory (vector DB)

USAGE:
    loct memex [OPTIONS]

DESCRIPTION:
    Pushes analysis artifacts to vector memory for agents.

OPTIONS:
    --help, -h          Show this help message";

const CROWD_HELP: &str = "loct crowd - Detect functional crowds (similar files clustering)

USAGE:
    loct crowd [PATTERN]

DESCRIPTION:
    Groups related files around a seed pattern (name or path fragment).

OPTIONS:
    --help, -h          Show this help message

EXAMPLES:
    loct crowd cache
    loct crowd session";

const TAGMAP_HELP: &str = "loct tagmap - Unified search around a keyword

USAGE:
    loct tagmap <KEYWORD> [OPTIONS]

DESCRIPTION:
    Aggregates three analyses into one view:
    1. FILES:  All files with keyword in path or name
    2. CROWD:  Functional clustering around the keyword
    3. DEAD:   Dead exports related to the keyword

    Perfect for understanding everything about a domain/feature at once.

OPTIONS:
    --include-tests    Include test files in analysis
    --limit <N>        Maximum results per section (default: 20)
    --json             Output as JSON for AI tools
    --help, -h         Show this help message

ARGUMENTS:
    <KEYWORD>          Keyword to search for (required)

EXAMPLES:
    loct tagmap patient           # Everything about 'patient' feature
    loct tagmap auth              # Auth-related files, crowds, dead code
    loct tagmap message --json    # JSON output for AI processing
    loct tagmap api --limit 10    # Limit results

OUTPUT FORMAT:
    === TAGMAP: 'patient' ===

    FILES MATCHING KEYWORD (12):
      src/features/patients/PatientsList.tsx
      src/hooks/usePatient.ts
      ...

    CROWD ANALYSIS (8 files):
      Score: 7.2/10
      Members: PatientsList, PatientDetail, PatientForm...
      Issues: Consider consolidating similar files

    DEAD EXPORTS (3):
      oldPatientHandler in src/api/patients.ts
      PatientV1 in src/types/patient.ts
      ...

RELATED COMMANDS:
    loct crowd <pattern>    Detailed crowd analysis
    loct dead               All dead exports
    loct find <query>       Symbol/file search
    loct focus <dir>        Directory-level context";

const TWINS_HELP: &str = "loct twins - Find dead parrots (0 imports) and duplicate exports

USAGE:
    loct twins [OPTIONS] [PATH]

DESCRIPTION:
    Detects semantic issues in your export/import graph:

    Dead Parrots:   Exports with 0 imports anywhere in the codebase
                    (Monty Python reference - code that looks alive but isn't used)

    Exact Twins:    Same symbol name exported from multiple files
                    (can cause import confusion)

    Barrel Chaos:   Re-export anti-patterns
                    (missing index.ts, deep re-export chains)

    This is a code smell detector - findings are hints, not verdicts.

OPTIONS:
    --path <DIR>           Root directory to analyze
    --dead-only            Show only dead parrots (0 imports)
    --include-tests        Include test files (excluded by default)
    --include-suppressed   Show suppressed findings too
    --help, -h             Show this help message

EXAMPLES:
    loct twins                    # Full analysis
    loct twins --dead-only        # Only exports with 0 imports
    loct twins src/               # Analyze specific directory
    loct twins --include-tests    # Include test files
    loct twins --include-suppressed  # Include suppressed items

SUPPRESSION:
    Mark findings as false positives (they won't show in subsequent runs):
    loct suppress twins <symbol>              # Suppress a twin
    loct suppress twins <symbol> --file <f>   # Suppress only in specific file
    loct suppress --list                      # Show all suppressions
    loct suppress --clear                     # Clear all suppressions

RELATED COMMANDS:
    loct dead              Detailed dead code analysis
    loct sniff             Aggregate code smell analysis
    loct suppress          Manage false positive suppressions
    loct find --dead       Search for specific dead symbols";

const DIST_HELP: &str = "loct dist - Analyze bundle distribution using source maps

USAGE:
    loct dist --source-map <PATH> --src <DIR>

DESCRIPTION:
    Compares source code exports with bundled JavaScript to find truly dead exports.
    Uses source maps to detect code that was completely tree-shaken out by the bundler.

    This is different from regular dead code detection:
    - Regular: Finds exports with 0 imports in your source code
    - Dist: Finds exports removed from the production bundle

    Useful for:
    - Validating tree-shaking effectiveness
    - Finding code that can be safely removed
    - Understanding bundle size optimizations

OPTIONS:
    --source-map <PATH>    Path to source map file (e.g., dist/main.js.map)
    --src <DIR>            Source directory to scan (e.g., src/)
    --help, -h             Show this help message

EXAMPLES:
    loct dist --source-map dist/main.js.map --src src/
    loct dist --source-map build/app.js.map --src app/src/

OUTPUT (JSON):
    {
      \"sourceExports\": 500,
      \"bundledExports\": 120,
      \"deadExports\": [
        { \"file\": \"src/utils.ts\", \"line\": 42, \"name\": \"unusedHelper\" }
      ],
      \"reduction\": \"76%\"
    }";

const COVERAGE_HELP: &str = "loct coverage - Analyze test coverage gaps (structural coverage)

USAGE:
    loct coverage [OPTIONS] [PATHS...]

DESCRIPTION:
    Performs structural test coverage analysis by cross-referencing:
    - Frontend invoke/emit calls (what the app uses)
    - Backend handlers and events (what the app exposes)
    - Test file imports (what tests actually cover)

    Unlike line coverage tools, this shows:
    - Which handlers have no corresponding tests
    - Which events are emitted but never tested
    - Which exports are tested but never used in production

    This is semantic coverage - not 'how many lines' but 'what functionality'.

OPTIONS:
    --handlers-only       Only show handler gaps (skip events/exports)
    --events-only         Only show event gaps (skip handlers/exports)
    --min-severity <LVL>  Filter by minimum severity: critical, high, medium, low
    --json                Output as JSON for programmatic use
    --help, -h            Show this help message

ARGUMENTS:
    [PATHS...]            Root directories to scan (default: current directory)

EXAMPLES:
    loct coverage                          # Show all coverage gaps
    loct coverage --handlers-only          # Focus on untested handlers
    loct coverage --min-severity high      # Only critical/high issues
    loct coverage --json                   # Machine-readable output

OUTPUT:
    Groups findings by severity:
    - CRITICAL: Handlers without any test (used in production!)
    - HIGH: Events emitted but never tested
    - MEDIUM: Exports without test imports
    - LOW: Tests that import unused code

    Each gap shows the source location and usage context.";

const SNIFF_HELP: &str = "loct sniff - Sniff for code smells (aggregate analysis)

USAGE:
    loct sniff [OPTIONS]

DESCRIPTION:
    Aggregates all smell-level findings worth checking:

    Twins:        Same symbol name in multiple files
                  - Can cause import confusion

    Dead Parrots: Exports with 0 imports
                  - Potentially unused code

    Crowds:       Files with similar dependency patterns
                  - Possible duplication or fragmentation

    Output is friendly and non-judgmental. These are hints, not verdicts.

OPTIONS:
    --path <DIR>           Root directory to analyze (default: current directory)
    --dead-only            Show only dead parrots (skip twins and crowds)
    --twins-only           Show only twins (skip dead parrots and crowds)
    --crowds-only          Show only crowds (skip twins and dead parrots)
    --include-tests        Include test files in analysis (default: false)
    --min-crowd-size <N>   Minimum crowd size to report (default: 2)
    --json                 Output as JSON for programmatic use
    --help, -h             Show this help message

EXAMPLES:
    loct sniff                    # Full code smell analysis
    loct sniff --dead-only        # Only dead parrots
    loct sniff --twins-only       # Only duplicate names
    loct sniff --crowds-only      # Only similar file clusters
    loct sniff --include-tests    # Include test files
    loct sniff --json             # Machine-readable output

OUTPUT:
    Aggregates three types of code smells:
    - TWINS: Same symbol exported from multiple files
    - DEAD PARROTS: Exports with 0 imports
    - CROWDS: Files clustering around similar functionality

    Each section provides actionable suggestions for consolidation or cleanup.";

const SUPPRESS_HELP: &str = "loct suppress - Mark findings as false positives

USAGE:
    loct suppress <type> <symbol> [OPTIONS]
    loct suppress --list
    loct suppress --clear

DESCRIPTION:
    Manages false positive suppressions so reviewed findings don't appear
    in subsequent runs. Suppressions are stored in .loctree/suppressions.toml.

    Use this when you've reviewed a finding and determined it's intentional:
    - FE/BE type mirrors (same type defined in TypeScript and Rust)
    - Intentional re-exports for public APIs
    - Entry points that appear 'dead' but are used externally

TYPES:
    twins          Exact twin (same symbol in multiple files)
    dead_parrot    Dead parrot (export with 0 imports)
    dead_export    Dead export (unused export)
    circular       Circular import

OPTIONS:
    --file <PATH>      Only suppress in specific file (default: all files)
    --reason <TEXT>    Document why this is a false positive
    --list             Show all current suppressions
    --clear            Remove all suppressions
    --help, -h         Show this help message

EXAMPLES:
    loct suppress twins Message              # Suppress 'Message' twin everywhere
    loct suppress twins Message --file src/types.ts  # Only in specific file
    loct suppress dead_parrot unusedHelper --reason 'Used via dynamic import'
    loct suppress --list                     # View all suppressions
    loct suppress --clear                    # Reset suppressions

STORAGE:
    Suppressions are stored in .loctree/suppressions.toml
    Commit this file to share suppressions with your team.

RELATED COMMANDS:
    loct twins         Find twins and dead parrots (--include-suppressed to show all)
    loct dead          Find unused exports
    loct sniff         Aggregate smell analysis";

const FOCUS_HELP: &str = "loct focus - Extract holographic context for a directory

USAGE:
    loct focus <DIRECTORY> [OPTIONS]

DESCRIPTION:
    Like 'slice' but for directories. Extracts a holographic view of a directory:

    Core:       All files within the target directory
    Internal:   Import edges between files inside the directory
    Deps:       External files imported by core (outside the directory)
    Consumers:  Files outside the directory that import core files

    Perfect for understanding feature modules like 'src/features/patients/'.

OPTIONS:
    --consumers, -c    Include files that import from this directory
    --depth <N>        Maximum depth for external dependency traversal
    --root <PATH>      Project root (default: current directory)
    --json             Output as JSON for agent consumption
    --help, -h         Show this help message

ARGUMENTS:
    <DIRECTORY>        Path to the directory to analyze (required)

EXAMPLES:
    loct focus src/features/patients/           # Focus on patients feature
    loct focus src/components/ --consumers      # Include external consumers
    loct focus lib/utils/ --depth 1             # Limit external dep depth
    loct focus src/api/ --json                  # JSON output for AI tools

OUTPUT FORMAT:
    Focus: src/features/patients/

    Core (12 files, 2,340 LOC):
      src/features/patients/index.ts
      src/features/patients/PatientsList.tsx
      ...

    Internal edges: 18 imports within directory

    External Deps (8 files, 890 LOC):
      [d1] src/components/Button.tsx
      ...

    Consumers (3 files, 450 LOC):
      src/App.tsx
      ...

    Total: 23 files, 3,680 LOC

RELATED COMMANDS:
    loct slice <file>       Extract context for a single file
    loct impact <file>      Show what breaks if you change a file
    loct crowd <pattern>    Find files clustering around a pattern";

const HOTSPOTS_HELP: &str = "loct hotspots - Import frequency heatmap (core vs peripheral)

USAGE:
    loct hotspots [OPTIONS]

DESCRIPTION:
    Ranks files by how often they are imported (in-degree) to identify:

    CORE:       Files imported by 10+ others (critical infrastructure)
    SHARED:     Files imported by 3-9 others (shared utilities)
    PERIPHERAL: Files imported by 1-2 others (feature-specific)
    LEAF:       Files with 0 importers (entry points or dead code)

    This helps AI agents understand which files are risky to modify
    (high in-degree = many dependents) vs safe to refactor (low in-degree).

OPTIONS:
    --min <N>              Minimum import count to show (default: 1)
    --limit <N>            Maximum files to show (default: 50)
    --leaves               Show only leaf nodes (0 importers)
    --coupling             Include out-degree (files that import many others)
    --root <PATH>          Project root (default: current directory)
    --json                 Output as JSON for agent consumption
    --help, -h             Show this help message

EXAMPLES:
    loct hotspots                    # Show top 50 most-imported files
    loct hotspots --limit 20         # Top 20 only
    loct hotspots --leaves           # Find leaf nodes (entry points / dead)
    loct hotspots --coupling         # Show both in-degree and out-degree
    loct hotspots --min 5            # Only files with 5+ importers
    loct hotspots --json             # JSON output for AI tools

OUTPUT FORMAT:
    Import Hotspots (42 files analyzed)

    CORE (10+ importers):
      [32] src/utils/helpers.ts           # hub module
      [18] src/components/Button.tsx

    SHARED (3-9 importers):
      [7]  src/hooks/useAuth.ts
      [5]  src/api/client.ts

    PERIPHERAL (1-2 importers):
      [2]  src/features/login/form.tsx
      [1]  src/features/login/types.ts

    LEAF (0 importers):
      src/pages/index.tsx               # entry point
      src/features/old/legacy.ts        # possibly dead

    With --coupling:
      [in:32 out:3]  src/utils/helpers.ts    # hub, low coupling
      [in:2  out:15] src/features/main.tsx   # feature root, high coupling

RELATED COMMANDS:
    loct dead               Find unused exports
    loct impact <file>      Show what breaks if you modify a file
    loct focus <dir>        Extract context for a directory";

const LAYOUTMAP_HELP: &str = "loct layoutmap - Analyze CSS layout properties

USAGE:
    loct layoutmap [OPTIONS]

DESCRIPTION:
    Extracts and analyzes layout-related CSS properties from your codebase:

    Z-INDEX:    Shows all z-index values across CSS/SCSS files, sorted by value.
                Helps identify layering conflicts and understand UI stacking.

    POSITION:   Lists sticky/fixed positioned elements.
                Useful for understanding what elements persist during scroll.

    DISPLAY:    Identifies grid/flex layouts and their locations.
                Maps out the layout architecture of your components.

OPTIONS:
    --zindex-only          Show only z-index values
    --sticky-only          Show only sticky/fixed position elements
    --grid-only            Show only grid/flex layouts
    --min-zindex <N>       Filter z-index values >= N (default: show all)
    --exclude <PATTERN>    Exclude paths matching glob (can be repeated)
    --root <PATH>          Project root (default: current directory)
    --json                 Output as JSON for agent consumption
    --help, -h             Show this help message

EXAMPLES:
    loct layoutmap                  # Full CSS layout analysis
    loct layoutmap --zindex-only    # Only z-index hierarchy
    loct layoutmap --sticky-only    # Only sticky/fixed elements
    loct layoutmap --min-zindex 100 # High z-index values (likely overlays)
    loct layoutmap --exclude .obsidian --exclude prototype  # Skip dirs
    loct layoutmap --json           # JSON output for AI tools

OUTPUT FORMAT:
    Z-INDEX HIERARCHY:
      [9999] src/components/Modal.css:15       .modal-overlay
      [1000] src/components/Toast.css:8        .toast-container
      [ 100] src/components/Dropdown.css:23    .dropdown-menu
      [  10] src/components/Header.css:5       .header

    STICKY/FIXED ELEMENTS:
      [fixed]  src/components/Header.css:12    .header
      [sticky] src/components/Sidebar.css:5    .sidebar-nav

    GRID/FLEX LAYOUTS:
      [grid]   src/layouts/Dashboard.css:8     .dashboard-grid
      [flex]   src/components/Card.css:3       .card-content

RELATED COMMANDS:
    loct crowd              Find functionally similar components
    loct find <pattern>     Search for CSS selectors or properties";

const ZOMBIE_HELP: &str = "loct zombie - Find all zombie code (combined analysis)

USAGE:
    loct zombie [OPTIONS] [PATHS...]

DESCRIPTION:
    Combines three sources of dead code into one actionable report:

    DEAD EXPORTS:     Unused exports detected by dead code analysis
                      (symbols with 0 imports)

    ORPHAN FILES:     Files with 0 importers (not imported by any other file)
                      Entry points are OK, but others might be dead

    SHADOW EXPORTS:   Same symbol exported by multiple files where some
                      have 0 imports (likely consolidation candidates)

    This is a comprehensive zombie hunter - finds all forms of potentially
    dead code in a single scan.

OPTIONS:
    --include-tests    Include test files in analysis (default: false)
    --json             Output as JSON for programmatic use
    --help, -h         Show this help message

ARGUMENTS:
    [PATHS...]         Root directories to scan (default: current directory)

EXAMPLES:
    loct zombie                    # Find all zombie code
    loct zombie --include-tests    # Include test files
    loct zombie src/               # Analyze specific directory
    loct zombie --json             # Machine-readable output

OUTPUT FORMAT:
    🧟 Zombie Code Report

    Dead Exports (3):
      src/utils/old.ts:15 - unusedFunction
      src/hooks/legacy.ts:8 - useLegacyHook
      ...

    Orphan Files (0 importers, 2):
      src/features/patients/PatientsList.tsx (504 LOC)
      src/components/deprecated/OldButton.tsx (89 LOC)

    Shadow Exports (1):
      conversationHostStore exported by 2 files, 1 dead

    Total: 6 zombie items, ~950 LOC to review

RELATED COMMANDS:
    loct dead               Detailed dead export analysis
    loct twins              Dead parrots and semantic duplicates
    loct hotspots --leaves  Find leaf nodes (0 importers)
    loct sniff              Code smell analysis";

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_default_is_auto() {
        let cmd = Command::default();
        assert_eq!(cmd.name(), "auto");
    }

    #[test]
    fn test_command_names() {
        assert_eq!(Command::Scan(ScanOptions::default()).name(), "scan");
        assert_eq!(Command::Tree(TreeOptions::default()).name(), "tree");
        assert_eq!(Command::Slice(SliceOptions::default()).name(), "slice");
        assert_eq!(Command::Dead(DeadOptions::default()).name(), "dead");
        assert_eq!(Command::Cycles(CyclesOptions::default()).name(), "cycles");
    }

    #[test]
    fn test_parsed_command_deprecation_warning() {
        let cmd = ParsedCommand::from_legacy(
            Command::Dead(DeadOptions::default()),
            GlobalOptions::default(),
            "loct -A --dead".to_string(),
            "loct dead".to_string(),
        );
        assert!(cmd.from_legacy);
        assert_eq!(cmd.legacy_invocation, Some("loct -A --dead".to_string()));
    }

    #[test]
    fn test_help_format_contains_commands() {
        let help = Command::format_help();
        assert!(help.contains("auto"));
        assert!(help.contains("scan"));
        assert!(help.contains("tree"));
        assert!(help.contains("slice"));
        assert!(help.contains("dead"));
        assert!(help.contains("cycles"));
    }

    #[test]
    fn test_legacy_help_format_contains_mappings() {
        let help = Command::format_legacy_help();
        assert!(help.contains("--tree"));
        assert!(help.contains("-A --dead"));
        assert!(help.contains("loct dead"));
    }

    #[test]
    fn test_command_specific_help_exists() {
        let tree_help = Command::format_command_help("tree").unwrap();
        assert!(tree_help.contains("loct tree"));
        assert!(Command::format_command_help("unknown").is_none());
    }
}
