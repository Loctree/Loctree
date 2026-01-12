//! Parsers for code analysis commands: dead, cycles, find, query, impact, twins, sniff.
//!
//! These commands analyze the codebase for issues, patterns, and relationships.

use std::path::PathBuf;

use super::super::command::{
    Command, CyclesOptions, DeadOptions, FindOptions, ImpactCommandOptions, QueryKind,
    QueryOptions, SniffOptions, TwinsOptions,
};

/// Parse `loct dead [options]` command - detect unused exports.
pub(super) fn parse_dead_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct dead - Detect unused exports / dead code

USAGE:
    loct dead [OPTIONS] [PATHS...]

DESCRIPTION:
    Finds exported symbols that are never imported anywhere in the codebase.
    Uses import graph analysis with alias-awareness to minimize false positives.

OPTIONS:
    --confidence <LEVEL>   Filter by confidence: high, medium, low (default: all)
    --top <N>              Limit to top N results (default: 20)
    --full, --all          Show all results (ignore top limit)
    --path <PATTERN>       Filter to files matching pattern
    --with-tests           Include test files in analysis
    --exclude-tests        Exclude test files (default)
    --with-helpers         Include helper/utility files
    --help, -h             Show this help message

EXAMPLES:
    loct dead                          # All dead exports
    loct dead --confidence high        # Only high-confidence
    loct dead --path src/components/   # Dead exports in components
    loct dead --top 50                 # Top 50 dead exports"
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
            "--with-shadows" => {
                opts.with_shadows = true;
                i += 1;
            }
            "--with-ambient" | "--include-ambient" => {
                opts.with_ambient = true;
                i += 1;
            }
            "--with-dynamic" | "--include-dynamic" => {
                opts.with_dynamic = true;
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

/// Parse `loct cycles [options]` command - detect circular imports.
pub(super) fn parse_cycles_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct cycles - Detect circular import chains

USAGE:
    loct cycles [OPTIONS] [PATHS...]

DESCRIPTION:
    Detects circular dependencies in your import graph and classifies them
    by compilability impact.

OPTIONS:
    --path <PATTERN>     Filter to files matching path pattern
    --breaking-only      Only show cycles that would break compilation
    --explain            Show detailed explanation for each cycle
    --legacy             Use legacy output format (old grouping by pattern)
    --help, -h           Show this help message

EXAMPLES:
    loct cycles                       # Show all cycles with new format
    loct cycles --breaking-only       # Only show compilation-breaking cycles
    loct cycles --explain             # Detailed pattern explanations"
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
            "--breaking-only" => {
                opts.breaking_only = true;
                i += 1;
            }
            "--explain" => {
                opts.explain = true;
                i += 1;
            }
            "--legacy" => {
                opts.legacy_format = true;
                i += 1;
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

/// Parse `loct find [options]` command - semantic search for symbols.
pub(super) fn parse_find_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct find - Semantic search for symbols by name pattern

USAGE:
    loct find [QUERY...] [OPTIONS]

DESCRIPTION:
    Semantic search for symbols (functions, classes, types) matching name patterns.
    Uses regex patterns. Multiple queries are combined with OR logic.
    Uses snapshot for instant results (15x faster than re-scanning).

OPTIONS:
    --symbol <PATTERN>, -s <PATTERN>    Search for symbols matching regex
    --pattern <PATTERN>                 Alias for --symbol (regex)
    --file <PATTERN>, -f <PATTERN>      Search for files matching regex
    --similar <SYMBOL>                  Find symbols with similar names (fuzzy)
    --dead                              Only show dead/unused symbols
    --exported                          Only show exported symbols
    --lang <LANG>                       Filter by language (ts, rs, js, py, etc.)
    --limit <N>                         Maximum results to show (default: unlimited)
    --help, -h                          Show this help message

EXAMPLES:
    loct find Patient                   # Find symbols containing \"Patient\"
    loct find foo bar baz               # Multi-query: find any of these (NEW!)
    loct find --symbol \".*Config$\"      # Regex: symbols ending with Config"
            .to_string());
    }

    let mut opts = FindOptions::default();
    let mut queries: Vec<String> = Vec::new();
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
                // Collect all positional args as queries (multi-query support!)
                queries.push(arg.clone());
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'find' command.", arg));
            }
        }
    }

    // Combine multiple queries with | for regex OR matching
    if !queries.is_empty() {
        opts.query = Some(queries.join("|"));
    }

    // Validate that at least one search criterion is specified and not empty
    let effective_query = opts
        .query
        .as_ref()
        .or(opts.symbol.as_ref())
        .or(opts.file.as_ref())
        .or(opts.similar.as_ref())
        .or(opts.impact.as_ref());

    if effective_query.is_some_and(|q| q.trim().is_empty()) {
        return Err("Error: Query cannot be empty".to_string());
    }

    Ok(Command::Find(opts))
}

