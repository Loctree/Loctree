use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;
use std::io::IsTerminal;

use crate::fs_utils::{
    count_lines, is_allowed_hidden, normalise_ignore_patterns, should_ignore, sort_dir_entries,
    GitIgnoreChecker,
};
use crate::types::{
    Collectors, ColorMode, LargeEntry, LineEntry, Options, OutputMode, Stats, COLOR_RED,
    COLOR_RESET,
};

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn walk(
    dir: &Path,
    options: &Options,
    prefix_parts: &mut Vec<bool>,
    collectors: &mut Collectors,
    depth: usize,
    root: &Path,
    root_canon: &Path,
    git_checker: Option<&GitIgnoreChecker>,
    visited: &mut HashSet<PathBuf>,
) -> io::Result<bool> {
    let dir_canon = dir.canonicalize()?;
    if !dir_canon.starts_with(root_canon) {
        return Ok(false);
    }
    if !visited.insert(dir_canon.clone()) {
        return Ok(false);
    }

    // nosemgrep:rust.actix.path-traversal.tainted-path.tainted-path - dir path canonicalized and bounded to root_canon
    let mut dir_entries: Vec<_> = std::fs::read_dir(&dir_canon)?
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let is_hidden = name_str.starts_with('.');
            options.show_hidden || !is_hidden || is_allowed_hidden(&name_str)
        })
        .collect();

    sort_dir_entries(dir_entries.as_mut_slice());

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

        let relative = path
            .canonicalize()
            .unwrap_or_else(|_| path.clone())
            .strip_prefix(root_canon)
            .unwrap_or(&path)
            .to_path_buf();
        if should_ignore(&path, options, git_checker) {
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
                .is_none_or(|set| set.contains(&ext));
            if matches_ext {
                loc = count_lines(&path);
                if let Some(value) = loc {
                    collectors.stats.files += 1;
                    collectors.stats.files_with_loc += 1;
                    collectors.stats.total_loc += value;
                    if value >= options.loc_threshold {
                        let relative_display = if relative.as_os_str().is_empty() {
                            name.clone()
                        } else {
                            relative.to_string_lossy().to_string()
                        };
                        collectors.large_entries.push(LargeEntry {
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
        let is_large = loc.is_some_and(|v| v >= options.loc_threshold);

        if is_dir && options.max_depth.is_none_or(|max| depth < max) {
            prefix_parts.push(!is_last);
            let child_has = walk(
                &path,
                options,
                prefix_parts,
                collectors,
                depth + 1,
                root,
                root_canon,
                git_checker,
                visited,
            )?;
            prefix_parts.pop();
            if child_has {
                collectors.stats.directories += 1;
                include_current = true;
            }
        }

        if include_current {
            collectors.entries.push(LineEntry {
                label,
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

pub fn run_tree(root_list: &[PathBuf], parsed: &crate::args::ParsedArgs) -> io::Result<()> {
    let options = Options {
        extensions: parsed.extensions.clone(),
        ignore_paths: Vec::new(),
        use_gitignore: parsed.use_gitignore,
        max_depth: parsed.max_depth,
        color: parsed.color,
        output: parsed.output,
        summary: parsed.summary,
        summary_limit: parsed.summary_limit,
        show_hidden: parsed.show_hidden,
        loc_threshold: parsed.loc_threshold,
        analyze_limit: parsed.analyze_limit,
        report_path: None,
        serve: false,
        editor_cmd: None,
    };

    let mut json_results = Vec::new();

    for (idx, root_path) in root_list.iter().enumerate() {
        let ignore_paths = normalise_ignore_patterns(&parsed.ignore_patterns, root_path);
        let root_canon = root_path
            .canonicalize()
            .unwrap_or_else(|_| root_path.clone());
        let root_options = Options {
            ignore_paths,
            loc_threshold: parsed.loc_threshold,
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
        let mut visited: HashSet<PathBuf> = HashSet::new();

        let mut collectors = Collectors {
            entries: &mut entries,
            large_entries: &mut large_entries,
            stats: &mut stats,
        };

        walk(
            root_path,
            &root_options,
            &mut prefix_parts,
            &mut collectors,
            0,
            root_path,
            &root_canon,
            git_checker.as_ref(),
            &mut visited,
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

        if matches!(root_options.output, OutputMode::Json | OutputMode::Jsonl) {
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

            if matches!(root_options.output, OutputMode::Jsonl) {
                match serde_json::to_string(&payload) {
                    Ok(line) => println!("{}", line),
                    Err(err) => {
                        eprintln!("[loctree][warn] failed to serialize JSONL line: {}", err)
                    }
                }
            } else {
                json_results.push(payload);
            }
            continue;
        }

        if idx > 0 {
            println!();
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
            || (matches!(root_options.color, ColorMode::Auto) && std::io::stdout().is_terminal());

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
            println!("\nLarge files (>= {} LOC):", root_options.loc_threshold);
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
            match serde_json::to_string_pretty(&json_results[0]) {
                Ok(out) => println!("{}", out),
                Err(err) => eprintln!("[loctree][warn] failed to serialize JSON: {}", err),
            }
        } else {
            match serde_json::to_string_pretty(&json_results) {
                Ok(out) => println!("{}", out),
                Err(err) => eprintln!("[loctree][warn] failed to serialize JSON: {}", err),
            }
        }
    }

    Ok(())
}
