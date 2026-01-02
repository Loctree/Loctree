//! Parsers for miscellaneous commands: crowd, tagmap, suppress, dist, layoutmap,
//! zombie, health, audit, doctor, help.
//!
//! These commands handle various specialized functionality.

use std::path::PathBuf;

use super::super::command::{
    AuditOptions, Command, CrowdOptions, DistOptions, DoctorOptions, HealthOptions, HelpOptions,
    LayoutmapOptions, SuppressOptions, TagmapOptions, ZombieOptions,
};

/// Parse `loct crowd [pattern] [options]` command - detect functional crowds.
pub(super) fn parse_crowd_command(args: &[String]) -> Result<Command, String> {
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

/// Parse `loct tagmap <keyword> [options]` command - map files by keyword.
pub(super) fn parse_tagmap_command(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("tagmap requires a keyword. Usage: loct tagmap <keyword>".to_string());
    }

    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("tagmap")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = TagmapOptions::default();

    // First positional argument is the keyword
    if !args[0].starts_with('-') {
        opts.keyword = args[0].clone();
    } else {
        return Err("tagmap requires a keyword as first argument".to_string());
    }

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            "--limit" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                opts.limit = Some(value.parse().map_err(|_| "--limit requires a number")?);
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'tagmap' command.", arg));
            }
        }
    }

    if opts.roots.is_empty() {
        opts.roots.push(PathBuf::from("."));
    }

    Ok(Command::Tagmap(opts))
}

/// Parse `loct suppress [options]` command - manage false positive suppressions.
pub(super) fn parse_suppress_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct suppress - Manage false positive suppressions

USAGE:
    loct suppress <type> <symbol> [OPTIONS]
    loct suppress --list
    loct suppress --clear

TYPES:
    twins         Exact twin (same symbol in multiple files)
    dead_parrot   Dead parrot (export with 0 imports)
    dead_export   Dead export (unused export)
    circular      Circular import

OPTIONS:
    --file <path>       Suppress only for this specific file
    --reason <text>     Reason for suppression (for documentation)
    --remove            Remove a suppression instead of adding
    --list              List all current suppressions
    --clear             Clear all suppressions

EXAMPLES:
    loct suppress twins Message --reason \"FE/BE mirror OK\"
    loct suppress dead_parrot unusedFunc --file src/utils.ts
    loct suppress --list
    loct suppress twins Message --remove"
            .to_string());
    }

    let mut opts = SuppressOptions::default();
    let mut i = 0;
    let mut positional_count = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--list" => {
                opts.list = true;
                i += 1;
            }
            "--clear" => {
                opts.clear = true;
                i += 1;
            }
            "--remove" => {
                opts.remove = true;
                i += 1;
            }
            "--file" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--file requires a path".to_string())?;
                opts.file = Some(value.clone());
                i += 2;
            }
            "--reason" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--reason requires a value".to_string())?;
                opts.reason = Some(value.clone());
                i += 2;
            }
            "--path" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--path requires a directory".to_string())?;
                opts.path = Some(PathBuf::from(value));
                i += 2;
            }
            _ => {
                if arg.starts_with('-') {
                    return Err(format!("Unknown option '{}' for 'suppress' command.", arg));
                }
                // Positional: first is type, second is symbol
                match positional_count {
                    0 => opts.suppression_type = Some(arg.clone()),
                    1 => opts.symbol = Some(arg.clone()),
                    _ => return Err(format!("Unexpected argument '{}'.", arg)),
                }
                positional_count += 1;
                i += 1;
            }
        }
    }

    Ok(Command::Suppress(opts))
}

/// Parse `loct dist [options]` command - analyze bundle distribution.
pub(super) fn parse_dist_command(args: &[String]) -> Result<Command, String> {
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

/// Parse `loct layoutmap [options]` command - analyze CSS layout.
pub(super) fn parse_layoutmap_command(args: &[String]) -> Result<Command, String> {
    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("layoutmap")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = LayoutmapOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--zindex" | "--z-index" | "--zindex-only" => {
                opts.zindex_only = true;
                i += 1;
            }
            "--sticky" | "--sticky-only" => {
                opts.sticky_only = true;
                i += 1;
            }
            "--grid" | "--grid-only" => {
                opts.grid_only = true;
                i += 1;
            }
            "--min-zindex" | "--min-z" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--min-zindex requires a value".to_string())?;
                opts.min_zindex = Some(value.parse::<i32>().map_err(|_| {
                    format!("Invalid z-index value '{}', expected a number", value)
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
            "--exclude" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--exclude requires a glob pattern".to_string())?;
                opts.exclude.push(value.clone());
                i += 2;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'layoutmap' command.", arg));
            }
        }
    }

    Ok(Command::Layoutmap(opts))
}

