use std::collections::HashSet;
use std::path::PathBuf;

use crate::types::{ColorMode, Mode, OutputMode, DEFAULT_LOC_THRESHOLD};

pub struct ParsedArgs {
    pub extensions: Option<HashSet<String>>,
    pub ignore_patterns: Vec<String>,
    pub ignore_symbols: Option<HashSet<String>>,
    pub ignore_symbols_preset: Option<String>,
    pub focus_patterns: Vec<String>,
    pub exclude_report_patterns: Vec<String>,
    pub graph: bool,
    pub use_gitignore: bool,
    pub max_depth: Option<usize>,
    pub color: ColorMode,
    pub output: OutputMode,
    pub json_output_path: Option<PathBuf>,
    pub summary: bool,
    pub summary_limit: usize,
    pub show_help: bool,
    pub show_version: bool,
    pub root_list: Vec<PathBuf>,
    pub py_roots: Vec<PathBuf>,
    pub show_hidden: bool,
    pub loc_threshold: usize,
    pub mode: Mode,
    pub analyze_limit: usize,
    pub report_path: Option<PathBuf>,
    pub serve: bool,
    pub serve_once: bool,
    pub serve_port: Option<u16>,
    pub editor_cmd: Option<String>,
    pub editor_kind: Option<String>,
    pub max_graph_nodes: Option<usize>,
    pub max_graph_edges: Option<usize>,
    pub verbose: bool,
    pub tauri_preset: bool,
    pub styles_preset: bool,
    pub fail_on_missing_handlers: bool,
    pub fail_on_ghost_events: bool,
    pub fail_on_races: bool,
    pub ai_mode: bool,
    pub top_dead_symbols: usize,
    pub skip_dead_symbols: bool,
}

impl Default for ParsedArgs {
    fn default() -> Self {
        Self {
            extensions: None,
            ignore_patterns: Vec::new(),
            ignore_symbols: None,
            ignore_symbols_preset: None,
            focus_patterns: Vec::new(),
            exclude_report_patterns: Vec::new(),
            graph: false,
            use_gitignore: false,
            max_depth: None,
            color: ColorMode::Auto,
            output: OutputMode::Human,
            json_output_path: None,
            summary: false,
            summary_limit: 5,
            show_help: false,
            show_version: false,
            root_list: Vec::new(),
            py_roots: Vec::new(),
            show_hidden: false,
            loc_threshold: DEFAULT_LOC_THRESHOLD,
            mode: Mode::Tree,
            analyze_limit: 8,
            report_path: None,
            serve: false,
            serve_once: false,
            serve_port: None,
            editor_cmd: None,
            editor_kind: None,
            max_graph_nodes: None,
            max_graph_edges: None,
            verbose: false,
            tauri_preset: false,
            styles_preset: false,
            fail_on_missing_handlers: false,
            fail_on_ghost_events: false,
            fail_on_races: false,
            ai_mode: false,
            top_dead_symbols: 20,
            skip_dead_symbols: false,
        }
    }
}

fn parse_color_mode(raw: &str) -> Result<ColorMode, String> {
    match raw {
        "auto" => Ok(ColorMode::Auto),
        "always" => Ok(ColorMode::Always),
        "never" => Ok(ColorMode::Never),
        _ => Err("--color expects auto|always|never".to_string()),
    }
}

fn parse_summary_limit(raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|_| "--summary expects a positive integer".to_string())?;
    if value == 0 {
        Err("--summary expects a positive integer".to_string())
    } else {
        Ok(value)
    }
}

