//! Parsers for miscellaneous commands: crowd, tagmap, suppress, dist, layoutmap,
//! zombie, health, audit, doctor, help.
//!
//! These commands handle various specialized functionality.

use std::path::PathBuf;

use super::super::command::{
    AuditOptions, CacheAction, CacheOptions, Command, CrowdOptions, DistOptions, DoctorOptions,
    HealthOptions, HelpOptions, LayoutmapOptions, PlanOptions, SuppressOptions, TagmapOptions,
    ZombieOptions,
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
        if let Some(help) = Command::format_command_help("dist") {
            return Err(help.to_string());
        }
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
                opts.source_maps.push(PathBuf::from(value));
                i += 2;
            }
            "--src" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--src requires a directory path".to_string())?;
                opts.src = Some(PathBuf::from(value));
                i += 2;
            }
            "--report" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--report requires a file path".to_string())?;
                opts.report_path = Some(PathBuf::from(value));
                i += 2;
            }
            _ if !arg.starts_with('-') => {
                // Legacy positional shorthand: first is source-map input, second is src
                if opts.source_maps.is_empty() {
                    opts.source_maps.push(PathBuf::from(arg));
                } else if opts.src.is_none() {
                    opts.src = Some(PathBuf::from(arg));
                } else {
                    return Err(format!(
                        "Unexpected argument '{}'. dist takes --src, repeated --source-map, and optional --report.",
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

    if opts.source_maps.is_empty() {
        return Err(
            "'dist' command requires at least one --source-map <path>. Usage: loct dist --src src/ --source-map dist/ or loct dist --src src/ --source-map dist/main.js.map --source-map dist/chunks/"
                .to_string(),
        );
    }

    if opts.src.is_none() {
        return Err(
            "'dist' command requires --src <dir>. Usage: loct dist --src src/ --source-map dist/main.js.map"
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
            "--todos" | "-t" => {
                opts.todos = true;
                i += 1;
            }
            "--limit" => {
                i += 1;
                if i < args.len() {
                    opts.limit = Some(
                        args[i]
                            .parse()
                            .map_err(|_| format!("Invalid limit value: {}", args[i]))?,
                    );
                    i += 1;
                } else {
                    return Err("--limit requires a numeric value".to_string());
                }
            }
            "--stdout" => {
                return Err(
                    "`loct audit` writes markdown reports to an artifact file only. Use `--json` for stdout-oriented automation.".to_string(),
                );
            }
            "--no-open" => {
                opts.no_open = true;
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

/// Parse `loct plan [options] [path]` command - generate refactoring plan.
pub(super) fn parse_plan_command(args: &[String]) -> Result<Command, String> {
    // Check for --help first
    if args.iter().any(|a| a == "--help" || a == "-h")
        && let Some(help) = Command::format_command_help("plan")
    {
        println!("{}", help);
        std::process::exit(0);
    }

    let mut opts = PlanOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--target-layout" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--target-layout requires a value".to_string())?;
                opts.target_layout = Some(value.clone());
                i += 2;
            }
            "--markdown" | "--md" => {
                opts.markdown = true;
                i += 1;
            }
            "--json" => {
                opts.json = true;
                i += 1;
            }
            "--script" | "--sh" => {
                opts.script = true;
                i += 1;
            }
            "--all" => {
                opts.all = true;
                i += 1;
            }
            "--output" | "-o" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--output requires a path".to_string())?;
                opts.output = Some(PathBuf::from(value));
                i += 2;
            }
            "--no-open" => {
                opts.no_open = true;
                i += 1;
            }
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            "--min-coupling" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--min-coupling requires a number".to_string())?;
                opts.min_coupling = Some(
                    value
                        .parse()
                        .map_err(|_| "--min-coupling requires a number (0.0-1.0)")?,
                );
                i += 2;
            }
            "--max-module-size" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--max-module-size requires a number".to_string())?;
                opts.max_module_size = Some(
                    value
                        .parse()
                        .map_err(|_| "--max-module-size requires a number")?,
                );
                i += 2;
            }
            _ => {
                // Treat as root path
                if arg.starts_with("--") {
                    return Err(format!("Unknown option '{}' for 'plan' command.", arg));
                }
                opts.roots.push(PathBuf::from(arg));
                i += 1;
            }
        }
    }

    // Default to markdown if no format specified
    if !opts.markdown && !opts.json && !opts.script && !opts.all {
        opts.markdown = true;
    }

    Ok(Command::Plan(opts))
}

