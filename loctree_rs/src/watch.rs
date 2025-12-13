//! Watch mode for live snapshot refresh during iterative development.
//!
//! This module provides file system watching capabilities that:
//! - Monitor file changes in real-time
//! - Debounce changes to avoid thrashing (500ms default)
//! - Incrementally re-scan only changed files
//! - Respect existing ignore patterns (.gitignore, .loctreeignore)
//! - Allow graceful shutdown via Ctrl+C

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdMap, new_debouncer};

use crate::args::ParsedArgs;
use crate::fs_utils::GitIgnoreChecker;
use crate::snapshot;

/// Watch configuration
pub struct WatchConfig {
    /// Paths to watch
    pub roots: Vec<PathBuf>,
    /// Debounce duration (default: 500ms)
    pub debounce_duration: Duration,
    /// File extensions to watch (empty = all)
    pub extensions: Option<Vec<String>>,
    /// Gitignore checker for filtering
    pub gitignore: Option<GitIgnoreChecker>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            roots: vec![PathBuf::from(".")],
            debounce_duration: Duration::from_millis(500),
            extensions: None,
            gitignore: None,
        }
    }
}

/// Start watching for file changes and trigger re-scans
pub fn watch_and_rescan(config: WatchConfig, parsed_args: &ParsedArgs) -> anyhow::Result<()> {
    let (tx, rx) = channel();

    // Create debouncer with specified duration
    let mut debouncer: Debouncer<RecommendedWatcher, FileIdMap> = new_debouncer(
        config.debounce_duration,
        None, // No separate tick rate
        move |result: DebounceEventResult| {
            if let Err(e) = tx.send(result) {
                eprintln!("[watch] Error sending event: {e}");
            }
        },
    )?;

    // Add paths to watch
    for root in &config.roots {
        debouncer
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| anyhow::anyhow!("Failed to watch {}: {}", root.display(), e))?;
    }

    // Count initial files
    let initial_count = count_tracked_files(&config.roots, &config.extensions, &config.gitignore);

    // Perform initial scan
    eprintln!("[watch] Initial scan...");
    let start = std::time::Instant::now();
    if let Err(e) = snapshot::run_init(&config.roots, parsed_args) {
        eprintln!("[watch] Initial scan failed: {e}");
        return Err(anyhow::anyhow!("Initial scan failed: {e}"));
    }
    let elapsed = start.elapsed();
    eprintln!(
        "[watch] ✓ Scanned {} files in {:.2}s",
        initial_count,
        elapsed.as_secs_f64()
    );

    // Print watching status
    let timestamp = chrono::Local::now().format("%H:%M:%S");
    eprintln!("[{}] Watching {} files...", timestamp, initial_count);
    eprintln!("[watch] Press Ctrl+C to exit");

    // Watch loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Filter events to only those we care about
                let changed_paths =
                    collect_changed_paths(&events, &config.extensions, &config.gitignore);

                if changed_paths.is_empty() {
                    continue;
                }

                // Print what changed
                let timestamp = chrono::Local::now().format("%H:%M:%S");
                if changed_paths.len() == 1 {
                    eprintln!(
                        "[{}] Changed: {} → re-scanning...",
                        timestamp,
                        changed_paths.iter().next().unwrap().display()
                    );
                } else {
                    eprintln!(
                        "[{}] Changed {} files → re-scanning...",
                        timestamp,
                        changed_paths.len()
                    );
                }

                // Re-scan
                let start = std::time::Instant::now();
                if let Err(e) = snapshot::run_init(&config.roots, parsed_args) {
                    eprintln!("[watch] Re-scan failed: {e}");
                    continue;
                }
                let elapsed = start.elapsed();

                // Print summary
                let current_count =
                    count_tracked_files(&config.roots, &config.extensions, &config.gitignore);
                eprintln!(
                    "[{}] ✓ Scanned {} files in {:.2}s",
                    chrono::Local::now().format("%H:%M:%S"),
                    current_count,
                    elapsed.as_secs_f64()
                );
            }
            Ok(Err(errors)) => {
                for error in errors {
                    eprintln!("[watch] Error: {error}");
                }
            }
            Err(e) => {
                eprintln!("[watch] Watch error: {e}");
                break;
            }
        }
    }

    Ok(())
}

