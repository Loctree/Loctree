use std::cmp::Ordering;
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::json;

const LARGE_FILE_THRESHOLD: usize = 1000;
const COLOR_RED: &str = "\u{001b}[31m";
const COLOR_RESET: &str = "\u{001b}[0m";

struct LineEntry {
    label: String,
    loc: Option<usize>,
    relative_path: String,
    is_dir: bool,
    is_large: bool,
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

struct ParsedArgs {
    extensions: Option<HashSet<String>>,
    ignore_patterns: Vec<String>,
    use_gitignore: bool,
    max_depth: Option<usize>,
    color: ColorMode,
    output: OutputMode,
    summary: bool,
    summary_limit: usize,
    show_help: bool,
    root_list: Vec<PathBuf>,
}

impl Default for ParsedArgs {
    fn default() -> Self {
        Self {
            extensions: None,
            ignore_patterns: Vec::new(),
            use_gitignore: false,
            max_depth: None,
            color: ColorMode::Auto,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 5,
            show_help: false,
            root_list: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Human,
    Json,
}

#[derive(Clone)]
struct Options {
    extensions: Option<HashSet<String>>,
    ignore_paths: Vec<PathBuf>,
    use_gitignore: bool,
    max_depth: Option<usize>,
    color: ColorMode,
    output: OutputMode,
    summary: bool,
    summary_limit: usize,
}

struct LargeEntry {
    path: String,
    loc: usize,
}

#[derive(Default)]
struct Stats {
    directories: usize,
    files: usize,
    files_with_loc: usize,
    total_loc: usize,
}

struct GitIgnoreChecker {
    root: PathBuf,
}

impl GitIgnoreChecker {
    fn new(root: &Path) -> Option<Self> {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?;
        if status.success() {
            Some(Self {
                root: root.to_path_buf(),
            })
        } else {
            None
        }
    }

    fn is_ignored(&self, relative_path: &Path) -> bool {
        if relative_path.as_os_str().is_empty() {
            return false;
        }
        Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .arg("check-ignore")
            .arg("-q")
            .arg(relative_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

fn parse_args() -> Result<ParsedArgs, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut parsed = ParsedArgs {
        ..ParsedArgs::default()
    };

    let mut roots: Vec<PathBuf> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--help" | "-h" => {
                parsed.show_help = true;
                i += 1;
            }
            "--color" | "-c" => {
                // Optional value: --color auto|always|never; shorthand sets always
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
            "--json" => {
                parsed.output = OutputMode::Json;
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
            "-I" | "--ignore" => {
                let next = args
                    .get(i + 1)
                    .ok_or_else(|| "-I/--ignore requires a path argument".to_string())?;
                parsed.ignore_patterns.push(next.clone());
                i += 2;
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
            _ if arg.starts_with('-') => {
                eprintln!("Ignoring unknown flag {}", arg);
                i += 1;
            }
            _ => {
                roots.push(PathBuf::from(arg));
                i += 1;
            }
        }
    }

    if roots.is_empty() {
        roots.push(PathBuf::from("."));
    }
    parsed.root_list = roots;

    Ok(parsed)
}

fn parse_extensions(raw: &str) -> Option<HashSet<String>> {
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

fn normalise_ignore_patterns(patterns: &[String], root: &Path) -> Vec<PathBuf> {
    patterns
        .iter()
        .map(|pattern| {
            let candidate = PathBuf::from(pattern);
            let full = if candidate.is_absolute() {
                candidate
            } else {
                root.join(candidate)
            };
            full.canonicalize().unwrap_or(full)
        })
        .collect()
}

fn count_lines(path: &Path) -> Option<usize> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut count = 0usize;
    for line in reader.lines() {
        if line.is_ok() {
            count += 1;
        }
    }
    Some(count)
}

fn should_ignore(
    full_path: &Path,
    relative_path: &Path,
    options: &Options,
    git_checker: Option<&GitIgnoreChecker>,
) -> bool {
    if options
        .ignore_paths
        .iter()
        .any(|ignored| full_path.starts_with(ignored))
    {
        return true;
    }
    if options.use_gitignore {
        if let Some(checker) = git_checker {
            if checker.is_ignored(relative_path) {
                return true;
            }
        }
    }
    false
}

fn walk(
    dir: &Path,
    options: &Options,
    prefix_parts: &mut Vec<bool>,
    entries: &mut Vec<LineEntry>,
    large_entries: &mut Vec<LargeEntry>,
    depth: usize,
    root: &Path,
    git_checker: Option<&GitIgnoreChecker>,
    stats: &mut Stats,
) -> io::Result<bool> {
    let mut dir_entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != ".DS_Store")
        .collect();

    dir_entries.sort_by(|a, b| {
        let a_path = a.path();
        let b_path = b.path();
        let a_is_dir = a_path.is_dir();
        let b_is_dir = b_path.is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&b.file_name().to_string_lossy().to_lowercase()),
        }
    });

    let len = dir_entries.len();
    let mut any_included = false;
    for (idx, entry) in dir_entries.into_iter().enumerate() {
        let path = entry.path();
        let is_last = idx + 1 == len;
        let mut prefix = String::new();
        for &has_more in prefix_parts.iter() {
            if has_more {
                prefix.push_str("│   ");
            } else {
                prefix.push_str("    ");
            }
        }
        let branch = if is_last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().to_string();
        let label = format!("{}{}{}", prefix, branch, name);

        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        if should_ignore(&path, &relative, options, git_checker) {
            continue;
        }

        let mut loc = None;
        let is_dir = path.is_dir();
        let mut include_current = false;

        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase();
            let matches_ext = options
                .extensions
                .as_ref()
                .map_or(true, |set| set.contains(&ext));
            if matches_ext {
                loc = count_lines(&path);
                if let Some(value) = loc {
                    stats.files += 1;
                    stats.files_with_loc += 1;
                    stats.total_loc += value;
                    if value >= LARGE_FILE_THRESHOLD {
                        let relative_display = if relative.as_os_str().is_empty() {
                            name.clone()
                        } else {
                            relative.to_string_lossy().to_string()
                        };
                        large_entries.push(LargeEntry {
                            path: relative_display.clone(),
                            loc: value,
                        });
                    }
                    include_current = true;
                }
            }
        }

        let relative_display = if relative.as_os_str().is_empty() {
            name.clone()
        } else {
            relative.to_string_lossy().to_string()
        };
        let is_large = loc.map_or(false, |v| v >= LARGE_FILE_THRESHOLD);

        if is_dir {
            if options.max_depth.map_or(true, |max| depth < max) {
                prefix_parts.push(!is_last);
                let child_has = walk(
                    &path,
                    options,
                    prefix_parts,
                    entries,
                    large_entries,
                    depth + 1,
                    root,
                    git_checker,
                    stats,
                )?;
                prefix_parts.pop();
                if child_has {
                    stats.directories += 1;
                    include_current = true;
                }
            }
        }

        if include_current {
            entries.push(LineEntry {
                label: label.clone(),
                loc,
                relative_path: relative_display,
                is_dir,
                is_large,
            });
            any_included = true;
        }
    }

