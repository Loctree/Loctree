//! Parsers for scan-related commands: auto, scan, tree.
//!
//! These commands handle the initial codebase scanning and visualization.

use std::path::PathBuf;

use super::super::command::{AutoOptions, Command, ScanOptions, TreeOptions};

/// Parse `loct auto [options]` command - the default full analysis command.
pub(super) fn parse_auto_command(args: &[String]) -> Result<Command, String> {
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

/// Parse `loct scan [options]` command - build/update snapshot.
pub(super) fn parse_scan_command(args: &[String]) -> Result<Command, String> {
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
    --watch           Watch for changes and re-scan automatically
    --help, -h        Show this help message

ARGUMENTS:
    [PATHS...]        Root directories to scan (default: current directory)

EXAMPLES:
    loct scan                    # Scan current directory
    loct scan --full-scan        # Force complete rescan
    loct scan src/ lib/          # Scan specific directories
    loct scan --scan-all         # Include all files (even hidden)
    loct scan --watch            # Watch mode with live refresh"
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
            "--watch" => {
                opts.watch = true;
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

/// Parse `loct tree [options]` command - display LOC tree.
pub(super) fn parse_tree_command(args: &[String]) -> Result<Command, String> {
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_auto_default() {
        let result = parse_auto_command(&[]).unwrap();
        if let Command::Auto(opts) = result {
            assert!(opts.roots.contains(&PathBuf::from(".")));
            assert!(!opts.full_scan);
        } else {
            panic!("Expected Auto command");
        }
    }

    #[test]
    fn test_parse_auto_with_flags() {
        let args = vec!["--full-scan".into(), "--scan-all".into()];
        let result = parse_auto_command(&args).unwrap();
        if let Command::Auto(opts) = result {
            assert!(opts.full_scan);
            assert!(opts.scan_all);
        } else {
            panic!("Expected Auto command");
        }
    }

    #[test]
    fn test_parse_scan_command() {
        let args = vec!["--full-scan".into()];
        let result = parse_scan_command(&args).unwrap();
        if let Command::Scan(opts) = result {
            assert!(opts.full_scan);
        } else {
            panic!("Expected Scan command");
        }
    }

    #[test]
    fn test_parse_tree_command_with_depth() {
        let args = vec!["--depth".into(), "3".into()];
        let result = parse_tree_command(&args).unwrap();
        if let Command::Tree(opts) = result {
            assert_eq!(opts.depth, Some(3));
        } else {
            panic!("Expected Tree command");
        }
    }
}
