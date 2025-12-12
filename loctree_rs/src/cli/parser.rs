//! New command parser for the subcommand-based CLI interface.
//!
//! This module parses `loct <command> [options]` style invocations.
//! It detects whether the input uses new subcommands or legacy flags
//! and routes accordingly.

use std::path::PathBuf;

use super::command::*;
use crate::types::ColorMode;

/// Known subcommand names for the new CLI interface.
const SUBCOMMANDS: &[&str] = &[
    "auto", "agent", "scan", "tree", "slice", "find", "dead", "unused", "cycles", "commands",
    "events", "info", "lint", "report", "help", "query", "diff", "memex", "crowd", "twins",
    "routes", "dist", "coverage", "sniff",
];

/// Check if an argument looks like a new-style subcommand.
pub fn is_subcommand(arg: &str) -> bool {
    SUBCOMMANDS.contains(&arg)
}

/// Check if argument looks like a jq filter expression
fn is_jq_filter(arg: &str) -> bool {
    let trimmed = arg.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Starts with . [ or { = jq filter
    if trimmed.starts_with('.') || trimmed.starts_with('[') || trimmed.starts_with('{') {
        // But not path-like ./foo or .\foo
        if trimmed.starts_with("./") || trimmed.starts_with(".\\") {
            return false;
        }
        // If it's a dotfile that exists on disk, treat as path
        if trimmed.starts_with('.')
            && !trimmed.contains('[')
            && !trimmed.contains('|')
            && std::path::Path::new(trimmed).exists()
        {
            return false;
        }
        return true;
    }
    false
}

/// Check if the argument list appears to use new-style subcommands.
///
/// Returns true if the first non-flag argument is a known subcommand,
/// or if only global flags like --help/--version are present.
pub fn uses_new_syntax(args: &[String]) -> bool {
    for arg in args {
        // Skip global flags that can appear before subcommand
        if arg == "--json"
            || arg == "--quiet"
            || arg == "--verbose"
            || arg.starts_with("--color")
            || arg == "--library-mode"
            || arg == "-v"
            || arg == "-q"
        {
            continue;
        }
        // These are always valid in new syntax (not legacy-specific)
        if arg == "--help"
            || arg == "-h"
            || arg == "--help-legacy"
            || arg == "--help-full"
            || arg == "--version"
            || arg == "-V"
        {
            return true;
        }
        // If we hit a flag, it's likely legacy syntax
        if arg.starts_with('-') {
            return false;
        }
        // First positional argument - check if it's a subcommand or jq filter
        return is_subcommand(arg) || is_jq_filter(arg);
    }
    // No arguments = default to auto (new syntax)
    true
}

/// Parse command-line arguments into a ParsedCommand.
///
/// This is the main entry point for the new CLI parser. It:
/// 1. Extracts global options (--json, --quiet, etc.)
/// 2. Identifies the subcommand
/// 3. Parses command-specific options
///
/// Returns `None` if the arguments should be handled by the legacy parser.
pub fn parse_command(args: &[String]) -> Result<Option<ParsedCommand>, String> {
    // Quick check: if this looks like legacy syntax, return None
    if !uses_new_syntax(args) {
        return Ok(None);
    }

    let mut global = GlobalOptions::default();
    let mut remaining_args: Vec<String> = Vec::new();
    let mut subcommand: Option<String> = None;

    // Check for jq-style query before extracting global options
    // This allows: loct '.metadata' to work without conflicts
    if !args.is_empty() && is_jq_filter(&args[0]) {
        return parse_jq_query_command(args, &global).map(Some);
    }

    // First pass: extract global options and find subcommand
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "--json" => {
                global.json = true;
                i += 1;
            }
            "--quiet" | "-q" => {
                global.quiet = true;
                i += 1;
            }
            "--verbose" | "-v" => {
                global.verbose = true;
                i += 1;
            }
            "--color" => {
                if let Some(value) = args.get(i + 1) {
                    global.color = parse_color_mode(value)?;
                    i += 2;
                } else {
                    global.color = ColorMode::Always;
                    i += 1;
                }
            }
            _ if arg.starts_with("--color=") => {
                let value = arg.trim_start_matches("--color=");
                global.color = parse_color_mode(value)?;
                i += 1;
            }
            "--library-mode" => {
                global.library_mode = true;
                i += 1;
            }
            "--python-library" => {
                global.python_library = true;
                i += 1;
            }
            "--help" | "-h" => {
                // Help is special - if no subcommand yet, show main help
                if subcommand.is_none() {
                    return Ok(Some(ParsedCommand::new(
                        Command::Help(HelpOptions::default()),
                        global,
                    )));
                }
                remaining_args.push(arg.clone());
                i += 1;
            }
            "--help-legacy" => {
                return Ok(Some(ParsedCommand::new(
                    Command::Help(HelpOptions {
                        legacy: true,
                        ..Default::default()
                    }),
                    global,
                )));
            }
            "--help-full" => {
                return Ok(Some(ParsedCommand::new(
                    Command::Help(HelpOptions {
                        full: true,
                        ..Default::default()
                    }),
                    global,
                )));
            }
            "--version" | "-V" => {
                return Ok(Some(ParsedCommand::new(Command::Version, global)));
            }
            _ if arg.starts_with('-') => {
                // Unknown flag - pass to command-specific parser
                remaining_args.push(arg.clone());
                i += 1;
            }
            _ => {
                // Positional argument
                if subcommand.is_none() && is_subcommand(arg) {
                    subcommand = Some(arg.clone());
                } else {
                    remaining_args.push(arg.clone());
                }
                i += 1;
            }
        }
    }

    // Parse the specific command
    if let Some(sub) = subcommand.as_deref()
        && remaining_args.iter().any(|a| a == "--help" || a == "-h")
    {
        return Ok(Some(ParsedCommand::new(
            Command::Help(HelpOptions {
                command: Some(sub.to_string()),
                ..Default::default()
            }),
            global,
        )));
    }

    let command = match subcommand.as_deref() {
        None | Some("auto") => parse_auto_command(&remaining_args)?,
        Some("agent") => {
            let cmd = parse_auto_command(&remaining_args)?;
            match cmd {
                Command::Auto(mut opts) => {
                    opts.for_agent_feed = true;
                    opts.agent_json = true;
                    Command::Auto(opts)
                }
                other => other,
            }
        }
        Some("scan") => parse_scan_command(&remaining_args)?,
        Some("tree") => parse_tree_command(&remaining_args)?,
        Some("slice") => parse_slice_command(&remaining_args)?,
        Some("find") => parse_find_command(&remaining_args)?,
        Some("dead") | Some("unused") => parse_dead_command(&remaining_args)?,
        Some("cycles") => parse_cycles_command(&remaining_args)?,
        Some("commands") => parse_commands_command(&remaining_args)?,
        Some("events") => parse_events_command(&remaining_args)?,
        Some("routes") => parse_routes_command(&remaining_args)?,
        Some("info") => parse_info_command(&remaining_args)?,
        Some("lint") => parse_lint_command(&remaining_args)?,
        Some("report") => parse_report_command(&remaining_args)?,
        Some("help") => parse_help_command(&remaining_args)?,
        Some("query") => parse_query_command(&remaining_args)?,
        Some("diff") => parse_diff_command(&remaining_args)?,
        Some("memex") => parse_memex_command(&remaining_args)?,
        Some("crowd") => parse_crowd_command(&remaining_args)?,
        Some("twins") => parse_twins_command(&remaining_args)?,
        Some("sniff") => parse_sniff_command(&remaining_args)?,
        Some("dist") => parse_dist_command(&remaining_args)?,
        Some("coverage") => parse_coverage_command(&remaining_args)?,
        Some(unknown) => {
            return Err(format!(
                "Unknown command '{}'. Run 'loct --help' for available commands.",
                unknown
            ));
        }
    };

    Ok(Some(ParsedCommand::new(command, global)))
}