    Ok(any_included)
}

fn format_usage() -> &'static str {
    "loc-tree (Rust)\n\nUsage: cargo run -- <root> [options]\n\nOptions:\n  --ext <list>         Comma-separated extensions to include (e.g. --ext rs,ts,tsx).\n  -I, --ignore <path>  Ignore a folder/file (relative or absolute). Repeatable.\n  --gitignore, -g      Respect current Git ignore rules (requires git).\n  -L, --max-depth <n>  Limit recursion depth (0 = only direct children).\n  --color[=mode]       Colorize large files. mode: auto|always|never (default auto).\n  --json               Emit JSON instead of a tree view.\n  --summary[=N]        Print totals and top large files (N entries, default 5).\n  --help, -h           Show this message.\n"
}

fn main() -> io::Result<()> {
    let parsed = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    if parsed.show_help {
        println!("{}", format_usage());
        return Ok(());
    }

    if parsed.max_depth.is_some() && parsed.max_depth.unwrap_or(0) == usize::MAX {
        eprintln!("Invalid max depth");
        std::process::exit(1);
    }

    let mut root_list: Vec<PathBuf> = Vec::new();
    for root in parsed.root_list.iter() {
        if !root.is_dir() {
            eprintln!("{} is not a directory", root.display());
            std::process::exit(1);
        }
        root_list.push(
            root
                .canonicalize()
                .unwrap_or_else(|_| root.clone()),
        );
    }

    let options = Options {
        extensions: parsed.extensions,
        ignore_paths: Vec::new(),
        use_gitignore: parsed.use_gitignore,
        max_depth: parsed.max_depth,
        color: parsed.color,
        output: parsed.output,
        summary: parsed.summary,
        summary_limit: parsed.summary_limit,
    };

    let mut json_results = Vec::new();

    for (idx, root_path) in root_list.iter().enumerate() {
        let ignore_paths = normalise_ignore_patterns(&parsed.ignore_patterns, root_path);
        let root_options = Options {
            ignore_paths,
            ..options.clone()
        };

        let git_checker = if root_options.use_gitignore {
            GitIgnoreChecker::new(root_path)
        } else {
            None
        };

        let mut entries: Vec<LineEntry> = Vec::new();
        let mut large_entries: Vec<LargeEntry> = Vec::new();
        let mut prefix_parts: Vec<bool> = Vec::new();
        let mut stats = Stats::default();

        walk(
            root_path,
            &root_options,
            &mut prefix_parts,
            &mut entries,
            &mut large_entries,
            0,
            root_path,
            git_checker.as_ref(),
            &mut stats,
        )?;

        let mut sorted_large = large_entries;
        sorted_large.sort_by(|a, b| b.loc.cmp(&a.loc));

        let summary = json!({
            "directories": stats.directories,
            "files": stats.files,
            "filesWithLoc": stats.files_with_loc,
            "totalLoc": stats.total_loc,
            "largeFiles": sorted_large
                .iter()
                .take(root_options.summary_limit)
                .map(|e| json!({"path": e.path, "loc": e.loc}))
                .collect::<Vec<_>>()
        });

        if root_options.output == OutputMode::Json {
            let entries_json: Vec<_> = entries
                .iter()
                .map(|entry| {
                    json!({
                        "path": entry.relative_path,
                        "type": if entry.is_dir { "dir" } else { "file" },
                        "loc": entry.loc,
                        "isLarge": entry.is_large,
                    })
                })
                .collect();

            let payload = json!({
                "root": root_path,
                "options": {
                    "exts": root_options.extensions.as_ref().map(|set| {
                        let mut exts: Vec<_> = set.iter().cloned().collect();
                        exts.sort();
                        exts
                    }),
                    "ignore": root_options
                        .ignore_paths
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>(),
                    "maxDepth": root_options.max_depth,
                    "useGitignore": root_options.use_gitignore,
                    "color": match root_options.color {
                        ColorMode::Auto => "auto",
                        ColorMode::Always => "always",
                        ColorMode::Never => "never",
                    },
                    "summary": if root_options.summary {
                        serde_json::Value::from(root_options.summary_limit)
                    } else {
                        serde_json::Value::Bool(false)
                    },
                },
                "summary": summary,
                "entries": entries_json,
            });

            json_results.push(payload);
            continue;
        }

        if idx > 0 {
            println!("");
        }

        if entries.is_empty() {
            println!("{}/ (empty)", root_path.display());
            continue;
        }

        let max_label_len = entries
            .iter()
            .map(|entry| entry.label.len())
            .max()
            .unwrap_or(0);
        let root_name = root_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root_path.display().to_string());

        let color_enabled = matches!(root_options.color, ColorMode::Always)
            || (matches!(root_options.color, ColorMode::Auto) && io::stdout().is_terminal());

        println!("{}/", root_name);
        for entry in &entries {
            if let Some(loc) = entry.loc {
                let line = format!("{:<width$}  {:>6}", entry.label, loc, width = max_label_len);
                if color_enabled && entry.is_large {
                    println!("{}{}{}", COLOR_RED, line, COLOR_RESET);
                } else {
                    println!("{}", line);
                }
            } else {
                println!("{}", entry.label);
            }
        }

        if !sorted_large.is_empty() {
            println!("\nLarge files (>= {} LOC):", LARGE_FILE_THRESHOLD);
            for item in &sorted_large {
                let summary_line = format!("  {} ({} LOC)", item.path, item.loc);
                if color_enabled {
                    println!("{}{}{}", COLOR_RED, summary_line, COLOR_RESET);
                } else {
                    println!("{}", summary_line);
                }
            }
        }

        if root_options.summary {
            println!(
                "\nSummary: directories: {}, files: {}, files with LOC: {}, total LOC: {}",
                stats.directories, stats.files, stats.files_with_loc, stats.total_loc
            );
            if sorted_large.is_empty() {
                println!("No files exceed the large-file threshold.");
            }
        }
    }

    if matches!(options.output, OutputMode::Json) {
        if json_results.len() == 1 {
            println!("{}", serde_json::to_string_pretty(&json_results[0]).unwrap());
        } else {
            println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
        }
    }

    Ok(())
}