pub fn parse_extensions(raw: &str) -> Option<HashSet<String>> {
    let set: HashSet<String> = raw
        .split(',')
        .filter_map(|segment| {
            let trimmed = segment.trim().trim_start_matches('.').to_lowercase();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect();
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

fn parse_glob_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

fn parse_positive_usize(raw: &str, flag: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires a positive integer"))?;
    if value == 0 {
        Err(format!("{flag} requires a positive integer"))
    } else {
        Ok(value)
    }
}

fn parse_port(raw: &str, flag: &str) -> Result<u16, String> {
    let value = raw
        .parse::<u16>()
        .map_err(|_| format!("{flag} requires a port number (0-65535)"))?;
    Ok(value)
}

fn validate_globs(patterns: &[String], flag: &str) -> Result<(), String> {
    for pat in patterns {
        if pat.trim().is_empty() {
            continue;
        }
        globset::Glob::new(pat).map_err(|e| format!("{flag}: invalid glob '{pat}': {e}"))?;
    }
    Ok(())
}

fn detect_glob_conflicts(focus: &[String], exclude: &[String]) -> Result<(), String> {
    if focus.is_empty() || exclude.is_empty() {
        return Ok(());
    }
    let focus_set: std::collections::HashSet<_> = focus.iter().collect();
    let exclude_set: std::collections::HashSet<_> = exclude.iter().collect();
    let duplicates: Vec<_> = focus_set
        .intersection(&exclude_set)
        .map(|s| s.to_string())
        .collect();
    if !duplicates.is_empty() {
        return Err(format!(
            "Conflicting globs between --focus and --exclude-report: {}",
            duplicates.join(", ")
        ));
    }
    Ok(())
}

pub fn parse_ignore_symbols(raw: &str) -> Option<HashSet<String>> {
    let set: HashSet<String> = raw
        .split(',')
        .filter_map(|segment| {
            let trimmed = segment.trim().to_lowercase();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect();
    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

pub fn preset_ignore_symbols(name: &str) -> Option<HashSet<String>> {
    match name.to_lowercase().as_str() {
        "common" => Some(
            ["main", "run", "setup", "test_*", "tests_*"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        "tauri" => Some(
            [
                // language-level boilerplate often duplicated across crates/configs
                "default", "new", "from", "try_from", "from_str", "into", "build", "init", "config",
                "main", "run", "setup", // python/ts interop noise
                "__all__", "__init__", "test_*", "tests_*",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        ),
        _ => None,
    }
}

pub fn parse_args() -> Result<ParsedArgs, String> {
    // nosemgrep:rust.lang.security.args-os.args-os - CLI arg parsing only; not used for security-sensitive decisions
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    let mut parsed = ParsedArgs {
        ..ParsedArgs::default()
    };

    let mut roots: Vec<PathBuf> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "tauri" | "--preset-tauri" => {
                parsed.tauri_preset = true;
                i += 1;
            }
            "styles" | "--preset-styles" => {
                parsed.styles_preset = true;
                parsed.mode = Mode::AnalyzeImports;
                i += 1;
            }
            "init" | "--init" => {
                parsed.mode = Mode::Init;
                i += 1;
            }
            "--ai" => {
                parsed.ai_mode = true;
                parsed.output = OutputMode::Json;
                parsed.summary = true;
                parsed.mode = Mode::AnalyzeImports;
                i += 1;
            }
            "--help" | "-h" => {
                parsed.show_help = true;
                i += 1;
            }
            "--version" | "-V" => {
                parsed.show_version = true;
                i += 1;
            }
            "--color" | "-c" => {
                if let Some(next) = args.get(i + 1) {
                    if !next.starts_with('-') {
                        parsed.color = parse_color_mode(next)?;
                        i += 2;
                        continue;
                    }
                }
                parsed.color = ColorMode::Always;
                i += 1;
            }
            _ if arg.starts_with("--color=") => {
                let value = arg.trim_start_matches("--color=");
                parsed.color = parse_color_mode(value)?;
                i += 1;
            }
            "--gitignore" | "-g" => {
                parsed.use_gitignore = true;
                i += 1;
            }
            "--graph" => {
                parsed.graph = true;
                i += 1;
            }
            "--verbose" | "-v" => {
                parsed.verbose = true;
                i += 1;
            }
            "--fail-on-missing-handlers" => {
                parsed.fail_on_missing_handlers = true;
                i += 1;
            }
            "--fail-on-ghost-events" => {
                parsed.fail_on_ghost_events = true;
                i += 1;
            }
            "--fail-on-races" => {
                parsed.fail_on_races = true;
                i += 1;
            }
            "--show-hidden" | "-H" => {
                parsed.show_hidden = true;
                i += 1;
            }
            "--json" => {
                parsed.output = OutputMode::Json;
                if let Some(next) = args.get(i + 1) {
                    if !next.starts_with('-') {
                        parsed.json_output_path = Some(PathBuf::from(next));
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
            "--json-out" | "--json-output" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--json-out requires a file path".to_string())?;
                parsed.output = OutputMode::Json;
                parsed.json_output_path = Some(PathBuf::from(next));
                i += 2;
            }
            "--jsonl" => {
                parsed.output = OutputMode::Jsonl;
                i += 1;
            }
            "--html-report" | "--report" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--html-report requires a file path".to_string())?;
                parsed.report_path = Some(PathBuf::from(next));
                i += 2;
            }
            "--serve" | "--serve-keepalive" | "--serve-wait" => {
                parsed.serve = true;
                i += 1;
            }
            "--serve-once" => {
                parsed.serve = true;
                parsed.serve_once = true;
                i += 1;
            }
            "--port" | "--serve-port" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--port requires a value".to_string())?;
                parsed.serve_port = Some(parse_port(next, "--port")?);
                i += 2;
            }
            "--editor-cmd" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--editor-cmd requires a command template".to_string())?;
                parsed.editor_cmd = Some(next.clone());
                i += 2;
            }
            "--editor" => {
                let next = args.get(i + 1).ok_or_else(|| {
                    "--editor requires a value (code|cursor|windsurf|jetbrains|none)".to_string()
                })?;
                parsed.editor_kind = Some(next.clone());
                i += 2;
            }
            _ if arg.starts_with("--editor=") => {
                let value = arg.trim_start_matches("--editor=");
                parsed.editor_kind = Some(value.to_string());
                i += 1;
            }
            "--summary" => {
                parsed.summary = true;
                if let Some(next) = args.get(i + 1) {
                    if !next.starts_with('-') {
                        parsed.summary_limit = parse_summary_limit(next)?;
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
            _ if arg.starts_with("--summary=") => {
                let value = arg.trim_start_matches("--summary=");
                parsed.summary = true;
                parsed.summary_limit = parse_summary_limit(value)?;
                i += 1;
            }
            "--loc" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--loc requires a positive integer".to_string())?;
                parsed.loc_threshold = parse_positive_usize(next, "--loc")?;
                i += 2;
            }
            "--limit" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a positive integer".to_string())?;
                parsed.analyze_limit = parse_positive_usize(next, "--limit")?;
                i += 2;
            }
            "--top-dead-symbols" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--top-dead-symbols requires a positive integer".to_string())?;
                parsed.top_dead_symbols = parse_positive_usize(next, "--top-dead-symbols")?;
                i += 2;
            }
            "--skip-dead-symbols" => {
                parsed.skip_dead_symbols = true;
                i += 1;
            }
            "--analyze-imports" | "-A" => {
                parsed.mode = Mode::AnalyzeImports;
                i += 1;
            }
            "-L" | "--max-depth" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "-L/--max-depth requires a non-negative integer".to_string())?;
                let depth = next
                    .parse::<usize>()
                    .map_err(|_| "-L/--max-depth requires a non-negative integer".to_string())?;
                parsed.max_depth = Some(depth);
                i += 2;
            }
            "--ext" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--ext requires a comma-separated value".to_string())?;
                parsed.extensions = parse_extensions(next);
                i += 2;
            }
            _ if arg.starts_with("--ext=") => {
                let value = arg.trim_start_matches("--ext=");
                parsed.extensions = parse_extensions(value);
                i += 1;
            }
            "--ignore-symbols" => {
                let next = args.get(i + 1).ok_or_else(|| {
                    "--ignore-symbols requires a comma-separated list".to_string()
                })?;
                parsed.ignore_symbols = parse_ignore_symbols(next);
                i += 2;
            }
            _ if arg.starts_with("--ignore-symbols=") => {
                let value = arg.trim_start_matches("--ignore-symbols=");
                parsed.ignore_symbols = parse_ignore_symbols(value);
                i += 1;
            }
            "--ignore-symbols-preset" => {
                let next = args.get(i + 1).ok_or_else(|| {
                    "--ignore-symbols-preset requires a name (e.g. common)".to_string()
                })?;
                parsed.ignore_symbols_preset = Some(next.clone());
                i += 2;
            }
            _ if arg.starts_with("--ignore-symbols-preset=") => {
                let value = arg.trim_start_matches("--ignore-symbols-preset=");
                parsed.ignore_symbols_preset = Some(value.to_string());
                i += 1;
            }
            "--focus" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--focus requires a glob or comma list".to_string())?;
                parsed.focus_patterns.extend(parse_glob_list(next));
                i += 2;
            }
            _ if arg.starts_with("--focus=") => {
                let value = arg.trim_start_matches("--focus=");
                parsed.focus_patterns.extend(parse_glob_list(value));
                i += 1;
            }
            "--exclude-report" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--exclude-report requires a glob or comma list".to_string())?;
                parsed.exclude_report_patterns.extend(parse_glob_list(next));
                i += 2;
            }
            _ if arg.starts_with("--exclude-report=") => {
                let value = arg.trim_start_matches("--exclude-report=");
                parsed
                    .exclude_report_patterns
                    .extend(parse_glob_list(value));
                i += 1;
            }
            "-I" | "--ignore" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "-I/--ignore requires a path argument".to_string())?;
                parsed.ignore_patterns.push(next.clone());
                i += 2;
            }
            "--max-nodes" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--max-nodes requires a positive integer".to_string())?;
                parsed.max_graph_nodes = Some(parse_positive_usize(next, "--max-nodes")?);
                i += 2;
            }
            "--max-edges" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--max-edges requires a positive integer".to_string())?;
                parsed.max_graph_edges = Some(parse_positive_usize(next, "--max-edges")?);
                i += 2;
            }
            "--py-root" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "--py-root requires a path".to_string())?;
                parsed.py_roots.push(PathBuf::from(next));
                i += 2;
            }
            _ if arg.starts_with("--py-root=") => {
                let value = arg.trim_start_matches("--py-root=");
                parsed.py_roots.push(PathBuf::from(value));
                i += 1;
            }
            _ if arg.starts_with('-') => {
                eprintln!("Ignoring unknown flag {}", arg);
                i += 1;
            }
            _ => {
                let trimmed = arg.trim();
                if !trimmed.is_empty() {
                    roots.push(PathBuf::from(trimmed));
                }
                i += 1;
            }
        }
    }

    if parsed.tauri_preset {
        if parsed.extensions.is_none() {
            parsed.extensions = Some(
                ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            );
        }
        if roots.is_empty() {
            roots.push(PathBuf::from("."));
        }
        parsed.mode = Mode::AnalyzeImports;
        parsed.graph = true;
        parsed.use_gitignore = true;
        if parsed.ignore_patterns.is_empty() {
            parsed.ignore_patterns.extend(
                [
                    "node_modules",
                    "dist",
                    "target",
                    "build",
                    "coverage",
                    "docs/*.json",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
        }
        if parsed.ignore_symbols.is_none() && parsed.ignore_symbols_preset.is_none() {
            parsed.ignore_symbols_preset = Some("tauri".to_string());
        }
    }

    // Default to Init mode when running bare `loctree` without any mode-setting flags
    // This implements "scan once" - bare loctree creates/updates the snapshot
    if roots.is_empty()
        && matches!(parsed.mode, Mode::Tree)
        && !parsed.summary
        && parsed.extensions.is_none()
    {
        parsed.mode = Mode::Init;
        parsed.use_gitignore = true;
    }

    if roots.is_empty() {
        roots.push(PathBuf::from("."));
    }
    for root in &roots {
        if !root.exists() {
            return Err(format!(
                "Path '{}' does not exist. Provide a valid file or directory.",
                root.display()
            ));
        }
        if root.is_file() && matches!(parsed.mode, Mode::AnalyzeImports) {
            return Err(format!(
                "Path '{}' is a file; import analyzer expects a directory.",
                root.display()
            ));
        }
    }
    parsed.root_list = roots;

    validate_globs(&parsed.focus_patterns, "--focus")?;
    validate_globs(&parsed.exclude_report_patterns, "--exclude-report")?;
    detect_glob_conflicts(&parsed.focus_patterns, &parsed.exclude_report_patterns)?;

    if parsed.serve && parsed.report_path.is_none() {
        return Err("--serve requires --html-report to be set".to_string());
    }

    for extra in &parsed.py_roots {
        if !extra.exists() {
            return Err(format!(
                "--py-root '{}' does not exist. Provide a valid directory.",
                extra.display()
            ));
        }
        if !extra.is_dir() {
            return Err(format!(
                "--py-root '{}' is not a directory.",
                extra.display()
            ));
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extensions() {
        let res = parse_extensions("rs,ts").expect("parse extensions");
        assert!(res.contains("rs"));
        assert!(res.contains("ts"));
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn test_parse_extensions_empty() {
        assert!(parse_extensions("").is_none());
    }

    #[test]
    fn test_parse_color_mode() {
        assert_eq!(
            parse_color_mode("always").expect("color always"),
            ColorMode::Always
        );
        assert_eq!(
            parse_color_mode("never").expect("color never"),
            ColorMode::Never
        );
        assert!(parse_color_mode("invalid").is_err());
    }

    #[test]
    fn test_parse_summary_limit() {
        assert_eq!(parse_summary_limit("5").expect("summary"), 5);
        assert!(parse_summary_limit("0").is_err());
        assert!(parse_summary_limit("abc").is_err());
    }

    #[test]
    fn detects_glob_conflicts() {
        let focus = vec!["src/**".to_string(), "pkg/**".to_string()];
        let exclude = vec!["pkg/**".to_string()];
        assert!(detect_glob_conflicts(&focus, &exclude).is_err());
    }

    #[test]
    fn allows_distinct_globs() {
        let focus = vec!["src/**".to_string()];
        let exclude = vec!["tests/**".to_string()];
        assert!(detect_glob_conflicts(&focus, &exclude).is_ok());
    }
}
