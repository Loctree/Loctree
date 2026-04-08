//! Handler for `loct cache` commands (list, clean).

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::cli::command::{CacheAction, CacheOptions};
use crate::snapshot::{SnapshotMetadata, cache_base_dir, project_cache_dir};
use serde::Deserialize;
use time::{OffsetDateTime, format_description::well_known::Iso8601};

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
    let mut rows: Vec<CacheBucketRow> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let bucket_id = entry.file_name().to_string_lossy().to_string();
        let row = collect_cache_bucket_row(&bucket_id, &path);
        total_size += row.size_bytes;
        rows.push(row);
    }

    rows.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.org_repo.cmp(&b.org_repo))
            .then_with(|| a.project_path.cmp(&b.project_path))
    });

    if rows.is_empty() {
        println!("No cached projects found.");
        println!("Cache dir: {}", projects_dir.display());
        return DispatchResult::Exit(0);
    }

    println!("Cache: {}", projects_dir.display());
    println!();
    println!("Org/Repo | Path | Cache size MB | Meta");
    println!("--- | --- | --- | ---");

    for row in &rows {
        println!(
            "{} | {} | {:.2} | {}",
            row.org_repo,
            row.project_path,
            size_in_mb(row.size_bytes),
            row.meta,
        );
    }

    println!();
    println!(
        "{} cache bucket(s), {:.2} MB total",
        rows.len(),
        size_in_mb(total_size),
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

#[derive(Debug, PartialEq, Eq)]
struct CacheBucketRow {
    org_repo: String,
    project_path: String,
    size_bytes: u64,
    meta: String,
}

#[derive(Clone, Debug)]
struct CacheSnapshotRecord {
    metadata: SnapshotMetadata,
    modified_at: SystemTime,
    is_latest_pointer: bool,
}

#[derive(Debug, Default)]
struct CacheBucketStats {
    size_bytes: u64,
    snapshots: Vec<CacheSnapshotRecord>,
}

#[derive(Debug, Default, Deserialize)]
struct SnapshotMetadataEnvelope {
    #[serde(default)]
    metadata: SnapshotMetadata,
}

fn collect_cache_bucket_row(bucket_id: &str, bucket_dir: &Path) -> CacheBucketRow {
    let stats = collect_cache_bucket_stats(bucket_dir);
    let snapshots = effective_bucket_snapshots(&stats.snapshots);
    let project_path =
        select_project_path(&snapshots).unwrap_or_else(|| "(unknown path)".to_string());
    let org_repo = resolve_org_repo_label(&snapshots, bucket_id, &project_path);
    let meta = format_cache_meta(&snapshots);

    CacheBucketRow {
        org_repo,
        project_path,
        size_bytes: stats.size_bytes,
        meta,
    }
}

fn collect_cache_bucket_stats(bucket_dir: &Path) -> CacheBucketStats {
    let mut size_bytes = 0;
    let mut snapshots = Vec::new();

    for entry in walkdir::WalkDir::new(bucket_dir).into_iter().flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if !metadata.is_file() {
            continue;
        }

        size_bytes += metadata.len();

        if entry.file_name().to_str() != Some("snapshot.json") {
            continue;
        }

        let modified_at = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if let Some(snapshot) = read_snapshot_record(entry.path(), bucket_dir, modified_at) {
            snapshots.push(snapshot);
        }
    }

    CacheBucketStats {
        size_bytes,
        snapshots,
    }
}

fn read_snapshot_record(
    snapshot_path: &Path,
    bucket_dir: &Path,
    modified_at: SystemTime,
) -> Option<CacheSnapshotRecord> {
    let bytes = fs::read(snapshot_path).ok()?;
    let envelope: SnapshotMetadataEnvelope = serde_json::from_slice(&bytes).ok()?;
    let is_latest_pointer = snapshot_path
        .parent()
        .and_then(Path::file_name)
        .and_then(|segment| segment.to_str())
        == Some("latest")
        && snapshot_path
            .parent()
            .and_then(Path::parent)
            .is_some_and(|parent| parent == bucket_dir);

    Some(CacheSnapshotRecord {
        metadata: envelope.metadata,
        modified_at,
        is_latest_pointer,
    })
}

fn effective_bucket_snapshots(snapshots: &[CacheSnapshotRecord]) -> Vec<&CacheSnapshotRecord> {
    let actual: Vec<_> = snapshots
        .iter()
        .filter(|snapshot| !snapshot.is_latest_pointer)
        .collect();
    if actual.is_empty() {
        snapshots.iter().collect()
    } else {
        actual
    }
}

fn select_project_path(snapshots: &[&CacheSnapshotRecord]) -> Option<String> {
    snapshots
        .iter()
        .flat_map(|snapshot| snapshot.metadata.roots.iter())
        .map(|root| root.trim())
        .filter(|root| !root.is_empty())
        .map(str::to_string)
        .min_by(compare_root_display)
}