/// Parse `loct zombie [options]` command - find zombie code.
pub(super) fn parse_zombie_command(args: &[String]) -> Result<Command, String> {
    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("zombie")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = ZombieOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            _ => {
                // Treat as root path
                if arg.starts_with("--") {
                    return Err(format!("Unknown option '{}' for 'zombie' command.", arg));
                }
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
        }
    }

    Ok(Command::Zombie(opts))
}

/// Parse `loct health [options]` command - codebase health check.
pub(super) fn parse_health_command(args: &[String]) -> Result<Command, String> {
    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("health")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = HealthOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            _ => {
                // Treat as root path
                if arg.starts_with("--") {
                    return Err(format!("Unknown option '{}' for 'health' command.", arg));
                }
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
        }
    }

    Ok(Command::Health(opts))
}

/// Parse `loct audit [options]` command - security audit.
pub(super) fn parse_audit_command(args: &[String]) -> Result<Command, String> {
    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("audit")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = AuditOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            _ => {
                // Treat as root path
                if arg.starts_with("--") {
                    return Err(format!("Unknown option '{}' for 'audit' command.", arg));
                }
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
        }
    }

    Ok(Command::Audit(opts))
}

/// Parse `loct doctor [options]` command - diagnose and fix issues.
pub(super) fn parse_doctor_command(args: &[String]) -> Result<Command, String> {
    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("doctor")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = DoctorOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            "--apply-suppressions" => {
                opts.apply_suppressions = true;
                i += 1;
            }
            _ => {
                // Treat as root path
                if arg.starts_with("--") {
                    return Err(format!("Unknown option '{}' for 'doctor' command.", arg));
                }
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
        }
    }

    Ok(Command::Doctor(opts))
}

/// Parse `loct help [command]` command - show help.
pub(super) fn parse_help_command(args: &[String]) -> Result<Command, String> {
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
    fn test_parse_crowd_command() {
        let args = vec!["message".into()];
        let result = parse_crowd_command(&args).unwrap();
        if let Command::Crowd(opts) = result {
            assert_eq!(opts.pattern, Some("message".into()));
        } else {
            panic!("Expected Crowd command");
        }
    }

    #[test]
    fn test_parse_crowd_auto_detect() {
        let args = vec!["--auto".into()];
        let result = parse_crowd_command(&args).unwrap();
        if let Command::Crowd(opts) = result {
            assert!(opts.auto_detect);
            assert!(opts.pattern.is_none());
        } else {
            panic!("Expected Crowd command");
        }
    }

    #[test]
    fn test_parse_tagmap_command() {
        let args = vec!["patient".into()];
        let result = parse_tagmap_command(&args).unwrap();
        if let Command::Tagmap(opts) = result {
            assert_eq!(opts.keyword, "patient");
        } else {
            panic!("Expected Tagmap command");
        }
    }

    #[test]
    fn test_parse_suppress_list() {
        let args = vec!["--list".into()];
        let result = parse_suppress_command(&args).unwrap();
        if let Command::Suppress(opts) = result {
            assert!(opts.list);
        } else {
            panic!("Expected Suppress command");
        }
    }

    #[test]
    fn test_parse_help_command() {
        let args = vec!["scan".into()];
        let result = parse_help_command(&args).unwrap();
        if let Command::Help(opts) = result {
            assert_eq!(opts.command, Some("scan".into()));
        } else {
            panic!("Expected Help command");
        }
    }

    #[test]
    fn test_parse_health_command() {
        let args = vec!["--include-tests".into()];
        let result = parse_health_command(&args).unwrap();
        if let Command::Health(opts) = result {
            assert!(opts.include_tests);
        } else {
            panic!("Expected Health command");
        }
    }
}