// ============================================================================
// Helper parsers
// ============================================================================

fn parse_color_mode(value: &str) -> Result<ColorMode, String> {
    match value.to_lowercase().as_str() {
        "auto" => Ok(ColorMode::Auto),
        "always" | "yes" | "true" => Ok(ColorMode::Always),
        "never" | "no" | "false" => Ok(ColorMode::Never),
        _ => Err(format!(
            "Invalid color mode '{}'. Use: auto, always, or never.",
            value
        )),
    }
}

fn parse_auto_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err(
            "loct auto - Full auto-scan with stack detection (default command)

USAGE:
    loct auto [OPTIONS] [PATHS...]
    loct [OPTIONS] [PATHS...]    # 'auto' is the default command

DESCRIPTION:
    Performs a comprehensive analysis of your codebase:
    - Detects project type and language stack automatically
    - Builds dependency graph and import relationships
    - Analyzes code structure and exports
    - Identifies potential issues (dead code, cycles, etc.)

    This is the default command when no subcommand is specified.

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
    loct --agent-json            # Single agent bundle JSON"
                .to_string(),
        );
    }

    let mut opts = AutoOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--full-scan" => {
                opts.full_scan = true;
                i += 1;
            }
            "--scan-all" => {
                opts.scan_all = true;
                i += 1;
            }
            "--for-agent-feed" => {
                opts.for_agent_feed = true;
                i += 1;
            }
            "--agent-json" => {
                opts.for_agent_feed = true;
                opts.agent_json = true;
                i += 1;
            }
            "--no-duplicates" => {
                opts.suppress_duplicates = true;
                i += 1;
            }
            "--no-dynamic-imports" => {
                opts.suppress_dynamic = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'auto' command.", arg));
            }
        }
    }

    // Default to current directory if no roots specified
    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Auto(opts))
}

fn parse_scan_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct scan - Build/update snapshot for current HEAD

USAGE:
    loct scan [OPTIONS] [PATHS...]

DESCRIPTION:
    Scans the codebase and updates the internal snapshot database.
    This command builds the dependency graph, analyzes imports/exports,
    and prepares data for other commands like 'dead', 'cycles', 'tree'.

    Unlike 'auto', this command only builds the snapshot without
    running additional analysis passes.

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
    loct scan --scan-all         # Include all files (even hidden)"
            .to_string());
    }

    let mut opts = ScanOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--full-scan" => {
                opts.full_scan = true;
                i += 1;
            }
            "--scan-all" => {
                opts.scan_all = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'scan' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Scan(opts))
}

fn parse_tree_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct tree - Display LOC tree / structural overview

USAGE:
    loct tree [OPTIONS] [PATHS...]

DESCRIPTION:
    Displays a hierarchical tree view of your codebase structure,
    annotated with lines of code (LOC) metrics for each directory
    and file. Helps understand code distribution and organization.

    Similar to 'tree' command but with LOC counting and better
    handling of gitignored files.

OPTIONS:
    --depth <N>, -L <N>    Maximum depth to display (default: unlimited)
    --summary [N]          Show summary of top N largest items (default: 5)
    --top [N]              Show only top N largest items (default: 50)
    --loc <N>              Only show items with LOC >= N
    --min-loc <N>          Alias for --loc
    --show-hidden, -H      Include hidden files/directories
    --find-artifacts       Highlight build artifacts and generated files
    --show-ignored         Show gitignored files (normally hidden)
    --help, -h             Show this help message

ARGUMENTS:
    [PATHS...]             Root directories to analyze (default: current directory)

EXAMPLES:
    loct tree                       # Full tree of current directory
    loct tree --depth 3             # Limit to 3 levels deep
    loct tree --summary             # Show top 5 largest items
    loct tree --summary 10          # Show top 10 largest items
    loct tree --loc 100             # Only show files/dirs with 100+ LOC
    loct tree src/ --show-hidden    # Include hidden files in src/"
            .to_string());
    }

    let mut opts = TreeOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--depth" | "-L" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--depth requires a value".to_string())?;
                opts.depth = Some(value.parse().map_err(|_| "--depth requires a number")?);
                i += 2;
            }
            "--summary" => {
                if let Some(next) = args.get(i + 1)
                    && !next.starts_with('-')
                {
                    opts.summary = Some(
                        next.parse()
                            .map_err(|_| "--summary value must be a number")?,
                    );
                    i += 2;
                    continue;
                }
                opts.summary = Some(5); // Default summary limit
                i += 1;
            }
            "--top" => {
                if let Some(next) = args.get(i + 1)
                    && !next.starts_with('-')
                {
                    opts.summary = Some(next.parse().map_err(|_| "--top value must be a number")?);
                    i += 2;
                } else {
                    opts.summary = Some(50);
                    i += 1;
                }
                opts.summary_only = true;
            }
            "--loc" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--loc requires a value".to_string())?;
                opts.loc_threshold = Some(value.parse().map_err(|_| "--loc requires a number")?);
                i += 2;
            }
            "--min-loc" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--min-loc requires a value".to_string())?;
                opts.loc_threshold =
                    Some(value.parse().map_err(|_| "--min-loc requires a number")?);
                i += 2;
            }
            "--show-hidden" | "-H" => {
                opts.show_hidden = true;
                i += 1;
            }
            "--find-artifacts" => {
                opts.find_artifacts = true;
                i += 1;
            }
            "--show-ignored" => {
                opts.show_ignored = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'tree' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Tree(opts))
}

fn parse_slice_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct slice - Extract holographic context for a file

USAGE:
    loct slice [OPTIONS] <TARGET_PATH>

