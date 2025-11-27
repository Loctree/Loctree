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
    if options.use_gitignore {
        if let Some(checker) = git_checker {
            if checker.is_ignored(full_path) {
                return true;
            }
        }
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
    use super::gather_files;
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
}
