//! Parsers for context extraction commands: slice, trace, focus, coverage, hotspots.
//!
//! These commands extract specific views/slices of the codebase for analysis.

use std::path::PathBuf;

use super::super::command::{
    Command, CoverageOptions, FocusOptions, HotspotsOptions, SliceOptions, TraceOptions,
};

/// Parse `loct slice <target> [options]` command - extract file + dependencies.
pub(super) fn parse_slice_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct slice - Extract file + dependencies for AI context

USAGE:
    loct slice <TARGET_PATH> [OPTIONS]

OPTIONS:
    --consumers, -c      Include reverse dependencies (files that import this file)
    --depth <N>          Maximum dependency depth to traverse (default: unlimited)
    --root <PATH>        Project root for resolving relative imports
    --rescan             Force snapshot update before slicing
    --help, -h           Show this help message

EXAMPLES:
    loct slice src/main.rs
    loct slice src/utils.ts --consumers"
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
            "--rescan" => {
                opts.rescan = true;
                i += 1;
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

/// Parse `loct trace <handler> [roots]` command - trace Tauri/IPC handler.
pub(super) fn parse_trace_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct trace - Trace a Tauri/IPC handler end-to-end

USAGE:
    loct trace <handler> [ROOTS...]

ARGUMENTS:
    <handler>         Handler name to trace (required)
    [ROOTS...]        Root directories to scan (default: current directory)

EXAMPLES:
    loct trace toggle_assistant
    loct trace standard_command apps/desktop"
            .to_string());
    }

    if args.is_empty() {
        return Err(
            "trace requires a handler name. Usage: loct trace <handler> [ROOTS...]".to_string(),
        );
    }

    let mut opts = TraceOptions::default();
    let mut i = 0;

    // First positional is the handler name
    let handler = &args[i];
    if handler.starts_with('-') {
        return Err("trace requires a handler name as the first argument".to_string());
    }
    opts.handler = handler.clone();
    i += 1;

    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with('-') {
            return Err(format!("Unknown option '{}' for 'trace' command.", arg));
        } else {
            opts.roots.push(PathBuf::from(arg));
        }
        i += 1;
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Trace(opts))
}

/// Parse `loct focus <dir> [options]` command - focus on specific directory.
pub(super) fn parse_focus_command(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("focus requires a target directory. Usage: loct focus <dir>".to_string());
    }

    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("focus")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = FocusOptions::default();

    // First positional argument is the target directory
    if !args[0].starts_with('-') {
        opts.target = args[0].clone();
    } else {
        return Err("focus requires a target directory as first argument".to_string());
    }

    let mut i = 1;

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
                opts.depth =
                    Some(value.parse::<usize>().map_err(|_| {
                        format!("Invalid depth value '{}', expected a number", value)
                    })?);
                i += 2;
            }
            "--root" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--root requires a path".to_string())?;
                opts.root = Some(PathBuf::from(value));
                i += 2;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'focus' command.", arg));
            }
        }
    }

    Ok(Command::Focus(opts))
}

/// Parse `loct coverage [options]` command - analyze test coverage gaps.
pub(super) fn parse_coverage_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct coverage - Analyze test coverage gaps

USAGE:
    loct coverage [OPTIONS] [PATHS...]

OPTIONS:
    --handlers       Show only handler coverage gaps
    --events         Show only event coverage gaps
    --min-severity <LEVEL>
                     Filter by minimum severity (critical/high/medium/low)
    --json           Output as JSON
    --help, -h       Show this help message

EXAMPLES:
    loct coverage
    loct coverage --handlers"
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

/// Parse `loct hotspots [options]` command - find high-impact files.
pub(super) fn parse_hotspots_command(args: &[String]) -> Result<Command, String> {
    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("hotspots")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = HotspotsOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--min" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--min requires a value".to_string())?;
                opts.min_imports =
                    Some(value.parse::<usize>().map_err(|_| {
                        format!("Invalid min value '{}', expected a number", value)
                    })?);
                i += 2;
            }
            "--limit" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a value".to_string())?;
                opts.limit =
                    Some(value.parse::<usize>().map_err(|_| {
                        format!("Invalid limit value '{}', expected a number", value)
                    })?);
                i += 2;
            }
            "--leaves" => {
                opts.leaves_only = true;
                i += 1;
            }
            "--coupling" => {
                opts.coupling = true;
                i += 1;
            }
            "--root" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--root requires a path".to_string())?;
                opts.root = Some(PathBuf::from(value));
                i += 2;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'hotspots' command.", arg));
            }
        }
    }

    Ok(Command::Hotspots(opts))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slice_command() {
        let args = vec!["src/main.rs".into(), "--consumers".into()];
        let result = parse_slice_command(&args).unwrap();
        if let Command::Slice(opts) = result {
            assert_eq!(opts.target, "src/main.rs");
            assert!(opts.consumers);
        } else {
            panic!("Expected Slice command");
        }
    }

    #[test]
    fn test_parse_trace_command() {
        let args = vec!["toggle_assistant".into(), "app".into()];
        let result = parse_trace_command(&args).unwrap();
        if let Command::Trace(opts) = result {
            assert_eq!(opts.handler, "toggle_assistant");
            assert_eq!(opts.roots, vec![PathBuf::from("app")]);
        } else {
            panic!("Expected Trace command");
        }
    }

    #[test]
    fn test_parse_focus_command() {
        let args = vec!["src/ui".into(), "--consumers".into()];
        let result = parse_focus_command(&args).unwrap();
        if let Command::Focus(opts) = result {
            assert_eq!(opts.target, "src/ui");
            assert!(opts.consumers);
        } else {
            panic!("Expected Focus command");
        }
    }

    #[test]
    fn test_parse_coverage_command() {
        let args = vec!["--handlers".into()];
        let result = parse_coverage_command(&args).unwrap();
        if let Command::Coverage(opts) = result {
            assert!(opts.handlers_only);
        } else {
            panic!("Expected Coverage command");
        }
    }

    #[test]
    fn test_parse_hotspots_command() {
        let args = vec!["--min".into(), "5".into(), "--limit".into(), "10".into()];
        let result = parse_hotspots_command(&args).unwrap();
        if let Command::Hotspots(opts) = result {
            assert_eq!(opts.min_imports, Some(5));
            assert_eq!(opts.limit, Some(10));
        } else {
            panic!("Expected Hotspots command");
        }
    }
}