DESCRIPTION:
    Extracts a 'holographic slice' - the minimal context needed to
    understand a specific file. Shows its dependencies (imports) and
    optionally its consumers (what imports it).

    This is useful for:
    - Understanding a file's context before editing
    - Feeding relevant code to AI assistants
    - Analyzing impact of changes to a file

OPTIONS:
    --consumers, -c      Include files that import the target (reverse deps)
    --depth <N>          Maximum dependency depth to traverse (default: unlimited)
    --root <PATH>        Project root for resolving relative imports
    --help, -h           Show this help message

ARGUMENTS:
    <TARGET_PATH>        Path to the file to analyze (required)

EXAMPLES:
    loct slice src/main.rs                  # Show dependencies of main.rs
    loct slice src/utils.ts --consumers     # Show deps and consumers
    loct slice lib/api.ts --depth 2         # Limit to 2 levels deep
    loct slice src/app.tsx --root ./        # Specify project root"
            .to_string());
    }

    let mut opts = SliceOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--consumers" | "-c" => {
                opts.consumers = true;
                i += 1;
            }
            "--depth" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--depth requires a value".to_string())?;
                opts.depth = Some(value.parse().map_err(|_| "--depth requires a number")?);
                i += 2;
            }
            "--root" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--root requires a path".to_string())?;
                opts.root = Some(PathBuf::from(value));
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                if opts.target.is_empty() {
                    opts.target = arg.clone();
                } else {
                    return Err(format!(
                        "Unexpected argument '{}'. slice takes one target path.",
                        arg
                    ));
                }
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'slice' command.", arg));
            }
        }
    }

    if opts.target.is_empty() {
        return Err(
            "'slice' command requires a target file path. Usage: loct slice <path>".to_string(),
        );
    }

    Ok(Command::Slice(opts))
}

fn parse_find_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct find - Search symbols/files with regex filters

USAGE:
    loct find [QUERY] [OPTIONS]

DESCRIPTION:
    Powerful search across the codebase with multiple filter modes.
    Supports regex patterns for symbols and file names, plus special
    modes for impact analysis and similarity detection.

    Search modes:
    - Symbol search: Find functions, classes, types by name pattern
    - File search: Find files by path pattern
    - Impact analysis: Find all symbols affected by a file change
    - Similarity: Find symbols with similar names

OPTIONS:
    --symbol <PATTERN>, -s <PATTERN>    Search for symbols matching regex
    --pattern <PATTERN>                 Alias for --symbol (regex)
    --file <PATTERN>, -f <PATTERN>      Search for files matching regex
    --impact <FILE>                     Show symbols affected by file changes
    --similar <SYMBOL>                  Find symbols with similar names
    --dead                              Only show dead/unused symbols
    --exported                          Only show exported symbols
    --lang <LANG>                       Filter by language (ts, rs, js, py, etc.)
    --limit <N>                         Maximum results to show (default: unlimited)
    --help, -h                          Show this help message

ARGUMENTS:
    [QUERY]                             Search pattern (alternative to --symbol)

EXAMPLES:
    loct find Patient                   # Find symbols containing \"Patient\"
    loct find --symbol \".*Config$\"      # Regex: symbols ending with Config
    loct find --file \"utils\"            # Files containing \"utils\" in path
    loct find --symbol Patient --lang ts # TypeScript Patient symbols
    loct find --dead --exported         # Dead exported symbols
    loct find --impact src/api.ts       # What's affected by api.ts changes
    loct find --similar handleClick     # Find similarly named handlers
    loct find --limit 50                # Limit to 50 results"
            .to_string());
    }

    let mut opts = FindOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--symbol" | "-s" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--symbol requires a pattern".to_string())?;
                opts.symbol = Some(value.clone());
                i += 2;
            }
            "--pattern" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    "--pattern requires a pattern (alias for --symbol)".to_string()
                })?;
                opts.symbol = Some(value.clone());
                i += 2;
            }
            "--file" | "-f" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--file requires a pattern".to_string())?;
                opts.file = Some(value.clone());
                i += 2;
            }
            "--impact" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--impact requires a file path".to_string())?;
                opts.impact = Some(value.clone());
                i += 2;
            }
            "--similar" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--similar requires a symbol name".to_string())?;
                opts.similar = Some(value.clone());
                i += 2;
            }
            "--dead" => {
                opts.dead_only = true;
                i += 1;
            }
            "--exported" => {
                opts.exported_only = true;
                i += 1;
            }
            "--lang" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--lang requires a language".to_string())?;
                opts.lang = Some(value.clone());
                i += 2;
            }
            "--limit" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                opts.limit = Some(value.parse().map_err(|_| "--limit requires a number")?);
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                // Positional argument is the query
                opts.query = Some(arg.clone());
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'find' command.", arg));
            }
        }
    }

    Ok(Command::Find(opts))
}

fn parse_dead_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct dead - Detect unused exports / dead code

USAGE:
    loct dead [OPTIONS] [PATHS...]

DESCRIPTION:
    Finds exported symbols that are never imported anywhere in the codebase.
    Uses import graph analysis with alias-awareness to minimize false positives.

    For Rust projects (v0.5.17+):
    - Resolves crate-internal imports: use crate::foo::Bar, use super::Bar
    - Detects same-file usage in generics, type annotations, struct literals
    - Handles nested brace imports: use crate::{foo::{A, B}, bar::C}
    - Tauri #[command] handler detection

    For TypeScript/JavaScript:
    - Path alias resolution via tsconfig.json
    - Barrel file (index.ts) awareness
    - Dynamic import() tracking

OPTIONS:
    --confidence <LEVEL>   Filter by confidence: high, medium, low (default: all)
                           high = not imported in production code
                           medium = only used in tests
                           low = complex re-export, may be false positive
    --top <N>              Limit to top N results (default: 20)
    --full, --all          Show all results (ignore top limit)
    --path <PATTERN>       Filter to files matching pattern
    --with-tests           Include test files in analysis
    --exclude-tests        Exclude test files (default)
    --with-helpers         Include helper/utility files
    --help, -h             Show this help message

EXAMPLES:
    loct dead                          # All dead exports
    loct dead --confidence high        # Only high-confidence (no test files)
    loct dead --path src/components/   # Dead exports in components
    loct dead --top 50                 # Top 50 dead exports
    loct dead --json                   # JSON output for AI agents