/// Parse `loct query <kind> <target>` command - graph queries.
pub(super) fn parse_query_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct query - Graph queries (who-imports, who-exports, etc.)

USAGE:
    loct query <KIND> <TARGET>

QUERY KINDS:
    who-imports <FILE>        Find all files that import the specified file
    where-symbol <SYMBOL>     Find where a symbol is defined/exported
    component-of <FILE>       Show which components/modules contain this file

EXAMPLES:
    loct query who-imports src/utils.ts
    loct query where-symbol PatientRecord"
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

/// Parse `loct impact <file> [options]` command - analyze impact of file changes.
pub(super) fn parse_impact_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct impact - Analyze impact of modifying/removing a file

USAGE:
    loct impact <FILE> [OPTIONS]

OPTIONS:
    --depth <N>          Limit traversal depth (default: unlimited)
    --root <PATH>        Project root (default: current directory)
    --help, -h           Show this help message

EXAMPLES:
    loct impact src/utils.ts
    loct impact src/api.ts --depth 2"
            .to_string());
    }

    let mut opts = ImpactCommandOptions::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--depth" | "--max-depth" => {
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
                        "Unexpected argument '{}'. impact takes one target path.",
                        arg
                    ));
                }
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option '{}' for 'impact' command.", arg));
            }
        }
    }

    if opts.target.is_empty() {
        return Err(
            "'impact' command requires a target file path. Usage: loct impact <path>".to_string(),
        );
    }

    Ok(Command::Impact(opts))
}

/// Parse `loct twins [options]` command - find dead parrots and duplicate exports.
pub(super) fn parse_twins_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err(
            "loct twins - Find dead parrots (0 imports) and duplicate exports

USAGE:
    loct twins [OPTIONS] [PATH]

OPTIONS:
    --path <DIR>       Root directory to analyze (default: current directory)
    --dead-only        Show only dead parrots (exports with 0 imports)
    --include-tests    Include test files in analysis (excluded by default)
    --help, -h         Show this help message

EXAMPLES:
    loct twins
    loct twins --dead-only"
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
            "--include-suppressed" => {
                opts.include_suppressed = true;
                i += 1;
            }
            "--include-tests" => {
                opts.include_tests = true;
                i += 1;
            }
            "--ignore-conventions" => {
                opts.ignore_conventions = true;
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

/// Parse `loct sniff [options]` command - aggregate code smell analysis.
pub(super) fn parse_sniff_command(args: &[String]) -> Result<Command, String> {
    // Check for help flag first
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Err("loct sniff - Sniff for code smells (aggregate analysis)

USAGE:
    loct sniff [OPTIONS]

OPTIONS:
    --path <DIR>           Root directory to analyze (default: current directory)
    --dead-only            Show only dead parrots (skip twins and crowds)
    --twins-only           Show only twins (skip dead parrots and crowds)
    --crowds-only          Show only crowds (skip twins and dead parrots)
    --include-tests        Include test files in analysis (default: false)
    --min-crowd-size <N>   Minimum crowd size to report (default: 2)
    --help, -h             Show this help message

EXAMPLES:
    loct sniff
    loct sniff --dead-only"
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dead_command() {
        let args = vec!["--confidence".into(), "high".into()];
        let result = parse_dead_command(&args).unwrap();
        if let Command::Dead(opts) = result {
            assert_eq!(opts.confidence, Some("high".into()));
        } else {
            panic!("Expected Dead command");
        }
    }

    #[test]
    fn test_parse_cycles_command() {
        let args = vec!["--breaking-only".into()];
        let result = parse_cycles_command(&args).unwrap();
        if let Command::Cycles(opts) = result {
            assert!(opts.breaking_only);
        } else {
            panic!("Expected Cycles command");
        }
    }

    #[test]
    fn test_parse_find_with_regex() {
        let args = vec![
            "--symbol".into(),
            ".*patient.*".into(),
            "--lang".into(),
            "ts".into(),
        ];
        let result = parse_find_command(&args).unwrap();
        if let Command::Find(opts) = result {
            assert_eq!(opts.symbol, Some(".*patient.*".into()));
            assert_eq!(opts.lang, Some("ts".into()));
        } else {
            panic!("Expected Find command");
        }
    }

    #[test]
    fn test_parse_query_who_imports() {
        let args = vec!["who-imports".into(), "src/utils.ts".into()];
        let result = parse_query_command(&args).unwrap();
        if let Command::Query(opts) = result {
            assert!(matches!(opts.kind, QueryKind::WhoImports));
            assert_eq!(opts.target, "src/utils.ts");
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_parse_twins_command() {
        let args = vec!["--dead-only".into()];
        let result = parse_twins_command(&args).unwrap();
        if let Command::Twins(opts) = result {
            assert!(opts.dead_only);
        } else {
            panic!("Expected Twins command");
        }
    }
}
