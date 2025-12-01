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
    "auto", "scan", "tree", "slice", "find", "dead", "unused", "cycles", "commands", "events",
    "info", "lint", "report", "help",
];

/// Check if an argument looks like a new-style subcommand.
pub fn is_subcommand(arg: &str) -> bool {
    SUBCOMMANDS.contains(&arg)
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
        // First positional argument - check if it's a subcommand
        return is_subcommand(arg);
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
    let command = match subcommand.as_deref() {
        None | Some("auto") => parse_auto_command(&remaining_args)?,
        Some("scan") => parse_scan_command(&remaining_args)?,
        Some("tree") => parse_tree_command(&remaining_args)?,
        Some("slice") => parse_slice_command(&remaining_args)?,
        Some("find") => parse_find_command(&remaining_args)?,
        Some("dead") | Some("unused") => parse_dead_command(&remaining_args)?,
        Some("cycles") => parse_cycles_command(&remaining_args)?,
        Some("commands") => parse_commands_command(&remaining_args)?,
        Some("events") => parse_events_command(&remaining_args)?,
        Some("info") => parse_info_command(&remaining_args)?,
        Some("lint") => parse_lint_command(&remaining_args)?,
        Some("report") => parse_report_command(&remaining_args)?,
        Some("help") => parse_help_command(&remaining_args)?,
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
            "--loc" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--loc requires a value".to_string())?;
                opts.loc_threshold = Some(value.parse().map_err(|_| "--loc requires a number")?);
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
            "--missing" => {
                opts.missing_only = true;
                i += 1;
            }
            "--unused" => {
                opts.unused_only = true;
                i += 1;
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

fn parse_info_command(args: &[String]) -> Result<Command, String> {
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
}