RUST CRATE-INTERNAL IMPORTS:
    Loctree v0.5.17+ correctly handles Rust internal imports like:
      use crate::ui::constants::MENU_GAP;
      use super::types::Config;
      use self::utils::helper;

    These are resolved to actual file paths for accurate dead code detection."
            .to_string());
    }

    let mut opts = DeadOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--confidence" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    "--confidence requires a value (high, medium, low)".to_string()
                })?;
                opts.confidence = Some(value.clone());
                i += 2;
            }
            "--top" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--top requires a number".to_string())?;
                opts.top = Some(value.parse().map_err(|_| "--top requires a number")?);
                i += 2;
            }
            "--full" | "--all" => {
                opts.full = true;
                i += 1;
            }
            "--path" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--path requires a pattern".to_string())?;
                opts.path_filter = Some(value.clone());
                i += 2;
            }
            "--with-tests" => {
                opts.with_tests = true;
                i += 1;
            }
            "--exclude-tests" => {
                opts.with_tests = false;
                i += 1;
            }
            "--with-helpers" => {
                opts.with_helpers = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'dead' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Dead(opts))
}

fn parse_cycles_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct cycles - Detect circular imports

USAGE:
    loct cycles [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects circular import dependencies in your codebase.
    Circular imports can cause:
    - Runtime initialization errors
    - Undefined behavior in module loading
    - Confusing dependency graphs
    - Build/bundling issues

    This command analyzes the import graph and reports all cycles,
    grouped by severity and size.

OPTIONS:
    --path <PATTERN>     Filter to files matching path pattern
    --help, -h           Show this help message

ARGUMENTS:
    [PATHS...]           Root directories to analyze (default: current directory)

EXAMPLES:
    loct cycles                       # Detect all cycles in current dir
    loct cycles src/                  # Only analyze src/ directory
    loct cycles --path components/    # Cycles involving components/
    loct cycles --json                # JSON output for CI/CD"
            .to_string());
    }

    let mut opts = CyclesOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--path" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--path requires a pattern".to_string())?;
                opts.path_filter = Some(value.clone());
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'cycles' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Cycles(opts))
}

fn parse_commands_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct commands - Show Tauri command bridges (FE ↔ BE)

USAGE:
    loct commands [OPTIONS] [PATHS...]

DESCRIPTION:
    Analyzes Tauri command invocations to detect:
    - Frontend invoke() calls matched with backend #[tauri::command] handlers
    - Missing handlers (FE calls BE command that doesn't exist)
    - Unused handlers (BE command exists but FE never calls it)

    This helps maintain contract integrity between frontend and backend in Tauri apps.

OPTIONS:
    --name <PATTERN>   Filter to commands matching pattern
    --missing, --missing-only
                       Show only missing handlers (FE calls → no BE)
    --unused, --unused-only
                       Show only unused handlers (BE exists → no FE calls)
    --limit <N>        Maximum results to show (default: unlimited)
    --no-duplicates    Hide duplicate export sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h         Show this help message

EXAMPLES:
    loct commands                     # Show all command bridges
    loct commands --missing           # Only missing handlers
    loct commands --name patient_*    # Commands matching pattern
    loct commands --unused            # Unused backend commands
    loct commands --limit 10 --json   # First 10 as JSON"
            .to_string());
    }

    let mut opts = CommandsOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--name" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--name requires a pattern".to_string())?;
                opts.name_filter = Some(value.clone());
                i += 2;
            }
            "--missing" | "--missing-only" => {
                opts.missing_only = true;
                i += 1;
            }
            "--unused" | "--unused-only" => {
                opts.unused_only = true;
                i += 1;
            }
            "--no-duplicates" => {
                opts.suppress_duplicates = true;
                i += 1;
            }
            "--no-dynamic-imports" => {
                opts.suppress_dynamic = true;
                i += 1;
            }
            "--limit" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                opts.limit = Some(
                    value
                        .parse()
                        .map_err(|_| format!("Invalid limit value: {}", value))?,
                );
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'commands' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Commands(opts))
}

fn parse_events_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct events - Show event flow and issues

USAGE:
    loct events [OPTIONS] [PATHS...]

DESCRIPTION:
    Analyzes Tauri event system to detect:
    - Ghost events (emit() calls with no listen() handlers)
    - Orphan listeners (listen() calls with no emit() sources)
    - Race conditions (multiple emitters for same event)

    Helps maintain event contract integrity in Tauri applications.

OPTIONS:
    --ghost      Show only ghost events (emitted but never listened)
    --orphan     Show only orphan listeners (listening but never emitted)
    --races      Show only potential race conditions (multiple emitters)
    --no-duplicates      Hide duplicate export sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h   Show this help message

EXAMPLES:
    loct events                  # Show all event flow analysis
    loct events --ghost          # Only ghost events
    loct events --orphan         # Only orphan listeners
    loct events --races          # Only race conditions"
            .to_string());
    }

    let mut opts = EventsOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--ghost" => {
                opts.ghost = true;
                i += 1;
            }
            "--orphan" => {
                opts.orphan = true;
                i += 1;
            }
            "--races" => {
                opts.races = true;
                i += 1;
            }
            "--no-duplicates" => {
                opts.suppress_duplicates = true;
                i += 1;
            }
            "--no-dynamic-imports" => {
                opts.suppress_dynamic = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'events' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Events(opts))
}

fn parse_routes_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct routes - List backend/web routes (FastAPI/Flask)

USAGE:
    loct routes [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects Python web routes based on common decorators:
    - FastAPI: @app.get/post/put/delete/patch, @router.*, @api_router.*
    - Flask:   @app.route, @blueprint.route, .route(...)

    Useful for contract checks and quick navigation of backend endpoints.

OPTIONS:
    --framework <NAME>   Filter by framework label (fastapi, flask)
    --path <PATTERN>     Filter by route path substring
    --help, -h           Show this help message

EXAMPLES:
    loct routes                    # Show all detected routes
    loct routes --framework fastapi
    loct routes --path /patients
    loct routes api/               # Limit analysis to api/ path"
            .to_string());
    }

    let mut opts = RoutesOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--framework" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--framework requires a value".to_string())?;
                opts.framework = Some(value.clone());
                i += 2;
            }
            "--path" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--path requires a value".to_string())?;
                opts.path_filter = Some(value.clone());
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'routes' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Routes(opts))
}

fn parse_info_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct info - Show snapshot metadata and project info

USAGE:
    loct info [PATH]

DESCRIPTION:
    Displays high-level metadata about the analyzed codebase:
    - Total file count
    - Lines of code (LOC) statistics
    - Language breakdown
    - Scan time and performance
    - Snapshot ID and timestamp

    Useful for quick project overview and verification of scan results.

ARGUMENTS:
    [PATH]     Root directory to analyze (default: current directory)

OPTIONS:
    --help, -h   Show this help message

EXAMPLES:
    loct info              # Show info for current directory
    loct info src/         # Show info for src directory
    loct info --json       # JSON output for automation"
            .to_string());
    }

    let mut opts = InfoOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            _ if !arg.starts_with('-') => {
                opts.root = Some(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'info' command.", arg));
            }
        }
    }

    Ok(Command::Info(opts))
}

