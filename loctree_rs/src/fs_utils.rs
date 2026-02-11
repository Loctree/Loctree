use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::types::Options;

pub struct GitIgnoreChecker {
    repo_root: PathBuf,
}

impl GitIgnoreChecker {
    /// Create a new GitIgnoreChecker for the given path.
    ///
    /// Uses libgit2's repository discovery which properly searches upward
    /// from the given path to find the git repository root. This handles:
    /// - Nested directories (e.g., running from src/deep/nested/)
    /// - Git worktrees (where .git is a file pointing to the main repo)
    /// - Submodules
    ///
    /// Returns `None` if the path is not inside a git repository.
    pub fn new(root: &Path) -> Option<Self> {
        // Use libgit2 to find git root (searches upward properly)
        let repo_root = crate::git::find_git_root(root)?;
        Some(Self { repo_root })
    }

    pub fn is_ignored(&self, full_path: &Path) -> bool {
        if full_path.as_os_str().is_empty() {
            return false;
        }
        let relative = full_path.strip_prefix(&self.repo_root).unwrap_or(full_path);
        Command::new("git")
            .arg("-C")
            .arg(&self.repo_root)
            .arg("check-ignore")
            .arg("-q")
            .arg(relative)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

#[derive(Debug, Default, Clone)]
pub struct LoctignoreRules {
    /// Ignore patterns for file scanning.
    pub ignore_patterns: Vec<String>,
    /// Glob patterns for suppressing dead-export findings.
    ///
    /// Lines in `.loctignore`:
    /// - `@loctignore:dead-ok <glob>`
    pub dead_ok_globs: Vec<String>,
}

fn parse_loctignore_directive(line: &str) -> Option<(&str, &str)> {
    // Syntax: "@loctignore:<directive> <arg...>"
    let rest = line.strip_prefix("@loctignore:")?.trim_start();
    if rest.is_empty() {
        return None;
    }
    // Split once on whitespace (directive + remainder)
    let mut split_at: Option<usize> = None;
    for (idx, ch) in rest.char_indices() {
        if ch.is_whitespace() {
            split_at = Some(idx);
            break;
        }
    }
    match split_at {
        Some(idx) => Some((&rest[..idx], rest[idx..].trim())),
        None => Some((rest, "")),
    }
}

pub fn load_loctignore_rules(root: &Path) -> LoctignoreRules {
    // Prefer .loctignore (short form, matches `loct` CLI)
    let ignore_file = root.join(".loctignore");
    let ignore_file = if ignore_file.exists() {
        ignore_file
    } else {
        // Fallback to .loctreeignore for backward compatibility
        let legacy = root.join(".loctreeignore");
        if !legacy.exists() {
            return LoctignoreRules::default();
        }
        legacy
    };

    let file = match File::open(&ignore_file) {
        Ok(f) => f,
        Err(_) => return LoctignoreRules::default(),
    };

    let reader = BufReader::new(file);
    let mut rules = LoctignoreRules::default();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("@loctignore:") {
            if let Some((directive, arg)) = parse_loctignore_directive(trimmed)
                && directive == "dead-ok"
                && !arg.is_empty()
            {
                rules.dead_ok_globs.push(arg.to_string());
            }
            continue;
        }

        // Treat each non-directive line as an ignore pattern
        rules.ignore_patterns.push(trimmed.to_string());
    }

    rules
}

/// Load ignore patterns from `.loctignore` (preferred) or `.loctreeignore` (legacy).
///
/// Notes:
/// - Supports `#` comments and empty lines.
/// - Skips `@loctignore:*` directives (handled separately by `load_loctignore_rules`).
/// - Returns empty vec if file doesn't exist.
pub fn load_loctreeignore(root: &Path) -> Vec<String> {
    load_loctignore_rules(root).ignore_patterns
}

pub fn load_loctignore_dead_ok_globs(root: &Path) -> Vec<String> {
    load_loctignore_rules(root).dead_ok_globs
}

fn is_glob_pattern(pattern: &str) -> bool {
    // Minimal, pragmatic detection: if it looks like a glob, treat it as one.
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[')
}

#[derive(Debug, Default, Clone)]
pub struct IgnoreMatchers {
    pub ignore_paths: Vec<PathBuf>,
    pub ignore_globs: Option<Arc<globset::GlobSet>>,
}

pub fn build_ignore_matchers(patterns: &[String], root: &Path) -> IgnoreMatchers {
    let mut ignore_paths: Vec<PathBuf> = Vec::new();
    let mut builder = globset::GlobSetBuilder::new();
    let mut any_globs = false;

    for pattern in patterns {
        let trimmed = pattern.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("@loctignore:") {
            continue;
        }

        if is_glob_pattern(trimmed) {
            // For relative patterns, anchor at scan root (absolute match against `full_path`).
            let mut add_glob = |glob_pat: &str| {
                let candidate = if Path::new(glob_pat).is_absolute() {
                    PathBuf::from(glob_pat)
                } else {
                    root.join(glob_pat)
                };
                let Some(mut glob_str) = candidate.to_str().map(|s| s.replace('\\', "/")) else {
                    return;
                };
                // Normalize accidental "./" segments for nicer patterns.
                if glob_str.contains("/./") {
                    glob_str = glob_str.replace("/./", "/");
                }
                match globset::Glob::new(&glob_str) {
                    Ok(glob) => {
                        builder.add(glob);
                        any_globs = true;
                    }
                    Err(e) => {
                        eprintln!("[loctree][warn] invalid ignore glob '{}': {}", glob_pat, e);
                    }
                }
            };

            // A trailing slash means "directory" in gitignore-ish conventions.
            // We add both the directory itself and its contents.
            if let Some(base) = trimmed.strip_suffix('/') {
                if !base.is_empty() {
                    add_glob(base);
                    add_glob(&format!("{}/**", base));
                }
            } else {
                add_glob(trimmed);
            }
            continue;
        }

        // Literal path prefix ignore (fast)
        let candidate = PathBuf::from(trimmed);
        let full = if candidate.is_absolute() {
            candidate
        } else {
            root.join(candidate)
        };
        ignore_paths.push(full.canonicalize().unwrap_or(full));
    }

    let ignore_globs = if any_globs {
        match builder.build() {
            Ok(set) => Some(Arc::new(set)),
            Err(e) => {
                eprintln!("[loctree][warn] failed to build ignore globset: {}", e);
                None
            }
        }
    } else {
        None
    };

    IgnoreMatchers {
        ignore_paths,
        ignore_globs,
    }
}

pub fn normalise_ignore_patterns(patterns: &[String], root: &Path) -> Vec<PathBuf> {
    patterns
        .iter()
        .filter(|pattern| {
            let trimmed = pattern.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("@loctignore:")
                && !is_glob_pattern(trimmed)
        })
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

pub fn count_lines(path: &Path) -> Option<usize> {
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

pub fn matches_extension(
    path: &Path,
    extensions: Option<&std::collections::HashSet<String>>,
) -> bool {
    match extensions {
        None => true,
        Some(set) => path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| set.contains(&ext.to_lowercase()))
            .unwrap_or(false),
    }
}

pub fn is_allowed_hidden(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == ".env"
        || lower.starts_with(".env.")
        || lower.starts_with(".loctree.")
        || lower == ".example"
}

pub fn should_ignore(
    full_path: &Path,
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
    if let Some(globs) = &options.ignore_globs
        && globs.is_match(full_path)
    {
        return true;
    }
    if options.use_gitignore
        && let Some(checker) = git_checker
        && checker.is_ignored(full_path)
    {
        return true;
    }
    false
}

pub fn gather_files(
    dir: &Path,
    options: &Options,
    depth: usize,
    git_checker: Option<&GitIgnoreChecker>,
    visited: &mut HashSet<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let dir_canon = dir.canonicalize()?;
    if !visited.insert(dir_canon.clone()) {
        return Ok(());
    }

    let mut dir_entries: Vec<_> = fs::read_dir(&dir_canon)?
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip common heavy directories unless --scan-all is set
            if !options.scan_all
                && (name_str == "node_modules"
                    || name_str == ".git"
                    || name_str == "target"
                    || name_str == ".venv"
                    || name_str == "venv"
                    || name_str == "__pycache__")
            {
                return false;
            }

            let is_hidden = name_str.starts_with('.');
            options.show_hidden || !is_hidden || is_allowed_hidden(&name_str)
        })
        .collect();

    dir_entries.sort_by(|a, b| {
        a.file_name()
            .to_string_lossy()
            .to_lowercase()
            .cmp(&b.file_name().to_string_lossy().to_lowercase())
    });

    for entry in dir_entries {
        let path = entry.path();
        if should_ignore(&path, options, git_checker) {
            continue;
        }

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if file_type.is_symlink() {
            let target = match fs::canonicalize(&path) {
                Ok(p) => p,
                Err(_) => continue, // broken symlink
            };
            if visited.contains(&target) {
                continue;
            }
            let meta = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() && options.max_depth.is_none_or(|max| depth < max) {
                gather_files(&target, options, depth + 1, git_checker, visited, files)?;
            } else if meta.is_file() && matches_extension(&target, options.extensions.as_ref()) {
                files.push(target);
            }
            continue;
        }

        if path.is_file() {
            let canonical = path.canonicalize().unwrap_or(path.clone());
            if matches_extension(&canonical, options.extensions.as_ref()) {
                files.push(canonical);
            }
            continue;
        }
        if path.is_dir() && options.max_depth.is_none_or(|max| depth < max) {
            gather_files(&path, options, depth + 1, git_checker, visited, files)?;
        }
    }

    Ok(())
}

pub fn sort_dir_entries(entries: &mut [std::fs::DirEntry]) {
    entries.sort_by(|a, b| {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ColorMode, Options, OutputMode};
    use std::collections::HashSet;
    use std::path::PathBuf;

    fn opts_with_ext(ext: &str) -> Options {
        Options {
            extensions: Some(HashSet::from([ext.to_string()])),
            ignore_paths: Vec::new(),
            ignore_globs: None,
            use_gitignore: false,
            max_depth: Some(1),
            color: ColorMode::Never,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 5,
            summary_only: false,
            show_hidden: false,
            show_ignored: false,
            loc_threshold: crate::types::DEFAULT_LOC_THRESHOLD,
            analyze_limit: 8,
            report_path: None,
            serve: false,
            editor_cmd: None,
            max_graph_nodes: None,
            max_graph_edges: None,
            verbose: false,
            scan_all: false,
            symbol: None,
            impact: None,
            find_artifacts: false,
        }
    }

    fn default_opts() -> Options {
        Options {
            extensions: None,
            ignore_paths: Vec::new(),
            ignore_globs: None,
            use_gitignore: false,
            max_depth: None,
            color: ColorMode::Never,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 5,
            summary_only: false,
            show_hidden: false,
            show_ignored: false,
            loc_threshold: crate::types::DEFAULT_LOC_THRESHOLD,
            analyze_limit: 8,
            report_path: None,
            serve: false,
            editor_cmd: None,
            max_graph_nodes: None,
            max_graph_edges: None,
            verbose: false,
            scan_all: false,
            symbol: None,
            impact: None,
            find_artifacts: false,
        }
    }

    #[test]
    fn gather_files_filters_by_extension_and_depth() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let root = tmp.path();
        std::fs::create_dir_all(root.join("nested")).expect("tmp nested dir");
        std::fs::write(root.join("keep.rs"), "// ok").expect("write keep.rs");
        std::fs::write(root.join("skip.txt"), "// skip").expect("write skip.txt");
        std::fs::write(root.join(".hidden.rs"), "// hidden").expect("write hidden");
        std::fs::write(root.join("nested").join("deep.rs"), "// deep").expect("write deep.rs");

        let mut files = Vec::new();
        let opts = opts_with_ext("rs");
        let mut visited = HashSet::new();
        gather_files(root, &opts, 0, None, &mut visited, &mut files).expect("gather files");

        let as_strings: Vec<String> = files
            .iter()
            .map(|p| {
                p.file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert!(as_strings.contains(&"keep.rs".to_string()));
        assert!(!as_strings.contains(&"skip.txt".to_string()));
        assert!(as_strings.contains(&"deep.rs".to_string()));
        assert!(!as_strings.contains(&".hidden.rs".to_string()));
    }

    #[test]
    fn allows_whitelisted_hidden_files() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let root = tmp.path();
        std::fs::write(root.join(".env.local"), "KEY=1").expect("env local");
        std::fs::write(root.join(".loctree.json"), "{}").expect("loctree json");
        std::fs::write(root.join(".example"), "// example").expect("example");
        std::fs::write(root.join(".ignored"), "// ignore").expect("ignored");

        let mut files = Vec::new();
        let opts = Options {
            extensions: None,
            ignore_paths: Vec::new(),
            ignore_globs: None,
            use_gitignore: false,
            max_depth: Some(1),
            color: ColorMode::Never,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 5,
            summary_only: false,
            show_hidden: false,
            show_ignored: false,
            loc_threshold: crate::types::DEFAULT_LOC_THRESHOLD,
            analyze_limit: 8,
            report_path: None,
            serve: false,
            editor_cmd: None,
            max_graph_nodes: None,
            max_graph_edges: None,
            verbose: false,
            scan_all: false,
            symbol: None,
            impact: None,
            find_artifacts: false,
        };
        let mut visited = HashSet::new();
        gather_files(root, &opts, 0, None, &mut visited, &mut files).expect("gather files");
        let names: HashSet<PathBuf> = files
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.into()))
            .collect();
        assert!(names.contains(&PathBuf::from(".env.local")));
        assert!(names.contains(&PathBuf::from(".loctree.json")));
        assert!(names.contains(&PathBuf::from(".example")));
        assert!(!names.contains(&PathBuf::from(".ignored")));
    }

