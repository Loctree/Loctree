//! Per-command option structs for all CLI commands.
//!
//! Created by M&K (c)2025 The LibraxisAI Team
//! Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>

use std::path::PathBuf;

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

    /// Include ambient declarations (declare global/module/namespace) in analysis.
    /// By default these are excluded as they're consumed by TypeScript compiler, not imports.
    pub with_ambient: bool,

    /// Include dynamically generated symbols (exec/eval/compile templates) in analysis.
    /// By default these are excluded as they're generated at runtime, not actual dead code.
    pub with_dynamic: bool,
}

/// Options for the `cycles` command.
#[derive(Debug, Clone, Default)]
pub struct CyclesOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Filter by file path pattern (regex)
    pub path_filter: Option<String>,

    /// Only show cycles that would break compilation
    pub breaking_only: bool,

    /// Show detailed explanation for each cycle
    pub explain: bool,

    /// Use legacy output format (for backwards compatibility)
    pub legacy_format: bool,
}

/// Options for the `trace` command (Tauri/IPC handler tracing).
#[derive(Debug, Clone, Default)]
pub struct TraceOptions {
    /// Handler name to trace
    pub handler: String,

    /// Root directories to analyze
    pub roots: Vec<PathBuf>,
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

    /// Show only FE<->FE sync events (window sync pattern)
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

    /// Ignore framework conventions when detecting twins.
    /// By default, framework-specific patterns (e.g., Django mixins) are filtered.
    pub ignore_conventions: bool,
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
    pub snapshot_path: Option<PathBuf>,
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

/// Options for the `health` command.
/// Quick health check summary (cycles + dead + twins).
#[derive(Debug, Clone, Default)]
pub struct HealthOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Include test files in analysis (default: false)
    pub include_tests: bool,
}

/// Options for the `audit` command.
/// Full audit combining all structural analyses into one actionable markdown report.
#[derive(Debug, Clone)]
pub struct AuditOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Include test files in analysis (default: false)
    pub include_tests: bool,

    /// Output as actionable todo checklist (default: false)
    pub todos: bool,

    /// Maximum items per category (default: 20)
    pub limit: usize,
}

impl Default for AuditOptions {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            include_tests: false,
            todos: false,
            limit: 20,
        }
    }
}

/// Options for the `doctor` command.
/// Interactive diagnostics with categorized findings and suggested suppressions.
#[derive(Debug, Clone, Default)]
pub struct DoctorOptions {
    /// Root directories to analyze
    pub roots: Vec<PathBuf>,

    /// Include test files in analysis (default: false)
    pub include_tests: bool,

    /// Automatically apply suggested suppressions to .loctignore
    pub apply_suppressions: bool,
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