fn parse_lint_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct lint - Structural lint and policy checks

USAGE:
    loct lint [OPTIONS] [PATHS...]

DESCRIPTION:
    Performs structural and architectural policy checks:
    - Entrypoint validation (main.ts, index.html, etc.)
    - Import policy violations
    - Circular dependency detection
    - Tauri-specific contract validation

    CI-friendly with exit codes:
    - Exit 0: No issues found
    - Exit 1: Policy violations detected (with --fail)

OPTIONS:
    --entrypoints    Validate entrypoint files exist and are properly configured
    --fail           Exit with code 1 if any violations found (CI mode)
    --sarif          Output in SARIF format (GitHub Code Scanning compatible)
    --tauri          Enable Tauri-specific contract checks (commands, events)
    --no-duplicates  Hide duplicate export sections in CLI output
    --no-dynamic-imports Hide dynamic import sections in CLI output
    --help, -h       Show this help message

EXAMPLES:
    loct lint                     # Run all lint checks
    loct lint --fail              # Exit 1 on violations (CI)
    loct lint --tauri             # Include Tauri contract checks
    loct lint --sarif > lint.sarif   # SARIF output for GitHub"
            .to_string());
    }

    let mut opts = LintOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--entrypoints" => {
                opts.entrypoints = true;
                i += 1;
            }
            "--fail" => {
                opts.fail = true;
                i += 1;
            }
            "--sarif" => {
                opts.sarif = true;
                i += 1;
            }
            "--tauri" => {
                opts.tauri = true;
                i += 1;
            }
            "--no-duplicates" => {
                opts.suppress_duplicates = true;
                i += 1;
            }
            "--no-dynamic-imports" => {
                opts.suppress_dynamic = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'lint' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Lint(opts))
}

fn parse_report_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct report - Generate HTML/JSON reports

USAGE:
    loct report [OPTIONS] [PATHS...]

DESCRIPTION:
    Generates interactive analysis reports with:
    - Import graph visualization
    - Dead code highlighting
    - Crowd analysis results
    - Semantic duplicate detection
    - Tauri contract validation

    HTML reports are interactive with clickable navigation to source files.

OPTIONS:
    --format <FORMAT>    Output format: html (default) or json
    --output, -o <FILE>  Write report to file (default: auto-generate name)
    --serve              Start HTTP server to view report
    --port <PORT>        Server port (default: 8080, with --serve)
    --editor <EDITOR>    Editor for click-to-open (vscode, cursor, etc.)
    --help, -h           Show this help message

EXAMPLES:
    loct report                        # Generate HTML report
    loct report --serve                # Generate and serve on http://localhost:8080
    loct report --format json -o out.json   # JSON output
    loct report --editor cursor        # Open files in Cursor editor"
            .to_string());
    }

    let mut opts = ReportOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--format" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--format requires a value (html, json)".to_string())?;
                opts.format = Some(value.clone());
                i += 2;
            }
            "--output" | "-o" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--output requires a file path".to_string())?;
                opts.output = Some(PathBuf::from(value));
                i += 2;
            }
            "--serve" => {
                opts.serve = true;
                i += 1;
            }
            "--port" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--port requires a number".to_string())?;
                opts.port = Some(value.parse().map_err(|_| "--port requires a number")?);
                i += 2;
            }
            "--editor" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--editor requires a value".to_string())?;
                opts.editor = Some(value.clone());
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'report' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Report(opts))
}

fn parse_help_command(args: &[String]) -> Result<Command, String> {
    let mut opts = HelpOptions::default();

    for arg in args {
        match arg.as_str() {
            "--legacy" => opts.legacy = true,
            "--full" => opts.full = true,
            _ if !arg.starts_with('-') => opts.command = Some(arg.clone()),
            _ => {
                return Err(format!("Unknown option '{}' for 'help' command.", arg));
            }
        }
    }

    Ok(Command::Help(opts))
}

fn parse_query_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct query - Query snapshot data

USAGE:
    loct query <KIND> <TARGET>

DESCRIPTION:
    Query the import graph and symbol index for specific information:

    who-imports <FILE>      Find all files that import the specified file
    where-symbol <SYMBOL>   Find where a symbol is defined/exported
    component-of <FILE>     Show which components/modules contain this file

    Results include import paths, symbol locations, and dependency chains.

QUERY KINDS:
    who-imports       List all importers of a file
    where-symbol      Locate symbol definitions and exports
    component-of      Show containing components/modules

EXAMPLES:
    loct query who-imports src/utils.ts        # Who imports utils.ts?
    loct query where-symbol PatientRecord      # Where is PatientRecord defined?
    loct query component-of src/ui/Button.tsx  # What component owns Button?"
            .to_string());
    }

    if args.len() < 2 {
        return Err(
            "query command requires a kind and target.\nUsage: loct query <kind> <target>\nKinds: who-imports, where-symbol, component-of"
                .to_string(),
        );
    }

    let kind_str = &args[0];
    let target = args[1].clone();

    let kind = match kind_str.as_str() {
        "who-imports" => QueryKind::WhoImports,
        "where-symbol" => QueryKind::WhereSymbol,
        "component-of" => QueryKind::ComponentOf,
        _ => {
            return Err(format!(
                "Unknown query kind '{}'. Valid kinds: who-imports, where-symbol, component-of",
                kind_str
            ));
        }
    };

    Ok(Command::Query(QueryOptions { kind, target }))
}

fn parse_diff_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct diff - Compare snapshots and show semantic delta

USAGE:
    loct diff --since <SNAPSHOT> [--to <SNAPSHOT>] [OPTIONS]
    loct diff <SNAPSHOT1> [SNAPSHOT2]

DESCRIPTION:
    Compares two code snapshots and shows semantic changes:
    - New/removed files
    - Import graph changes
    - New dead code introduced
    - Symbol additions/removals
    - Architecture drift

    Snapshots can be Git refs (main, HEAD~1), tags, or snapshot IDs.

OPTIONS:
    --since <SNAPSHOT>   Base snapshot to compare from (required)
    --to <SNAPSHOT>      Target snapshot to compare to (default: current working tree)
    --jsonl              Output in JSONL format (one change per line)
    --problems-only      Show only regressions (new dead code, new cycles)
    --help, -h           Show this help message