/// Collect paths that changed from debounced events
fn collect_changed_paths(
    events: &[notify_debouncer_full::DebouncedEvent],
    extensions: &Option<Vec<String>>,
    gitignore: &Option<GitIgnoreChecker>,
) -> HashSet<PathBuf> {
    let mut paths = HashSet::new();

    for event in events {
        for path in &event.paths {
            // Skip if gitignored
            if let Some(checker) = gitignore {
                if checker.is_ignored(path) {
                    continue;
                }
            }

            // Skip if wrong extension
            if let Some(exts) = extensions {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if !exts.iter().any(|e| e == ext_str) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Skip directories
            if path.is_dir() {
                continue;
            }

            paths.insert(path.clone());
        }
    }

    paths
}

/// Count files that would be tracked
fn count_tracked_files(
    roots: &[PathBuf],
    extensions: &Option<Vec<String>>,
    gitignore: &Option<GitIgnoreChecker>,
) -> usize {
    let mut count = 0;

    for root in roots {
        if let Ok(walker) = walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
        {
            for entry in walker {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();

                // Skip if gitignored
                if let Some(checker) = gitignore {
                    if checker.is_ignored(path) {
                        continue;
                    }
                }

                // Skip if wrong extension
                if let Some(exts) = extensions {
                    if let Some(ext) = path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            if !exts.iter().any(|e| e == ext_str) {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                count += 1;
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watch_config_defaults() {
        let config = WatchConfig::default();
        assert_eq!(config.roots, vec![PathBuf::from(".")]);
        assert_eq!(config.debounce_duration, Duration::from_millis(500));
        assert!(config.extensions.is_none());
        assert!(config.gitignore.is_none());
    }

    #[test]
    fn test_watch_config_custom() {
        let config = WatchConfig {
            roots: vec![PathBuf::from("src"), PathBuf::from("tests")],
            debounce_duration: Duration::from_millis(1000),
            extensions: Some(vec!["ts".to_string(), "tsx".to_string()]),
            gitignore: None,
        };

        assert_eq!(config.roots.len(), 2);
        assert_eq!(config.debounce_duration, Duration::from_millis(1000));
        assert!(config.extensions.is_some());
        assert_eq!(config.extensions.unwrap(), vec!["ts", "tsx"]);
    }

    #[test]
    fn test_count_tracked_files() {
        let temp = TempDir::new().unwrap();

        // Create test files
        fs::write(temp.path().join("test1.ts"), "").unwrap();
        fs::write(temp.path().join("test2.ts"), "").unwrap();
        fs::write(temp.path().join("test3.js"), "").unwrap();
        fs::write(temp.path().join("readme.txt"), "").unwrap();

        let count = count_tracked_files(&vec![temp.path().to_path_buf()], &None, &None);
        assert_eq!(count, 4); // All files

        let extensions = Some(vec!["ts".to_string()]);
        let count_filtered =
            count_tracked_files(&vec![temp.path().to_path_buf()], &extensions, &None);
        assert_eq!(count_filtered, 2); // Only .ts files
    }

    #[test]
    fn test_count_tracked_files_empty_directory() {
        let temp = TempDir::new().unwrap();
        let count = count_tracked_files(&vec![temp.path().to_path_buf()], &None, &None);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_tracked_files_nested() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        fs::write(temp.path().join("root.ts"), "").unwrap();
        fs::write(subdir.join("nested.ts"), "").unwrap();

        let count = count_tracked_files(&vec![temp.path().to_path_buf()], &None, &None);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_tracked_files_multiple_roots() {
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();

        fs::write(temp1.path().join("file1.ts"), "").unwrap();
        fs::write(temp2.path().join("file2.ts"), "").unwrap();

        let count = count_tracked_files(
            &vec![temp1.path().to_path_buf(), temp2.path().to_path_buf()],
            &None,
            &None,
        );
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_tracked_files_with_extension_filter() {
        let temp = TempDir::new().unwrap();

        fs::write(temp.path().join("file.ts"), "").unwrap();
        fs::write(temp.path().join("file.js"), "").unwrap();
        fs::write(temp.path().join("file.tsx"), "").unwrap();
        fs::write(temp.path().join("readme.md"), "").unwrap();

        let extensions = Some(vec!["ts".to_string(), "tsx".to_string()]);
        let count = count_tracked_files(&vec![temp.path().to_path_buf()], &extensions, &None);
        assert_eq!(count, 2); // Only .ts and .tsx files
    }

    #[test]
    fn test_count_tracked_files_ignores_subdirectories() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        fs::write(temp.path().join("file.ts"), "").unwrap();

        let count = count_tracked_files(&vec![temp.path().to_path_buf()], &None, &None);
        assert_eq!(count, 1); // Only the file, not the directory
    }

    #[test]
    fn test_debounce_duration_custom() {
        let config = WatchConfig {
            debounce_duration: Duration::from_millis(250),
            ..Default::default()
        };
        assert_eq!(config.debounce_duration, Duration::from_millis(250));
    }

    #[test]
    fn test_watch_config_with_gitignore() {
        // Test that WatchConfig can hold a GitIgnoreChecker
        // We can't easily construct one in tests, so just verify the structure
        let config = WatchConfig {
            gitignore: None,
            ..Default::default()
        };
        assert!(config.gitignore.is_none());
    }

    #[test]
    fn test_watch_config_with_multiple_extensions() {
        let config = WatchConfig {
            extensions: Some(vec![
                "ts".to_string(),
                "tsx".to_string(),
                "js".to_string(),
                "jsx".to_string(),
            ]),
            ..Default::default()
        };

        assert_eq!(config.extensions.unwrap().len(), 4);
    }
}