fn compare_root_display(left: &String, right: &String) -> Ordering {
    path_depth(left)
        .cmp(&path_depth(right))
        .then_with(|| left.len().cmp(&right.len()))
        .then_with(|| left.cmp(right))
}

fn path_depth(path: &str) -> usize {
    Path::new(path).components().count()
}

fn resolve_org_repo_label(
    snapshots: &[&CacheSnapshotRecord],
    bucket_id: &str,
    project_path: &str,
) -> String {
    snapshots
        .iter()
        .filter_map(|snapshot| option_str(&snapshot.metadata.git_owner_repo))
        .max_by(|left, right| compare_option_str(left, right))
        .map(str::to_string)
        .or_else(|| {
            snapshots
                .iter()
                .filter_map(|snapshot| option_str(&snapshot.metadata.git_repo))
                .max_by(|left, right| compare_option_str(left, right))
                .map(|repo| format!("unknown/{repo}"))
        })
        .or_else(|| fallback_local_org_repo(project_path))
        .unwrap_or_else(|| format!("unknown/{bucket_id}"))
}

fn fallback_local_org_repo(project_path: &str) -> Option<String> {
    if project_path == "(unknown path)" {
        return None;
    }

    let repo_name = Path::new(project_path)
        .file_name()
        .and_then(|segment| segment.to_str())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())?;

    Some(format!("local/{repo_name}"))
}

fn format_cache_meta(snapshots: &[&CacheSnapshotRecord]) -> String {
    if snapshots.is_empty() {
        return "scans 0; latest unknown; schema unknown".to_string();
    }

    let root_count = distinct_non_empty_values(
        snapshots
            .iter()
            .flat_map(|snapshot| snapshot.metadata.roots.iter())
            .map(|root| root.as_str()),
    )
    .len();
    let branch_count = distinct_non_empty_values(
        snapshots
            .iter()
            .filter_map(|snapshot| option_str(&snapshot.metadata.git_branch)),
    )
    .len();
    let schemas = distinct_non_empty_values(
        snapshots
            .iter()
            .filter_map(|snapshot| non_empty_str(snapshot.metadata.schema_version.as_str())),
    );
    let latest = snapshots
        .iter()
        .copied()
        .max_by(|a, b| compare_snapshot_records(a, b))
        .expect("snapshots is non-empty");

    let mut parts = vec![format!("scans {}", snapshots.len())];
    if root_count > 1 {
        parts.push(format!("roots {root_count}"));
    }
    if branch_count > 1 {
        parts.push(format!("branches {branch_count}"));
    }
    parts.push(format!("latest {}", latest_timestamp(latest)));
    if let Some(reference) = format_git_reference(latest) {
        parts.push(format!("ref {reference}"));
    }
    parts.push(format_schema_meta(&schemas, latest));

    parts.join("; ")
}

fn distinct_non_empty_values<'a>(values: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    values
        .into_iter()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect()
}

fn compare_snapshot_records(left: &CacheSnapshotRecord, right: &CacheSnapshotRecord) -> Ordering {
    left.modified_at
        .cmp(&right.modified_at)
        .then_with(|| {
            non_empty_str(left.metadata.generated_at.as_str())
                .cmp(&non_empty_str(right.metadata.generated_at.as_str()))
        })
        .then_with(|| {
            option_str(&left.metadata.git_scan_id).cmp(&option_str(&right.metadata.git_scan_id))
        })
        .then_with(|| {
            select_first_root(left.metadata.roots.as_slice())
                .cmp(&select_first_root(right.metadata.roots.as_slice()))
        })
}

fn select_first_root(roots: &[String]) -> Option<&str> {
    roots
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .find(|root| !root.is_empty())
}

fn latest_timestamp(snapshot: &CacheSnapshotRecord) -> String {
    non_empty_str(snapshot.metadata.generated_at.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format_system_time(snapshot.modified_at))
}

fn format_system_time(timestamp: SystemTime) -> String {
    OffsetDateTime::from(timestamp)
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".to_string())
}

fn format_git_reference(snapshot: &CacheSnapshotRecord) -> Option<String> {
    match (
        option_str(&snapshot.metadata.git_branch),
        option_str(&snapshot.metadata.git_commit),
    ) {
        (Some(branch), Some(commit)) => Some(format!("{branch}@{commit}")),
        (Some(branch), None) => Some(branch.to_string()),
        (None, Some(commit)) => Some(commit.to_string()),
        (None, None) => None,
    }
}