EXAMPLES:
    loct diff --since main              # Compare main branch to working tree
    loct diff --since HEAD~1            # Compare to previous commit
    loct diff --since v1.0.0 --to v2.0.0   # Compare two tags
    loct diff main --problems-only      # Only show regressions since main"
            .to_string());
    }

    let mut opts = DiffOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--since" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--since requires a snapshot ID or path".to_string())?;
                opts.since = Some(value.clone());
                i += 2;
            }
            "--to" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--to requires a snapshot ID or path".to_string())?;
                opts.to = Some(value.clone());
                i += 2;
            }
            "--jsonl" => {
                opts.jsonl = true;
                i += 1;
            }
            "--problems-only" => {
                opts.problems_only = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                // First positional arg is --since value
                if opts.since.is_none() {
                    opts.since = Some(arg.clone());
                } else if opts.to.is_none() {
                    opts.to = Some(arg.clone());
                } else {
                    return Err(format!(
                        "Unexpected argument '{}'. diff takes at most two snapshot IDs.",
                        arg
                    ));
                }
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'diff' command.", arg));
            }
        }
    }

    if opts.since.is_none() {
        return Err(
            "'diff' command requires a snapshot ID to compare from.\nUsage: loct diff --since <snapshot-id> [--to <snapshot-id>]"
                .to_string(),
        );
    }

    Ok(Command::Diff(opts))
}

fn parse_memex_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct memex - Index analysis into AI memory (vector DB)

USAGE:
    loct memex [REPORT_PATH] [OPTIONS]

DESCRIPTION:
    Indexes code analysis results into a vector database for AI agent memory.
    Creates semantic embeddings of:
    - File summaries and purposes
    - Symbol definitions and usage patterns
    - Import relationships
    - Dead code and quality issues

    Enables AI agents to query codebase semantically: \"find patient validation logic\"
    instead of keyword search.

    NOTE: Requires building with --features memex

OPTIONS:
    --report-path, -r <PATH>   Path to analysis report (JSON format)
    --project-id <ID>          Project identifier for multi-project databases
    --namespace, -n <NAME>     Namespace for embeddings (default: default)
    --db-path <PATH>           Custom vector DB path (default: ~/.loctree/memex.db)
    --help, -h                 Show this help message

EXAMPLES:
    loct memex report.json                 # Index report into default DB
    loct memex -r report.json --project-id vista   # Index with project ID
    loct memex --namespace prod --db-path /data/memex.db   # Custom namespace and DB

BUILDING WITH MEMEX:
    cargo build --features memex
    cargo install loctree --features memex"
            .to_string());
    }

    let mut opts = MemexOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--report-path" | "-r" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--report-path requires a path".to_string())?;
                opts.report_path = PathBuf::from(value);
                i += 2;
            }
            "--project-id" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--project-id requires a value".to_string())?;
                opts.project_id = Some(value.clone());
                i += 2;
            }
            "--namespace" | "-n" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--namespace requires a value".to_string())?;
                opts.namespace = value.clone();
                i += 2;
            }
            "--db-path" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
                opts.db_path = Some(value.clone());
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                // Positional argument is report path
                opts.report_path = PathBuf::from(arg);
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'memex' command.", arg));
            }
        }
    }

    Ok(Command::Memex(opts))
}

fn parse_crowd_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err(
            "loct crowd - Detect functional crowds (similar files clustering)

USAGE:
    loct crowd [PATTERN] [OPTIONS]

ARGUMENTS:
    [PATTERN]    Pattern to detect crowd around (e.g., \"message\", \"patient\")
                 If not specified, auto-detects all crowds

OPTIONS:
    --auto, -a         Detect all crowds automatically
    --min-size <N>     Minimum crowd size to report (default: 2)
    --limit <N>        Maximum crowds to show (default: 10)
    --include-tests    Include test files (excluded by default)
    --help, -h         Show this help message

EXAMPLES:
    loct crowd                  # Auto-detect all crowds
    loct crowd message          # Find files clustering around \"message\"
    loct crowd --limit 5        # Show top 5 crowds"
                .to_string(),
        );
    }

    let mut opts = CrowdOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--auto" | "-a" => {
                opts.auto_detect = true;
                i += 1;
            }
            "--min-size" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--min-size requires a number".to_string())?;
                opts.min_size = Some(value.parse().map_err(|_| "--min-size requires a number")?);
                i += 2;
            }
            "--limit" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                opts.limit = Some(value.parse().map_err(|_| "--limit requires a number")?);
                i += 2;
            }
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            _ if !arg.starts_with('-') => {
                // Positional argument is the pattern (if not a root path)
                if opts.pattern.is_none() && !std::path::Path::new(arg).exists() {
                    opts.pattern = Some(arg.clone());
                } else {
                    opts.roots.push(PathBuf::from(arg));
                }
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'crowd' command.", arg));
            }
        }
    }

    // If no pattern and no auto flag, enable auto-detect
    if opts.pattern.is_none() && !opts.auto_detect {
        opts.auto_detect = true;
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Crowd(opts))
}

fn parse_twins_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err(
            "loct twins - Detect semantic duplicates (dead parrots, exact twins, barrel chaos)

USAGE:
    loct twins [OPTIONS]

DESCRIPTION:
    Identifies three types of semantic issues:

    Dead Parrots:   Exports with 0 imports (Monty Python reference)
                    - Code that looks alive but is actually unused

    Exact Twins:    Same symbol exported from multiple files
                    - Duplicate exports causing confusion

    Barrel Chaos:   Missing index.ts, deep re-export chains, inconsistent import paths
                    - Barrel file issues and re-export problems

OPTIONS:
    --path <DIR>       Root directory to analyze (default: current directory)
    --dead-only        Show only dead parrots (exports with 0 imports)
    --help, -h         Show this help message

EXAMPLES:
    loct twins                  # Full semantic analysis (all three types)
    loct twins --dead-only      # Only dead parrots (0 imports)
    loct twins --path src/      # Analyze specific directory"
                .to_string(),
        );
    }

    let mut opts = TwinsOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--path" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--path requires a directory".to_string())?;
                opts.path = Some(PathBuf::from(value));
                i += 2;
            }
            "--dead-only" => {
                opts.dead_only = true;
                i += 1;
            }
            _ => {
                // Treat as path if no flag prefix
                if !arg.starts_with('-') {
                    opts.path = Some(PathBuf::from(arg));
                    i += 1;
                } else {
                    return Err(format!("Unknown option '{}' for 'twins' command.", arg));
                }
            }
        }
    }

    Ok(Command::Twins(opts))
}

fn parse_sniff_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct sniff - Sniff for code smells (aggregate analysis)

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
    --help, -h             Show this help message