    #[test]
    #[cfg(unix)]
    fn avoids_symlink_loops() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().expect("tmp dir");
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        std::fs::create_dir_all(&a).expect("mkdir a");
        std::fs::create_dir_all(&b).expect("mkdir b");
        std::fs::write(a.join("keep.rs"), "// ok").expect("write keep");
        symlink(&b, a.join("loop_to_b")).expect("symlink b");
        symlink(&a, b.join("loop_to_a")).expect("symlink a");

        let mut files = Vec::new();
        let opts = opts_with_ext("rs");
        let mut visited = HashSet::new();
        gather_files(root, &opts, 0, None, &mut visited, &mut files).expect("gather files");
        let names: Vec<String> = files
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        assert_eq!(names, vec!["keep.rs".to_string()]);
    }

    #[test]
    fn test_count_lines() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2\nline3\n").expect("write file");

        let count = count_lines(&file_path);
        assert_eq!(count, Some(3));
    }

    #[test]
    fn test_count_lines_empty_file() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let file_path = tmp.path().join("empty.txt");
        std::fs::write(&file_path, "").expect("write file");

        let count = count_lines(&file_path);
        assert_eq!(count, Some(0));
    }

    #[test]
    fn test_count_lines_missing_file() {
        let count = count_lines(Path::new("/nonexistent/file.txt"));
        assert!(count.is_none());
    }

    #[test]
    fn test_matches_extension_with_set() {
        let extensions: HashSet<String> = ["rs", "ts", "js"]
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        assert!(matches_extension(Path::new("file.rs"), Some(&extensions)));
        assert!(matches_extension(Path::new("file.ts"), Some(&extensions)));
        assert!(matches_extension(Path::new("file.RS"), Some(&extensions))); // case insensitive
        assert!(!matches_extension(Path::new("file.py"), Some(&extensions)));
        assert!(!matches_extension(Path::new("noext"), Some(&extensions)));
    }

    #[test]
    fn test_matches_extension_none() {
        // None means no filter - all files match
        assert!(matches_extension(Path::new("file.rs"), None));
        assert!(matches_extension(Path::new("file.txt"), None));
        assert!(matches_extension(Path::new("noext"), None));
    }

    #[test]
    fn test_is_allowed_hidden() {
        // Allowed hidden files
        assert!(is_allowed_hidden(".env"));
        assert!(is_allowed_hidden(".ENV")); // case insensitive
        assert!(is_allowed_hidden(".env.local"));
        assert!(is_allowed_hidden(".env.production"));
        assert!(is_allowed_hidden(".loctree.json"));
        assert!(is_allowed_hidden(".loctree.yml"));
        assert!(is_allowed_hidden(".example"));

        // Not allowed
        assert!(!is_allowed_hidden(".gitignore"));
        assert!(!is_allowed_hidden(".npmrc"));
        assert!(!is_allowed_hidden(".hidden"));
    }

    #[test]
    fn test_should_ignore_with_ignore_paths() {
        let opts = Options {
            ignore_paths: vec![PathBuf::from("/ignored/path")],
            ..default_opts()
        };

        assert!(should_ignore(
            Path::new("/ignored/path/file.rs"),
            &opts,
            None
        ));
        assert!(!should_ignore(
            Path::new("/other/path/file.rs"),
            &opts,
            None
        ));
    }

    #[test]
    fn test_load_loctreeignore_nonexistent() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let patterns = load_loctreeignore(tmp.path());
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_load_loctreeignore_with_patterns() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let ignore_file = tmp.path().join(".loctreeignore");
        std::fs::write(
            &ignore_file,
            "# Comment\nnode_modules\n\n*.log\n# Another comment\nbuild/\n",
        )
        .expect("write loctreeignore");

        let patterns = load_loctreeignore(tmp.path());
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"node_modules".to_string()));
        assert!(patterns.contains(&"*.log".to_string()));
        assert!(patterns.contains(&"build/".to_string()));
    }

    #[test]
    fn test_load_loctignore_directives() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let ignore_file = tmp.path().join(".loctignore");
        std::fs::write(
            &ignore_file,
            "# Comment\nfixtures/\n@loctignore:dead-ok src/generated/**\n",
        )
        .expect("write loctignore");

        let patterns = load_loctreeignore(tmp.path());
        assert_eq!(patterns, vec!["fixtures/".to_string()]);

        let dead_ok = load_loctignore_dead_ok_globs(tmp.path());
        assert_eq!(dead_ok, vec!["src/generated/**".to_string()]);
    }

    #[test]
    fn test_should_ignore_with_ignore_globs() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let patterns = vec!["**/*.log".to_string()];
        let matchers = build_ignore_matchers(&patterns, tmp.path());
        let opts = Options {
            ignore_paths: matchers.ignore_paths,
            ignore_globs: matchers.ignore_globs,
            ..default_opts()
        };

        assert!(should_ignore(&tmp.path().join("app.log"), &opts, None));
        assert!(!should_ignore(&tmp.path().join("app.txt"), &opts, None));
    }

    #[test]
    fn test_normalise_ignore_patterns_relative() {
        let tmp = tempfile::tempdir().expect("tmp dir");
        let patterns = vec!["src".to_string(), "lib".to_string()];

        let normalized = normalise_ignore_patterns(&patterns, tmp.path());
        assert_eq!(normalized.len(), 2);
        // Normalized paths should be based on root
        assert!(normalized[0].ends_with("src") || normalized[0].to_string_lossy().contains("src"));
    }

    #[test]
    fn test_sort_dir_entries() {
        let tmp = tempfile::tempdir().expect("tmp dir");

        // Create some files and directories
        std::fs::create_dir(tmp.path().join("z_dir")).expect("mkdir");
        std::fs::create_dir(tmp.path().join("a_dir")).expect("mkdir");
        std::fs::write(tmp.path().join("z_file.txt"), "").expect("write");
        std::fs::write(tmp.path().join("a_file.txt"), "").expect("write");

        let mut entries: Vec<_> = std::fs::read_dir(tmp.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .collect();

        sort_dir_entries(&mut entries);

        // After sorting: directories first (a_dir, z_dir), then files (a_file, z_file)
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        // First two should be directories
        assert!(entries[0].path().is_dir());
        assert!(entries[1].path().is_dir());
        // Directories alphabetically
        assert_eq!(names[0], "a_dir");
        assert_eq!(names[1], "z_dir");
        // Files alphabetically
        assert_eq!(names[2], "a_file.txt");
        assert_eq!(names[3], "z_file.txt");
    }
}
