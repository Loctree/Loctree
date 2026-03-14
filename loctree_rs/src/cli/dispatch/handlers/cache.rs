//! Handler for `loct cache` commands (list, clean).

use std::fs;
use std::time::SystemTime;

use crate::cli::command::{CacheAction, CacheOptions};
use crate::snapshot::{cache_base_dir, project_cache_dir};

use super::super::DispatchResult;

pub fn handle_cache_command(opts: &CacheOptions) -> DispatchResult {
    match &opts.action {
        CacheAction::List => handle_list(),
        CacheAction::Clean {
            project,
            older_than,
            force,
        } => handle_clean(project.as_deref(), older_than.as_deref(), *force),
    }
}

fn handle_list() -> DispatchResult {
    let base = cache_base_dir();
    let projects_dir = base.join("projects");

    if !projects_dir.exists() {
        println!("No cached projects found.");
        println!("Cache dir: {}", base.display());
        return DispatchResult::Exit(0);
    }

    let entries = match fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Failed to read cache directory: {}", err);
            return DispatchResult::Exit(1);
        }
    };

    let mut total_size: u64 = 0;
    let mut rows: Vec<CacheEntry> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = entry.file_name().to_string_lossy().to_string();
        let size = dir_size(&path);
        let age = dir_age(&path);
        let project_path = read_project_root(&path);

        total_size += size;
        rows.push(CacheEntry {
            id,
            project_path,
            size,
            age,
        });
    }

    // Sort by size descending — biggest caches first
    rows.sort_by(|a, b| b.size.cmp(&a.size));

    if rows.is_empty() {
        println!("No cached projects found.");
        println!("Cache dir: {}", projects_dir.display());
        return DispatchResult::Exit(0);
    }

    println!("Cache: {}", projects_dir.display());
    println!();

    for entry in &rows {
        let name = entry.project_path.as_deref().unwrap_or("(unknown project)");
        println!(
            "  {} {:>10}  {}  {}",
            &entry.id[..8],
            format_size(entry.size),
            entry.age,
            name,
        );
    }

    println!();
    println!(
        "  {} project(s), {} total",
        rows.len(),
        format_size(total_size),
    );

    DispatchResult::Exit(0)
}

fn handle_clean(
    project: Option<&std::path::Path>,
    older_than: Option<&str>,
    force: bool,
) -> DispatchResult {
    let base = cache_base_dir();
    let projects_dir = base.join("projects");

    if !projects_dir.exists() {
        println!("Nothing to clean.");
        return DispatchResult::Exit(0);
    }

    // If --project specified, only clean that project's cache
    if let Some(proj) = project {
        let proj_path = if proj.is_relative() {
            std::env::current_dir().unwrap_or_default().join(proj)
        } else {
            proj.to_path_buf()
        };
        let cache_dir = project_cache_dir(&proj_path);
        if !cache_dir.exists() {
            println!("No cache found for project: {}", proj_path.display());
            return DispatchResult::Exit(0);
        }
        let size = dir_size(&cache_dir);
        if !force {
            eprintln!(
                "Will remove cache for {} ({}).",
                proj_path.display(),
                format_size(size)
            );
            eprintln!("Use --force to skip this confirmation.");
            return DispatchResult::Exit(1);
        }
        if let Err(err) = fs::remove_dir_all(&cache_dir) {
            eprintln!("Failed to remove {}: {}", cache_dir.display(), err);
            return DispatchResult::Exit(1);
        }
        println!(
            "Removed cache for {} ({})",
            proj_path.display(),
            format_size(size)
        );
        return DispatchResult::Exit(0);
    }

    // Parse --older-than duration
    let max_age_secs = older_than.and_then(parse_duration_days);

    let entries: Vec<_> = fs::read_dir(&projects_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();

    if entries.is_empty() {
        println!("Nothing to clean.");
        return DispatchResult::Exit(0);
    }

    let mut to_remove: Vec<(std::path::PathBuf, u64)> = Vec::new();

    for entry in &entries {
        let path = entry.path();

        if let Some(max_secs) = max_age_secs {
            let age_secs = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if age_secs < max_secs {
                continue; // Skip entries newer than threshold
            }
        }

        to_remove.push((path.clone(), dir_size(&path)));
    }

    if to_remove.is_empty() {
        println!("Nothing to clean (no entries match criteria).");
        return DispatchResult::Exit(0);
    }

    let total_size: u64 = to_remove.iter().map(|(_, s)| s).sum();

    if !force {
        eprintln!(
            "Will remove {} project(s) ({}).",
            to_remove.len(),
            format_size(total_size)
        );
        eprintln!("Use --force to skip this confirmation.");
        return DispatchResult::Exit(1);
    }

    let mut removed = 0;
    for (path, size) in &to_remove {
        if let Err(err) = fs::remove_dir_all(path) {
            eprintln!("Failed to remove {}: {}", path.display(), err);
        } else {
            removed += 1;
            if let Some(name) = path.file_name() {
                eprintln!(
                    "  removed {} ({})",
                    name.to_string_lossy(),
                    format_size(*size)
                );
            }
        }
    }

    println!(
        "Cleaned {} project(s), freed {}.",
        removed,
        format_size(total_size)
    );

    DispatchResult::Exit(0)
}

struct CacheEntry {
    id: String,
    project_path: Option<String>,
    size: u64,
    age: String,
}

/// Read the project root path from snapshot metadata in a cache directory.
fn read_project_root(cache_dir: &std::path::Path) -> Option<String> {
    // Look for snapshot.json and read metadata.roots[0]
    let snapshot_file = cache_dir.join("snapshot.json");
    if !snapshot_file.exists() {
        return None;
    }
    // Read just enough to extract roots — use a lightweight partial parse
    let content = fs::read_to_string(&snapshot_file).ok()?;
    // Parse just the metadata.roots field
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    val.get("metadata")?
        .get("roots")?
        .as_array()?
        .first()?
        .as_str()
        .map(|s| s.to_string())
}

/// Calculate total size of a directory recursively.
fn dir_size(path: &std::path::Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Get human-readable age of a directory (based on most recent modification).
fn dir_age(path: &std::path::Path) -> String {
    let newest = walkdir::WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        .max();

    match newest {
        Some(time) => {
            let elapsed = SystemTime::now().duration_since(time).unwrap_or_default();
            let secs = elapsed.as_secs();
            if secs < 60 {
                "just now".to_string()
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else if secs < 86400 {
                format!("{}h ago", secs / 3600)
            } else {
                format!("{}d ago", secs / 86400)
            }
        }
        None => "unknown".to_string(),
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Parse "7d" or "30d" into seconds.
fn parse_duration_days(s: &str) -> Option<u64> {
    let trimmed = s.trim().to_lowercase();
    if let Some(days_str) = trimmed.strip_suffix('d') {
        days_str.parse::<u64>().ok().map(|d| d * 86400)
    } else {
        // Try plain number as days
        trimmed.parse::<u64>().ok().map(|d| d * 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1536), "1.5KB");
        assert_eq!(format_size(1048576), "1.0MB");
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration_days("7d"), Some(7 * 86400));
        assert_eq!(parse_duration_days("30d"), Some(30 * 86400));
        assert_eq!(parse_duration_days("1d"), Some(86400));
        assert_eq!(parse_duration_days("30"), Some(30 * 86400));
        assert_eq!(parse_duration_days("abc"), None);
    }
}