EXAMPLES:
    loct sniff                    # Full code smell analysis
    loct sniff --dead-only        # Only dead parrots
    loct sniff --twins-only       # Only duplicate names
    loct sniff --crowds-only      # Only similar file clusters
    loct sniff --include-tests    # Include test files
    loct sniff --json             # Machine-readable output"
            .to_string());
    }

    let mut opts = SniffOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--path" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--path requires a directory".to_string())?;
                opts.path = Some(PathBuf::from(value));
                i += 2;
            }
            "--dead-only" => {
                opts.dead_only = true;
                i += 1;
            }
            "--twins-only" => {
                opts.twins_only = true;
                i += 1;
            }
            "--crowds-only" => {
                opts.crowds_only = true;
                i += 1;
            }
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            "--min-crowd-size" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--min-crowd-size requires a number".to_string())?;
                opts.min_crowd_size = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("Invalid number for --min-crowd-size: {}", value))?,
                );
                i += 2;
            }
            _ => {
                // Treat as path if no flag prefix
                if !arg.starts_with('-') {
                    opts.path = Some(PathBuf::from(arg));
                    i += 1;
                } else {
                    return Err(format!("Unknown option '{}' for 'sniff' command.", arg));
                }
            }
        }
    }

    Ok(Command::Sniff(opts))
}

fn parse_dist_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct dist - Analyze bundle distribution using source maps

USAGE:
    loct dist --source-map <PATH> --src <DIR>

DESCRIPTION:
    Compares source code exports with bundled JavaScript to find truly dead exports.
    Uses source maps to detect code that was completely tree-shaken out by the bundler.

OPTIONS:
    --source-map <PATH>    Path to source map file (e.g., dist/main.js.map)
    --src <DIR>            Source directory to scan (e.g., src/)
    --help, -h             Show this help message

EXAMPLES:
    loct dist --source-map dist/main.js.map --src src/
    loct dist --source-map build/app.js.map --src app/src/"
            .to_string());
    }

    let mut opts = DistOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--source-map" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--source-map requires a path".to_string())?;
                opts.source_map = Some(PathBuf::from(value));
                i += 2;
            }
            "--src" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--src requires a directory path".to_string())?;
                opts.src = Some(PathBuf::from(value));
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                // Positional: first is source-map, second is src
                if opts.source_map.is_none() {
                    opts.source_map = Some(PathBuf::from(arg));
                } else if opts.src.is_none() {
                    opts.src = Some(PathBuf::from(arg));
                } else {
                    return Err(format!(
                        "Unexpected argument '{}'. dist takes --source-map and --src.",
                        arg
                    ));
                }
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'dist' command.", arg));
            }
        }
    }

    if opts.source_map.is_none() {
        return Err(
            "'dist' command requires --source-map <path>. Usage: loct dist --source-map dist/main.js.map --src src/"
                .to_string(),
        );
    }

    if opts.src.is_none() {
        return Err(
            "'dist' command requires --src <dir>. Usage: loct dist --source-map dist/main.js.map --src src/"
                .to_string(),
        );
    }

    Ok(Command::Dist(opts))
}

fn parse_coverage_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct coverage - Analyze test coverage gaps

USAGE:
    loct coverage [OPTIONS] [PATHS...]

DESCRIPTION:
    Cross-references production usage (FE invoke/emit calls) with test imports
    to find handlers and events without test coverage.

    Identifies:
    - Handlers called from production but not tested (CRITICAL)
    - Events emitted in production but not tested (HIGH)
    - Code used in production without test imports (MEDIUM)
    - Tested code not used in production (LOW - potential dead code)

OPTIONS:
    --handlers       Show only handler coverage gaps
    --events         Show only event coverage gaps
    --min-severity <LEVEL>
                     Filter by minimum severity (critical/high/medium/low)
    --json           Output as JSON
    --help, -h       Show this help message

EXAMPLES:
    loct coverage                          # All coverage gaps
    loct coverage --handlers               # Only handler gaps
    loct coverage --events                 # Only event gaps
    loct coverage --min-severity critical  # Only critical issues
    loct coverage --json                   # JSON output for CI"
            .to_string());
    }

    let mut opts = CoverageOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--handlers" => {
                opts.handlers_only = true;
                i += 1;
            }
            "--events" => {
                opts.events_only = true;
                i += 1;
            }
            "--min-severity" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    "--min-severity requires a value (critical/high/medium/low)".to_string()
                })?;
                opts.min_severity = Some(value.clone());
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'coverage' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Coverage(opts))
}

