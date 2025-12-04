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
            Command::Events(_) => "events",
            Command::Info(_) => "info",
            Command::Lint(_) => "lint",
            Command::Report(_) => "report",
            Command::Help(_) => "help",
            Command::Version => "version",
            Command::Query(_) => "query",
            Command::Diff(_) => "diff",
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
        }
    }

    /// Generate the main help text listing all commands.
    pub fn format_help() -> String {
        let commands = [
            ("auto", "Full auto-scan with stack detection (default)"),
            ("scan", "Build/update snapshot for current HEAD"),
            ("tree", "Display LOC tree / structural overview"),
            ("slice <path>", "Extract holographic context for a file"),
            ("find", "Search symbols/files with regex filters"),
            ("dead", "Detect unused exports / dead code"),
            ("cycles", "Detect circular imports"),
            ("commands", "Show Tauri command bridges (FE ↔ BE)"),
            ("events", "Show event flow and issues"),
            ("info", "Show snapshot metadata and project info"),
            ("lint", "Structural lint/policy checks"),
            ("report", "Generate HTML/JSON reports"),
            (
                "query <kind> <target>",
                "Query snapshot (who-imports, where-symbol, component-of)",
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
}
