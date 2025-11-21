use std::cmp::Ordering;
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

/// Represents a single entry in the tree with its label and optional line count.
struct LineEntry {
    label: String,
    loc: Option<usize>,
}

/// Sort directory entries so that directories appear before files and both are sorted
/// case-insensitively by name.
fn sort_entries(dir: &Path) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
    entries.sort_by(|a, b| {
        let a_path = a.path();
        let b_path = b.path();
        let a_is_dir = a_path.is_dir();
        let b_is_dir = b_path.is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => {
                let an = a
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase();
                let bn = b
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase();
                an.cmp(&bn)
            }
        }
    });
    Ok(entries)
}

/// Count the number of lines in a file. Returns `None` if the file can't be read.
fn count_lines(path: &Path) -> Option<usize> {
    let file = fs::File::open(path).ok()?;
    let reader = io::BufReader::new(file);
    let mut count = 0usize;
    for line in reader.lines() {
        if line.is_ok() {
            count += 1;
        }
    }
    Some(count)
}

/// Recursively walk the directory tree, collecting entries with their labels and line counts.
fn walk(
    dir: &Path,
    exts: &Option<Vec<String>>,
    prefix_parts: &mut Vec<bool>,
    out: &mut Vec<LineEntry>,
) -> io::Result<()> {
    let entries = sort_entries(dir)?;
    let len = entries.len();
    for (idx, entry) in entries.into_iter().enumerate() {
        let path: PathBuf = entry.path();
        let is_last = idx + 1 == len;
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();
        // Skip macOS metadata files
        if name == ".DS_Store" {
            continue;
        }
        // Build prefix showing tree structure
        let mut prefix = String::new();
        for &has_more in prefix_parts.iter() {
            if has_more {
                prefix.push_str("│   ");
            } else {
                prefix.push_str("    ");
            }
        }
        let branch = if is_last { "└── " } else { "├── " };
        let label = format!("{}{}{}", prefix, branch, name);
        // Determine whether to count lines
        let mut loc = None;
        if path.is_file() {
            let mut ok_ext = true;
            if let Some(ext_list) = exts {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    ok_ext = ext_list.iter().any(|e| e == ext);
                } else {
                    ok_ext = false;
                }
            }
            if ok_ext {
                loc = count_lines(&path);
            }
        }
        out.push(LineEntry { label: label.clone(), loc });
        // Recurse into directories
        if path.is_dir() {
            prefix_parts.push(!is_last);
            walk(&path, exts, prefix_parts, out)?;
            prefix_parts.pop();
        }
    }
    Ok(())
}

/// Parse command-line arguments for the root directory and optional extension filter.
fn parse_args() -> (PathBuf, Option<Vec<String>>) {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: loc-tree <root> [--ext rs,ts,tsx]");
        std::process::exit(1);
    }
    let root = PathBuf::from(args[0].clone());
    let mut exts: Option<Vec<String>> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--ext" && i + 1 < args.len() {
            let list = args[i + 1]
                .split(',')
                .map(|s| s.trim().trim_start_matches('.').to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if !list.is_empty() {
                exts = Some(list);
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    (root, exts)
}

/// Main entry point: parse args, walk directory, and print results.
fn main() -> io::Result<()> {
    let (root, exts) = parse_args();
    if !root.is_dir() {
        eprintln!("{} is not a directory", root.display());
        std::process::exit(1);
    }
    let mut entries: Vec<LineEntry> = Vec::new();
    let mut prefix_parts: Vec<bool> = Vec::new();
    walk(&root, &exts, &mut prefix_parts, &mut entries)?;
    if entries.is_empty() {
        return Ok(());
    }
    let max_label_len = entries.iter().map(|e| e.label.len()).max().unwrap_or(0);
    let root_name = root
        .file_name()
        .unwrap_or_else(|| root.as_os_str())
        .to_string_lossy();
    println!("{}/", root_name);
    for e in entries {
        if let Some(loc) = e.loc {
            let padding = " ".repeat(max_label_len - e.label.len() + 2);
            println!("{}{}{:>6}", e.label, padding, loc);
        } else {
            println!("{}", e.label);
        }
    }
    Ok(())
}