fn format_schema_meta(schemas: &BTreeSet<&str>, latest_snapshot: &CacheSnapshotRecord) -> String {
    match schemas.len() {
        0 => "schema unknown".to_string(),
        1 => format!("schema {}", schemas.iter().next().expect("single schema")),
        count => {
            let latest_schema = non_empty_str(latest_snapshot.metadata.schema_version.as_str())
                .unwrap_or("unknown");
            format!("schema {latest_schema} (+{} more)", count - 1)
        }
    }
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn option_str(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn compare_option_str(left: &str, right: &str) -> Ordering {
    path_depth(left)
        .cmp(&path_depth(right))
        .then_with(|| left.len().cmp(&right.len()))
        .then_with(|| left.cmp(right))
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

fn size_in_mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
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
    use tempfile::TempDir;

    #[allow(clippy::too_many_arguments)]
    fn snapshot_record(
        roots: &[&str],
        generated_at: Option<&str>,
        owner_repo: Option<&str>,
        repo: Option<&str>,
        branch: Option<&str>,
        commit: Option<&str>,
        schema: Option<&str>,
        modified_at: SystemTime,
        is_latest_pointer: bool,
    ) -> CacheSnapshotRecord {
        CacheSnapshotRecord {
            metadata: SnapshotMetadata {
                schema_version: schema.unwrap_or_default().to_string(),
                generated_at: generated_at.unwrap_or_default().to_string(),
                roots: roots.iter().map(|root| root.to_string()).collect(),
                git_repo: repo.map(str::to_string),
                git_owner_repo: owner_repo.map(str::to_string),
                git_branch: branch.map(str::to_string),
                git_commit: commit.map(str::to_string),
                git_scan_id: None,
                ..SnapshotMetadata::default()
            },
            modified_at,
            is_latest_pointer,
        }
    }

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

    #[test]
    fn test_select_project_path_prefers_shortest_root() {
        let now = SystemTime::UNIX_EPOCH;
        let primary = snapshot_record(
            &["/tmp/demo"],
            Some("2026-03-31T16:18:00Z"),
            Some("VetCoders/demo"),
            Some("demo"),
            Some("main"),
            Some("abc123"),
            Some("0.9.0"),
            now,
            false,
        );
        let nested = snapshot_record(
            &["/tmp/demo/src"],
            Some("2026-03-31T16:19:00Z"),
            Some("VetCoders/demo"),
            Some("demo"),
            Some("feature"),
            Some("def456"),
            Some("0.9.0"),
            now,
            false,
        );

        let snapshots = vec![&primary, &nested];
        assert_eq!(
            select_project_path(&snapshots),
            Some("/tmp/demo".to_string())
        );
    }

    #[test]
    fn test_resolve_org_repo_label_uses_local_fallback_for_non_git_bucket() {
        let snapshot = snapshot_record(
            &["/tmp/local-project"],
            Some("2026-03-31T16:18:00Z"),
            None,
            None,
            None,
            None,
            Some("0.9.0"),
            SystemTime::UNIX_EPOCH,
            false,
        );
        let snapshots = vec![&snapshot];

        assert_eq!(
            resolve_org_repo_label(&snapshots, "abc123deadbeef00", "/tmp/local-project"),
            "local/local-project"
        );
    }

    #[test]
    fn test_format_cache_meta_skips_latest_pointer_duplicates() {
        let older = snapshot_record(
            &["/tmp/demo"],
            Some("2026-03-30T12:00:00Z"),
            Some("VetCoders/demo"),
            Some("demo"),
            Some("main"),
            Some("aaa111"),
            Some("0.9.0"),
            SystemTime::UNIX_EPOCH,
            false,
        );
        let newer = snapshot_record(
            &["/tmp/demo"],
            Some("2026-03-31T12:00:00Z"),
            Some("VetCoders/demo"),
            Some("demo"),
            Some("feature"),
            Some("bbb222"),
            Some("0.9.0"),
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10),
            false,
        );
        let latest_pointer = snapshot_record(
            &["/tmp/demo"],
            Some("2026-03-31T12:00:00Z"),
            Some("VetCoders/demo"),
            Some("demo"),
            Some("feature"),
            Some("bbb222"),
            Some("0.9.0"),
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(20),
            true,
        );

        let snapshots = [older, newer, latest_pointer];
        let effective = effective_bucket_snapshots(&snapshots);
        assert_eq!(
            format_cache_meta(&effective),
            "scans 2; branches 2; latest 2026-03-31T12:00:00Z; ref feature@bbb222; schema 0.9.0"
        );
    }

    #[test]
    fn test_collect_cache_bucket_row_falls_back_without_snapshot_metadata() {
        let temp = TempDir::new().expect("create temp bucket");
        fs::write(temp.path().join("artifact.bin"), b"cache-bytes").expect("write artifact");

        let row = collect_cache_bucket_row("feedfacecafebeef", temp.path());

        assert_eq!(row.org_repo, "unknown/feedfacecafebeef");
        assert_eq!(row.project_path, "(unknown path)");
        assert_eq!(row.meta, "scans 0; latest unknown; schema unknown");
        assert!(row.size_bytes > 0);
    }
}