fn parse_jq_query_command(
    args: &[String],
    global: &GlobalOptions,
) -> Result<ParsedCommand, String> {
    if args.is_empty() {
        return Err("jq query requires a filter expression".to_string());
    }

    let mut opts = JqQueryOptions::default();

    // First arg should be the filter
    let mut i = if is_jq_filter(&args[0]) {
        opts.filter = args[0].clone();
        1
    } else {
        return Err(format!("Expected jq filter expression, got: '{}'", args[0]));
    };

    // Parse remaining jq-specific flags
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-r" | "--raw-output" => {
                opts.raw_output = true;
                i += 1;
            }
            "-c" | "--compact-output" => {
                opts.compact_output = true;
                i += 1;
            }
            "-e" | "--exit-status" => {
                opts.exit_status = true;
                i += 1;
            }
            "--arg" => {
                let name = args
                    .get(i + 1)
                    .ok_or_else(|| "--arg requires a name and value".to_string())?;
                let value = args
                    .get(i + 2)
                    .ok_or_else(|| "--arg requires a name and value".to_string())?;
                opts.string_args.push((name.clone(), value.clone()));
                i += 3;
            }
            "--argjson" => {
                let name = args
                    .get(i + 1)
                    .ok_or_else(|| "--argjson requires a name and JSON value".to_string())?;
                let json_value = args
                    .get(i + 2)
                    .ok_or_else(|| "--argjson requires a name and JSON value".to_string())?;
                opts.json_args.push((name.clone(), json_value.clone()));
                i += 3;
            }
            "--snapshot" => {
                let path = args
                    .get(i + 1)
                    .ok_or_else(|| "--snapshot requires a path".to_string())?;
                opts.snapshot_path = Some(PathBuf::from(path));
                i += 2;
            }
            "--help" | "-h" => {
                return Err("loct jq - Query snapshot with jq-style filters

USAGE:
    loct '<filter>' [OPTIONS]

DESCRIPTION:
    Execute jq-style filter expressions on the latest snapshot JSON.
    Automatically finds the most recent snapshot in .loctree/ directory.

    The filter syntax follows jq conventions:
    - .metadata          Extract metadata field
    - .files[]           Iterate over files array
    - .files[0]          Get first file
    - .[\"key\"]           Access key with special characters

OPTIONS:
    -r, --raw-output         Output raw strings, not JSON
    -c, --compact-output     Compact JSON output (no pretty-printing)
    -e, --exit-status        Set exit code based on output (0 if truthy)
    --arg <name> <value>     Pass string variable to filter
    --argjson <name> <json>  Pass JSON variable to filter
    --snapshot <path>        Use specific snapshot file instead of latest
    --help, -h               Show this help message

EXAMPLES:
    loct '.metadata'                    # Extract metadata
    loct '.files | length'              # Count files
    loct '.files[] | .path'             # List all file paths
    loct '.metadata.total_loc' -r       # Raw number output
    loct '.files[] | select(.lang == \"ts\")' -c  # Find TypeScript files
    loct --snapshot .loctree/snap-abc123.json '.metadata'  # Query specific snapshot

GLOBAL OPTIONS:
    --json           Output as JSON (default for jq mode)
    --quiet          Suppress warnings
    --verbose        Show debug info

NOTE: This command requires jaq library (built with --features jq)"
                    .to_string());
            }
            _ => {
                return Err(format!("Unknown option '{}' for jq query mode", arg));
            }
        }
    }

    Ok(ParsedCommand::new(Command::JqQuery(opts), global.clone()))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_subcommand() {
        assert!(is_subcommand("auto"));
        assert!(is_subcommand("scan"));
        assert!(is_subcommand("tree"));
        assert!(is_subcommand("slice"));
        assert!(is_subcommand("dead"));
        assert!(!is_subcommand("--tree"));
        assert!(!is_subcommand("-A"));
        assert!(!is_subcommand("unknown"));
    }

    #[test]
    fn test_uses_new_syntax() {
        // New syntax
        assert!(uses_new_syntax(&[]));
        assert!(uses_new_syntax(&["scan".into()]));
        assert!(uses_new_syntax(&["tree".into()]));
        assert!(uses_new_syntax(&["--json".into(), "scan".into()]));

        // Legacy syntax
        assert!(!uses_new_syntax(&["--tree".into()]));
        assert!(!uses_new_syntax(&["-A".into()]));
        assert!(!uses_new_syntax(&["-A".into(), "--dead".into()]));
    }

    #[test]
    fn test_parse_auto_default() {
        let result = parse_command(&[]).unwrap().unwrap();
        assert_eq!(result.command.name(), "auto");
    }

    #[test]
    fn test_parse_scan_command() {
        let args = vec!["scan".into(), "--full-scan".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "scan");
        if let Command::Scan(opts) = result.command {
            assert!(opts.full_scan);
        } else {
            panic!("Expected Scan command");
        }
    }

    #[test]
    fn test_parse_tree_command_with_depth() {
        let args = vec!["tree".into(), "--depth".into(), "3".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "tree");
        if let Command::Tree(opts) = result.command {
            assert_eq!(opts.depth, Some(3));
        } else {
            panic!("Expected Tree command");
        }
    }

    #[test]
    fn test_parse_slice_command() {
        let args = vec!["slice".into(), "src/main.rs".into(), "--consumers".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "slice");
        if let Command::Slice(opts) = result.command {
            assert_eq!(opts.target, "src/main.rs");
            assert!(opts.consumers);
        } else {
            panic!("Expected Slice command");
        }
    }

    #[test]
    fn test_parse_dead_command() {
        let args = vec!["dead".into(), "--confidence".into(), "high".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "dead");
        if let Command::Dead(opts) = result.command {
            assert_eq!(opts.confidence, Some("high".into()));
        } else {
            panic!("Expected Dead command");
        }
    }

    #[test]
    fn test_parse_global_json_flag() {
        let args = vec!["--json".into(), "scan".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert!(result.global.json);
        assert_eq!(result.command.name(), "scan");
    }

    #[test]
    fn test_parse_help_flag() {
        let args = vec!["--help".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "help");
    }

    #[test]
    fn test_parse_version_flag() {
        let args = vec!["--version".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "version");
    }

    #[test]
    fn test_legacy_syntax_returns_none() {
        let args = vec!["--tree".into()];
        let result = parse_command(&args).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_find_with_regex() {
        let args = vec![
            "find".into(),
            "--symbol".into(),
            ".*patient.*".into(),
            "--lang".into(),
            "ts".into(),
        ];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::Find(opts) = result.command {
            assert_eq!(opts.symbol, Some(".*patient.*".into()));
            assert_eq!(opts.lang, Some("ts".into()));
        } else {
            panic!("Expected Find command");
        }
    }

    #[test]
    fn test_parse_crowd_command() {
        let args = vec!["crowd".into(), "message".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "crowd");
        if let Command::Crowd(opts) = result.command {
            assert_eq!(opts.pattern, Some("message".into()));
        } else {
            panic!("Expected Crowd command");
        }
    }

    #[test]
    fn test_parse_crowd_auto_detect() {
        let args = vec!["crowd".into(), "--auto".into()];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::Crowd(opts) = result.command {
            assert!(opts.auto_detect);
            assert!(opts.pattern.is_none());
        } else {
            panic!("Expected Crowd command");
        }
    }

    #[test]
    fn test_is_jq_filter() {
        // Valid jq filters
        assert!(is_jq_filter(".metadata"));
        assert!(is_jq_filter(".files[]"));
        assert!(is_jq_filter(".files[0]"));
        assert!(is_jq_filter("[.files]"));
        assert!(is_jq_filter("{foo: .bar}"));
        assert!(is_jq_filter(".foo | .bar"));

        // Not jq filters
        assert!(!is_jq_filter("./foo"));
        assert!(!is_jq_filter(".\\foo"));
        assert!(!is_jq_filter("scan"));
        assert!(!is_jq_filter("--help"));
        assert!(!is_jq_filter(""));
    }

    #[test]
    fn test_parse_jq_query_basic() {
        let args = vec![".metadata".into()];
        let result = parse_command(&args).unwrap().unwrap();
        assert_eq!(result.command.name(), "jq");
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(opts.filter, ".metadata");
            assert!(!opts.raw_output);
            assert!(!opts.compact_output);
        } else {
            panic!("Expected JqQuery command");
        }
    }

    #[test]
    fn test_parse_jq_query_with_flags() {
        let args = vec![".files[]".into(), "-r".into(), "-c".into()];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(opts.filter, ".files[]");
            assert!(opts.raw_output);
            assert!(opts.compact_output);
        } else {
            panic!("Expected JqQuery command");
        }
    }

    #[test]
    fn test_parse_jq_query_with_arg() {
        let args = vec![
            ".metadata".into(),
            "--arg".into(),
            "name".into(),
            "value".into(),
        ];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(opts.string_args.len(), 1);
            assert_eq!(opts.string_args[0].0, "name");
            assert_eq!(opts.string_args[0].1, "value");
        } else {
            panic!("Expected JqQuery command");
        }
    }

    #[test]
    fn test_parse_jq_query_with_snapshot() {
        let args = vec![
            ".metadata".into(),
            "--snapshot".into(),
            ".loctree/snap.json".into(),
        ];
        let result = parse_command(&args).unwrap().unwrap();
        if let Command::JqQuery(opts) = result.command {
            assert_eq!(
                opts.snapshot_path,
                Some(PathBuf::from(".loctree/snap.json"))
            );
        } else {
            panic!("Expected JqQuery command");
        }
    }
}
