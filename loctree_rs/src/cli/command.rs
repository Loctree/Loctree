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

    /// Show symbol registry and dead parrots (semantic duplicate detection).
    ///
    /// Builds a registry of all exported symbols and their import counts.
    /// Dead parrots are symbols with 0 imports - candidates for removal.
    Twins(TwinsOptions),

    /// Analyze bundle distribution using source maps.
    ///
    /// Compares source exports with bundled code to find truly dead exports
    /// that were tree-shaken out by the bundler.
    Dist(DistOptions),
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

/// Options for the `twins` command.
/// Shows symbol registry and dead parrots (0 import count).
#[derive(Debug, Clone, Default)]
pub struct TwinsOptions {
    /// Root directory to analyze (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Show only dead parrots (symbols with 0 imports)
    pub dead_only: bool,
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
            Command::Twins(_) => "twins",
            Command::Dist(_) => "dist",
        }
    }

    /// Get a short description of the command.
    pub fn description(&self) -> &'static str {
        match self {
            Command::Auto(_) => "Full auto-scan with stack detection (default)",
            Command::Scan(_) => "Build/update snapshot for current HEAD",
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
            Command::Twins(_) => "Show symbol registry and dead parrots (0 imports)",
            Command::Routes(_) => "List backend/web routes (FastAPI/Flask)",
            Command::Dist(_) => "Analyze bundle distribution using source maps",
        }
    }

    /// Generate the main help text listing all commands.
    pub fn format_help() -> String {
        let commands = [
            ("auto", "Full auto-scan with stack detection (default)"),
            ("agent", "Agent bundle JSON (alias for auto --agent-json)"),
            ("scan", "Build/update snapshot for current HEAD"),
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
            ("memex", "Index analysis into AI memory (vector DB)"),
            (
                "crowd [pattern]",
                "Detect functional crowds around a pattern",
            ),
            ("twins", "Dead parrots, exact twins, barrel chaos analysis"),
            (
                "dist <map> <src>",
                "Analyze bundle distribution using source maps",
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
            "diff" => Some(DIFF_HELP),
            "memex" => Some(MEMEX_HELP),
            "crowd" => Some(CROWD_HELP),
            "twins" => Some(TWINS_HELP),
            "routes" => Some(ROUTES_HELP),
            "dist" => Some(DIST_HELP),
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
    --help, -h        Show this help message

ARGUMENTS:
    [PATHS...]        Root directories to scan (default: current directory)

EXAMPLES:
    loct scan                    # Scan current directory
    loct scan --full-scan        # Force complete rescan
    loct scan src/ lib/          # Scan specific directories
    loct scan --scan-all         # Include all files (even hidden)";

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

const SLICE_HELP: &str = "loct slice - Extract holographic context for a file

USAGE:
    loct slice <FILE> [OPTIONS]

DESCRIPTION:
    Builds a context slice for a specific file/component:
    - Shows dependencies and consumers
    - Includes symbol exports/imports
    - Designed for AI agents and code review

OPTIONS:
    --consumers        Include files that import the target
    --root <PATH>      Project root (default: current directory)
    --help, -h         Show this help message

EXAMPLES:
    loct slice src/foo.ts
    loct slice src/foo.ts --consumers
    loct slice src/foo.ts --root ./packages/app";

const FIND_HELP: &str = "loct find - Search symbols/files with regex filters

USAGE:
    loct find [OPTIONS] <QUERY>

DESCRIPTION:
    Semantic search across the snapshot:
    - Symbols, files, impacts, similar components
    - Supports dead-only/exported-only filters

OPTIONS:
    --symbol <NAME>     Exact symbol search
    --impact <FILE>     Show files impacted by FILE
    --similar <NAME>    Find similar components/symbols
    --dead-only         Filter to dead symbols
    --exported-only     Filter to exported symbols
    --lang <LANG>       Restrict to language (ts, rs, py, etc.)
    --limit <N>         Result limit
    --help, -h          Show this help message

EXAMPLES:
    loct find useAuth
    loct find --symbol MyType
    loct find --impact src/foo.ts
    loct find --similar Button";

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
    --help, -h           Show this help message

EXAMPLES:
    loct dead
    loct dead --confidence high
    loct dead --with-tests";

const CYCLES_HELP: &str = "loct cycles - Detect circular imports

USAGE:
    loct cycles [PATHS...]

DESCRIPTION:
    Finds import cycles using the dependency graph. Supports Rust/TS/Py.

OPTIONS:
    --help, -h           Show this help message

EXAMPLES:
    loct cycles
    loct cycles src/";

const COMMANDS_HELP: &str = "loct commands - Show Tauri command bridges (FE ↔ BE)

USAGE:
    loct commands [OPTIONS] [PATHS...]

DESCRIPTION:
    Lists Tauri command handlers, missing/unused ones, and FE callsites.

OPTIONS:
    --name <FILTER>      Regex filter on command name
    --missing-only       Show only missing handlers
    --unused-only        Show only unused handlers
    --no-duplicates      Hide duplicate sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h           Show this help message

EXAMPLES:
    loct commands
    loct commands --missing-only";

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

const QUERY_HELP: &str = "loct query - Query snapshot data

USAGE:
    loct query <KIND> <TARGET>

KIND:
    who-imports <FILE>      Find all files that import FILE
    where-symbol <SYMBOL>   Find where a symbol is defined/exported
    component-of <FILE>     Show which component contains FILE

OPTIONS:
    --help, -h          Show this help message

EXAMPLES:
    loct query who-imports src/foo.ts
    loct query where-symbol MyType
    loct query component-of src/foo.ts";

const DIFF_HELP: &str = "loct diff - Compare snapshots and show semantic delta

USAGE:
    loct diff <FROM> [TO]

DESCRIPTION:
    Compares two snapshots (paths or refs) and reports semantic differences.

OPTIONS:
    --help, -h          Show this help message

EXAMPLES:
    loct diff HEAD~1
    loct diff .loctree/snapA.json .loctree/snapB.json";

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

const TWINS_HELP: &str =
    "loct twins - Detect semantic duplicates (dead parrots, exact twins, barrel chaos)

USAGE:
    loct twins [OPTIONS]

DESCRIPTION:
    Shows dead parrots (0 imports), twins, and barrel/export issues.

OPTIONS:
    --help, -h          Show this help message";

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