/// Parse `loct cache <list|clean> [options]` command.
pub(super) fn parse_cache_command(args: &[String]) -> Result<Command, String> {
    if args.iter().any(|a| a == "--help" || a == "-h") || args.is_empty() {
        return Err("loct cache - Manage snapshot cache

USAGE:
    loct cache <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    list                   List cached projects with sizes and ages
    clean                  Remove cached snapshots

CLEAN OPTIONS:
    --project <DIR>        Only clean cache for a specific project
    --older-than <DAYS>d   Only remove entries older than N days (e.g., 7d, 30d)
    --force, -f            Skip confirmation prompt

EXAMPLES:
    loct cache list                        # Show all cached projects
    loct cache clean                       # Remove all (with confirmation)
    loct cache clean --force               # Remove all without asking
    loct cache clean --project .           # Clean cache for current project
    loct cache clean --older-than 30d      # Remove entries older than 30 days"
            .to_string());
    }

    let sub = args[0].as_str();
    let sub_args = &args[1..];

    match sub {
        "list" | "ls" => Ok(Command::Cache(CacheOptions {
            action: CacheAction::List,
        })),
        "clean" | "rm" | "purge" => {
            let mut project = None;
            let mut older_than = None;
            let mut force = false;
            let mut i = 0;
            while i < sub_args.len() {
                match sub_args[i].as_str() {
                    "--project" | "-p" => {
                        i += 1;
                        if i >= sub_args.len() {
                            return Err("--project requires a directory argument".to_string());
                        }
                        project = Some(PathBuf::from(&sub_args[i]));
                    }
                    "--older-than" => {
                        i += 1;
                        if i >= sub_args.len() {
                            return Err(
                                "--older-than requires a duration (e.g., 7d, 30d)".to_string()
                            );
                        }
                        older_than = Some(sub_args[i].clone());
                    }
                    "--force" | "-f" => force = true,
                    other => return Err(format!("Unknown cache clean option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Cache(CacheOptions {
                action: CacheAction::Clean {
                    project,
                    older_than,
                    force,
                },
            }))
        }
        other => Err(format!(
            "Unknown cache subcommand '{}'. Use 'list' or 'clean'.",
            other
        )),
    }
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
        let result = parse_crowd_command(&args).expect("parse crowd command");
        if let Command::Crowd(opts) = result {
            assert_eq!(opts.pattern, Some("message".into()));
        } else {
            panic!("Expected Crowd command");
        }
    }

    #[test]
    fn test_parse_crowd_auto_detect() {
        let args = vec!["--auto".into()];
        let result = parse_crowd_command(&args).expect("parse crowd auto command");
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
        let result = parse_tagmap_command(&args).expect("parse tagmap command");
        if let Command::Tagmap(opts) = result {
            assert_eq!(opts.keyword, "patient");
        } else {
            panic!("Expected Tagmap command");
        }
    }

    #[test]
    fn test_parse_suppress_list() {
        let args = vec!["--list".into()];
        let result = parse_suppress_command(&args).expect("parse suppress command");
        if let Command::Suppress(opts) = result {
            assert!(opts.list);
        } else {
            panic!("Expected Suppress command");
        }
    }

    #[test]
    fn test_parse_help_command() {
        let args = vec!["scan".into()];
        let result = parse_help_command(&args).expect("parse help command");
        if let Command::Help(opts) = result {
            assert_eq!(opts.command, Some("scan".into()));
        } else {
            panic!("Expected Help command");
        }
    }

    #[test]
    fn test_parse_cache_list() {
        let args = vec!["list".into()];
        let result = parse_cache_command(&args).expect("parse cache list command");
        assert!(matches!(
            result,
            Command::Cache(CacheOptions {
                action: CacheAction::List
            })
        ));
    }

    #[test]
    fn test_parse_cache_clean() {
        let args: Vec<String> = vec!["clean".into(), "--force".into()];
        let result = parse_cache_command(&args).expect("parse cache clean command");
        if let Command::Cache(CacheOptions {
            action: CacheAction::Clean { force, .. },
        }) = result
        {
            assert!(force);
        } else {
            panic!("Expected Cache Clean command");
        }
    }

    #[test]
    fn test_parse_health_command() {
        let args = vec!["--include-tests".into()];
        let result = parse_health_command(&args).expect("parse health command");
        if let Command::Health(opts) = result {
            assert!(opts.include_tests);
        } else {
            panic!("Expected Health command");
        }
    }

    #[test]
    fn test_parse_audit_command_defaults_to_full_report() {
        let args: Vec<String> = vec![];
        let result = parse_audit_command(&args).expect("parse audit command");
        if let Command::Audit(opts) = result {
            assert_eq!(opts.limit, None);
            assert!(!opts.todos);
        } else {
            panic!("Expected Audit command");
        }
    }

    #[test]
    fn test_parse_audit_command_accepts_explicit_limit() {
        let args = vec!["--limit".into(), "7".into()];
        let result = parse_audit_command(&args).expect("parse audit command with limit");
        if let Command::Audit(opts) = result {
            assert_eq!(opts.limit, Some(7));
        } else {
            panic!("Expected Audit command");
        }
    }

    #[test]
    fn test_parse_audit_command_rejects_stdout() {
        let args = vec!["--stdout".into()];
        let err = parse_audit_command(&args).expect_err("audit should reject stdout");
        assert!(err.contains("writes markdown reports to an artifact file only"));
    }
}
