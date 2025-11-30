use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::types::Options;

pub struct GitIgnoreChecker {
    repo_root: PathBuf,
}

impl GitIgnoreChecker {
    pub fn new(root: &Path) -> Option<Self> {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .arg("rev-parse")
            .arg("--show-toplevel")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let repo_root = PathBuf::from(path_str);
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

/// Load patterns from `.loctreeignore` file in root directory.
/// Supports gitignore-style syntax: one pattern per line, # comments, empty lines ignored.
/// Returns empty vec if file doesn't exist.
pub fn load_loctreeignore(root: &Path) -> Vec<String> {
    let ignore_file = root.join(".loctreeignore");
    if !ignore_file.exists() {
        return Vec::new();
    }

    let file = match File::open(&ignore_file) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut patterns = Vec::new();

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

        // Treat each line as an ignore pattern
        patterns.push(trimmed.to_string());
    }

    patterns
}

pub fn normalise_ignore_patterns(patterns: &[String], root: &Path) -> Vec<PathBuf> {
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
            use_gitignore: false,
            max_depth: Some(1),
            color: ColorMode::Never,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 5,
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
            use_gitignore: false,
            max_depth: None,
            color: ColorMode::Never,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 5,
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
            use_gitignore: false,
            max_depth: Some(1),
            color: ColorMode::Never,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 5,
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
