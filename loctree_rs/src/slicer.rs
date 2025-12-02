//! VS2 Holographic Slice - Extract context for AI agents
//!
//! The slicer extracts a 3-layer context for a target file:
//! - Core: The target file itself (full source code)
//! - Deps: Files imported by target (signatures only by default)
//! - Consumers: Files that import target (optional, via --impact flag)
//!
//! This implements the "scan once, slice many" philosophy for AI-oriented analysis.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, IsTerminal, Write as IoWrite};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::args::ParsedArgs;
use crate::snapshot::Snapshot;

/// Configuration for slice operation
pub struct SliceConfig {
    /// Include consumer layer (files that import target)
    pub include_consumers: bool,
    /// Maximum depth for dependency traversal (default: 2)
    pub max_depth: usize,
}

impl Default for SliceConfig {
    fn default() -> Self {
        Self {
            include_consumers: false,
            max_depth: 2,
        }
    }
}

/// A file in the slice with its layer info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SliceFile {
    /// File path relative to project root
    pub path: String,
    /// Layer: core, deps, or consumers
    pub layer: String,
    /// Lines of code
    pub loc: usize,
    /// Language (rust, typescript, etc.)
    pub language: String,
    /// Depth from target (0 = core, 1 = direct dep, etc.)
    pub depth: usize,
}

/// The complete slice result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HolographicSlice {
    /// Target file that was sliced
    pub target: String,
    /// Core layer files (the target itself)
    pub core: Vec<SliceFile>,
    /// Dependencies layer files
    pub deps: Vec<SliceFile>,
    /// Consumer layer files (who imports target)
    pub consumers: Vec<SliceFile>,
    /// Command bridges involving the target
    pub command_bridges: Vec<String>,
    /// Event bridges involving the target
    pub event_bridges: Vec<String>,
    /// Statistics
    pub stats: SliceStats,
}

/// Statistics about the slice
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SliceStats {
    pub core_files: usize,
    pub core_loc: usize,
    pub deps_files: usize,
    pub deps_loc: usize,
    pub consumers_files: usize,
    pub consumers_loc: usize,
    pub total_files: usize,
    pub total_loc: usize,
}

/// Strip common extensions from a path for matching
fn strip_extension(path: &str) -> &str {
    // Common extensions that may be omitted in imports
    const EXTENSIONS: &[&str] = &[
        ".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs", ".rs", ".py", ".css", ".scss", ".sass",
    ];
    for ext in EXTENSIONS {
        if let Some(stripped) = path.strip_suffix(ext) {
            return stripped;
        }
    }
    path
}

impl HolographicSlice {
    /// Create a slice from a file path using snapshot data
    pub fn from_path(snapshot: &Snapshot, target_path: &str, config: &SliceConfig) -> Option<Self> {
        // Normalize target path (remove leading ./)
        let normalized = target_path.trim_start_matches("./").replace('\\', "/");

        // Build adjacency maps from snapshot edges
        // Note: edges may have paths with or without extensions, so we build
        // maps with both forms for flexible lookup
        let mut imports: HashMap<String, Vec<String>> = HashMap::new();
        let mut imported_by: HashMap<String, Vec<String>> = HashMap::new();

        for edge in &snapshot.edges {
            // Store with original key
            imports
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            imported_by
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());

            // Also store with stripped extension key for matching
            let from_stripped = strip_extension(&edge.from);
            let to_stripped = strip_extension(&edge.to);
            if from_stripped != edge.from {
                imports
                    .entry(from_stripped.to_string())
                    .or_default()
                    .push(edge.to.clone());
            }
            if to_stripped != edge.to {
                imported_by
                    .entry(to_stripped.to_string())
                    .or_default()
                    .push(edge.from.clone());
            }
        }

        // Find the target file in snapshot
        // Priority: exact match > ends_with match
        // Warn if multiple matches found
        let matches: Vec<_> = snapshot
            .files
            .iter()
            .filter(|f| {
                let path_normalized = f.path.trim_start_matches("./").replace('\\', "/");
                path_normalized == normalized
                    || path_normalized.ends_with(&normalized)
                    || normalized.ends_with(&path_normalized)
            })
            .collect();

        if matches.is_empty() {
            return None;
        }

        // Prefer exact match
        let target_file = matches
            .iter()
            .find(|f| {
                let path_normalized = f.path.trim_start_matches("./").replace('\\', "/");
                path_normalized == normalized
            })
            .copied()
            .or_else(|| {
                // Fallback to longest path match (most specific)
                if matches.len() > 1 {
                    eprintln!(
                        "[loctree][warn] Multiple files match '{}': {}. Using longest path.",
                        target_path,
                        matches
                            .iter()
                            .map(|f| f.path.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                matches.iter().max_by_key(|f| f.path.len()).copied()
            })?;

        let target_path_norm = target_file.path.clone();
        // Also create stripped version for edge lookup
        let target_stripped = strip_extension(&target_path_norm).to_string();

        let mut slice = Self {
            target: target_file.path.clone(),
            core: Vec::new(),
            deps: Vec::new(),
            consumers: Vec::new(),
            command_bridges: Vec::new(),
            event_bridges: Vec::new(),
            stats: SliceStats {
                core_files: 0,
                core_loc: 0,
                deps_files: 0,
                deps_loc: 0,
                consumers_files: 0,
                consumers_loc: 0,
                total_files: 0,
                total_loc: 0,
            },
        };

        // Layer 1: Core - the target file itself
        slice.core.push(SliceFile {
            path: target_file.path.clone(),
            layer: "core".to_string(),
            loc: target_file.loc,
            language: target_file.language.clone(),
            depth: 0,
        });

        // Layer 2: Deps - files imported by target (BFS)
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        visited.insert(target_path_norm.clone());
        visited.insert(target_stripped.clone());

        // Try lookup with both full path and stripped path
        let direct_deps: Vec<String> = imports
            .get(&target_path_norm)
            .into_iter()
            .chain(imports.get(&target_stripped))
            .flatten()
            .cloned()
            .collect();

        for dep in direct_deps {
            let dep_stripped = strip_extension(&dep).to_string();
            if !visited.contains(&dep) && !visited.contains(&dep_stripped) {
                queue.push_back((dep.clone(), 1));
                visited.insert(dep);
                visited.insert(dep_stripped);
            }
        }

        while let Some((path, depth)) = queue.pop_front() {
            if depth > config.max_depth {
                continue;
            }

            // Find matching file in snapshot (try exact match first, then stripped)
            let file = snapshot
                .files
                .iter()
                .find(|f| f.path == path || strip_extension(&f.path) == path);

            if let Some(file) = file {
                slice.deps.push(SliceFile {
                    path: file.path.clone(),
                    layer: "deps".to_string(),
                    loc: file.loc,
                    language: file.language.clone(),
                    depth,
                });
            }

            // Go deeper for transitive deps
            if depth < config.max_depth {
                let path_stripped = strip_extension(&path).to_string();
                let transitive: Vec<String> = imports
                    .get(&path)
                    .into_iter()
                    .chain(imports.get(&path_stripped))
                    .flatten()
                    .cloned()
                    .collect();

                for dep in transitive {
                    let dep_stripped = strip_extension(&dep).to_string();
                    if !visited.contains(&dep) && !visited.contains(&dep_stripped) {
                        queue.push_back((dep.clone(), depth + 1));
                        visited.insert(dep);
                        visited.insert(dep_stripped);
                    }
                }
            }
        }

        // Layer 3: Consumers - files that import target
        // For barrel files (index.ts), we need to transitively find consumers through re-export chains
        if config.include_consumers {
            let mut all_consumers = HashSet::new();
            let mut to_visit: VecDeque<String> = VecDeque::new();
            let mut visited_for_consumers = HashSet::new();

            // Start with direct consumers
            let direct_consumers: Vec<String> = imported_by
                .get(&target_path_norm)
                .into_iter()
                .chain(imported_by.get(&target_stripped))
                .flatten()
                .cloned()
                .collect();

            for consumer in direct_consumers {
                all_consumers.insert(consumer.clone());
                to_visit.push_back(consumer);
            }

            // Transitively follow through barrel files
            // If A imports barrel B, and B re-exports target, then A is a consumer of target
            while let Some(current) = to_visit.pop_front() {
                if visited_for_consumers.contains(&current) {
                    continue;
                }
                visited_for_consumers.insert(current.clone());

                // Check if current file is a barrel that re-exports the target
                let current_file = snapshot
                    .files
                    .iter()
                    .find(|f| f.path == current || strip_extension(&f.path) == current);

                let is_barrel = current_file
                    .map(|f| !f.reexports.is_empty())
                    .unwrap_or(false);

                if is_barrel {
                    // Find consumers of this barrel and add them
                    let current_stripped = strip_extension(&current).to_string();
                    let barrel_consumers: Vec<String> = imported_by
                        .get(&current)
                        .into_iter()
                        .chain(imported_by.get(&current_stripped))
                        .flatten()
                        .cloned()
                        .collect();

                    for consumer in barrel_consumers {
                        if all_consumers.insert(consumer.clone()) {
                            to_visit.push_back(consumer);
                        }
                    }
                }
            }

            // Convert consumer paths to SliceFile objects
            for consumer_path in all_consumers {
                let file = snapshot
                    .files
                    .iter()
                    .find(|f| f.path == consumer_path || strip_extension(&f.path) == consumer_path);

                if let Some(file) = file {
                    // Avoid duplicates (shouldn't happen with HashSet, but safety check)
                    if !slice.consumers.iter().any(|c| c.path == file.path) {
                        slice.consumers.push(SliceFile {
                            path: file.path.clone(),
                            layer: "consumers".to_string(),
                            loc: file.loc,
                            language: file.language.clone(),
                            depth: 1,
                        });
                    }
                }
            }
        }

        // Collect command bridges involving this file
        for bridge in &snapshot.command_bridges {
            let involves_target = bridge
                .frontend_calls
                .iter()
                .any(|(f, _)| f == &target_path_norm || strip_extension(f) == target_stripped)
                || bridge
                    .backend_handler
                    .as_ref()
                    .map(|(f, _)| f == &target_path_norm || strip_extension(f) == target_stripped)
                    .unwrap_or(false);
            if involves_target {
                slice.command_bridges.push(bridge.name.clone());
            }
        }

        // Collect event bridges involving this file
        for bridge in &snapshot.event_bridges {
            let involves_target =
                bridge.emits.iter().any(|(f, _, _)| {
                    f == &target_path_norm || strip_extension(f) == target_stripped
                }) || bridge
                    .listens
                    .iter()
                    .any(|(f, _)| f == &target_path_norm || strip_extension(f) == target_stripped);
            if involves_target {
                slice.event_bridges.push(bridge.name.clone());
            }
        }

        // Calculate stats
        slice.stats.core_files = slice.core.len();
        slice.stats.core_loc = slice.core.iter().map(|f| f.loc).sum();
        slice.stats.deps_files = slice.deps.len();
        slice.stats.deps_loc = slice.deps.iter().map(|f| f.loc).sum();
        slice.stats.consumers_files = slice.consumers.len();
        slice.stats.consumers_loc = slice.consumers.iter().map(|f| f.loc).sum();
        slice.stats.total_files =
            slice.stats.core_files + slice.stats.deps_files + slice.stats.consumers_files;
        slice.stats.total_loc =
            slice.stats.core_loc + slice.stats.deps_loc + slice.stats.consumers_loc;

        // Sort deps by depth, then by path
        slice
            .deps
            .sort_by(|a, b| a.depth.cmp(&b.depth).then(a.path.cmp(&b.path)));
        slice.consumers.sort_by(|a, b| a.path.cmp(&b.path));

        Some(slice)
    }

    /// Print slice in human-readable format
    pub fn print(&self) {
        println!("Slice for: {}", self.target);
        println!();

        println!(
            "Core ({} files, {} LOC):",
            self.stats.core_files, self.stats.core_loc
        );
        for f in &self.core {
            println!("  {} ({} LOC, {})", f.path, f.loc, f.language);
        }

        println!(
            "\nDeps ({} files, {} LOC):",
            self.stats.deps_files, self.stats.deps_loc
        );

        const DISPLAY_LIMIT: usize = 25;

        for (i, f) in self.deps.iter().enumerate() {
            if i >= DISPLAY_LIMIT {
                println!(
                    "  ... and {} more (use --json for full list)",
                    self.deps.len() - DISPLAY_LIMIT
                );
                break;
            }
            let indent = "  ".repeat(f.depth);
            println!(
                "{}[d{}] {} ({} LOC, {})",
                indent, f.depth, f.path, f.loc, f.language
            );
        }

        if !self.consumers.is_empty() {
            println!(
                "\nConsumers ({} files, {} LOC):",
                self.stats.consumers_files, self.stats.consumers_loc
            );

            for (i, f) in self.consumers.iter().enumerate() {
                if i >= DISPLAY_LIMIT {
                    println!(
                        "  ... and {} more (use --json for full list)",
                        self.consumers.len() - DISPLAY_LIMIT
                    );
                    break;
                }
                println!("  {} ({} LOC, {})", f.path, f.loc, f.language);
            }
        }

        if !self.command_bridges.is_empty() {
            println!("\nCommand bridges: {}", self.command_bridges.join(", "));
        }

        if !self.event_bridges.is_empty() {
            println!("Event bridges: {}", self.event_bridges.join(", "));
        }

        println!(
            "\nTotal: {} files, {} LOC",
            self.stats.total_files, self.stats.total_loc
        );
    }

    /// Output as JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "target": self.target,
            "core": self.core,
            "deps": self.deps,
            "consumers": self.consumers,
            "commandBridges": self.command_bridges,
            "eventBridges": self.event_bridges,
            "stats": self.stats,
        })
    }
}

/// Prompt user to create snapshot if it doesn't exist (TTY only)
fn prompt_create_snapshot(root: &Path, parsed: &ParsedArgs) -> io::Result<bool> {
    let snapshot_path = root.join(".loctree").join("snapshot.json");

    if !std::io::stdin().is_terminal() {
        // Non-interactive: print clear error and exit (avoid ugly Debug output)
        eprintln!();
        eprintln!("❌ No snapshot found at {}", snapshot_path.display());
        eprintln!();
        eprintln!("   The `slice` command requires a snapshot. Create one with:");
        eprintln!();
        eprintln!("     cd {} && loctree", root.display());
        eprintln!();
        std::process::exit(1);
    }

    eprintln!("No snapshot found at {}", snapshot_path.display());
    eprintln!("Run `loctree` first to create a snapshot.");
    eprintln!();
    eprint!("Create snapshot now? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().is_empty() || input.trim().to_lowercase() == "y" {
        // Run init first
        crate::snapshot::run_init(&[root.to_path_buf()], parsed)?;
        eprintln!();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Run slice command
pub fn run_slice(
    root: &Path,
    target: &str,
    include_consumers: bool,
    json_output: bool,
    parsed: &ParsedArgs,
) -> io::Result<()> {
    // Search upward for .loctree/ directory (like git finds .git/)
    let effective_root = Snapshot::find_loctree_root(root)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| Snapshot::find_loctree_root(&cwd))
        })
        .unwrap_or_else(|| root.to_path_buf());

    // Check if snapshot exists, prompt to create if not
    if !Snapshot::exists(&effective_root) {
        if prompt_create_snapshot(&effective_root, parsed)? {
            // Snapshot was created, continue
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No snapshot found. Run `loctree` first to create one.",
            ));
        }
    }

    let snapshot = Snapshot::load(&effective_root)?;

    let config = SliceConfig {
        include_consumers,
        max_depth: 2,
    };

    let slice = match HolographicSlice::from_path(&snapshot, target, &config) {
        Some(s) => s,
        None => {
            eprintln!();
            eprintln!("❌ Target file '{}' not found in snapshot.", target);
            eprintln!();
            eprintln!("   Possible causes:");
            eprintln!("   • File path is incorrect or uses wrong case");
            eprintln!("   • File was added after last snapshot (run `loctree` to update)");
            eprintln!("   • File is excluded by .gitignore or .loctignore");
            eprintln!();
            std::process::exit(1);
        }
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&slice.to_json())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        );
    } else {
        slice.print();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{EventBridge, GraphEdge, Snapshot, SnapshotMetadata};
    use crate::types::FileAnalysis;

    fn create_test_snapshot() -> Snapshot {
        Snapshot {
            metadata: SnapshotMetadata {
                schema_version: "0.5.0-test".to_string(),
                generated_at: "2025-01-01T00:00:00Z".to_string(),
                roots: vec!["/test".to_string()],
                languages: ["rust".to_string()].into_iter().collect(),
                file_count: 4,
                total_loc: 400,
                scan_duration_ms: 100,
                resolver_config: None,
                git_repo: None,
                git_branch: None,
                git_commit: None,
                git_scan_id: None,
            },
            files: vec![
                FileAnalysis {
                    path: "src/main.rs".to_string(),
                    loc: 100,
                    language: "rust".to_string(),
                    ..FileAnalysis::new("src/main.rs".to_string())
                },
                FileAnalysis {
                    path: "src/lib.rs".to_string(),
                    loc: 150,
                    language: "rust".to_string(),
                    ..FileAnalysis::new("src/lib.rs".to_string())
                },
                FileAnalysis {
                    path: "src/utils.rs".to_string(),
                    loc: 80,
                    language: "rust".to_string(),
                    ..FileAnalysis::new("src/utils.rs".to_string())
                },
                FileAnalysis {
                    path: "src/tests.rs".to_string(),
                    loc: 70,
                    language: "rust".to_string(),
                    ..FileAnalysis::new("src/tests.rs".to_string())
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "src/main.rs".to_string(),
                    to: "src/lib.rs".to_string(),
                    label: "import".to_string(),
                },
                GraphEdge {
                    from: "src/lib.rs".to_string(),
                    to: "src/utils.rs".to_string(),
                    label: "import".to_string(),
                },
                GraphEdge {
                    from: "src/tests.rs".to_string(),
                    to: "src/lib.rs".to_string(),
                    label: "import".to_string(),
                },
            ],
            export_index: Default::default(),
            command_bridges: vec![],
            event_bridges: vec![EventBridge {
                name: "test_event".to_string(),
                emits: vec![("src/lib.rs".to_string(), 10, "emit".to_string())],
                listens: vec![("src/main.rs".to_string(), 20)],
            }],
            barrels: vec![],
        }
    }

    #[test]
    fn test_slice_core_only() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig::default();

        let slice = HolographicSlice::from_path(&snapshot, "src/lib.rs", &config)
            .expect("slice src/lib.rs");

        assert_eq!(slice.target, "src/lib.rs");
        assert_eq!(slice.core.len(), 1);
        assert_eq!(slice.core[0].path, "src/lib.rs");
        assert_eq!(slice.stats.core_loc, 150);
    }

    #[test]
    fn test_slice_with_deps() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig::default();

        let slice = HolographicSlice::from_path(&snapshot, "src/lib.rs", &config)
            .expect("slice src/lib.rs");

        // lib.rs imports utils.rs
        assert_eq!(slice.deps.len(), 1);
        assert_eq!(slice.deps[0].path, "src/utils.rs");
        assert_eq!(slice.deps[0].depth, 1);
    }

    #[test]
    fn test_slice_with_consumers() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig {
            include_consumers: true,
            ..Default::default()
        };

        let slice = HolographicSlice::from_path(&snapshot, "src/lib.rs", &config)
            .expect("slice src/lib.rs with consumers");

        // lib.rs is imported by main.rs and tests.rs
        assert_eq!(slice.consumers.len(), 2);
        let consumer_paths: Vec<_> = slice.consumers.iter().map(|f| f.path.as_str()).collect();
        assert!(consumer_paths.contains(&"src/main.rs"));
        assert!(consumer_paths.contains(&"src/tests.rs"));
    }

    #[test]
    fn test_slice_transitive_deps() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig {
            include_consumers: false,
            max_depth: 2,
        };

        let slice = HolographicSlice::from_path(&snapshot, "src/main.rs", &config)
            .expect("slice src/main.rs with transitive deps");

        // main.rs -> lib.rs (depth 1) -> utils.rs (depth 2)
        assert_eq!(slice.deps.len(), 2);
        let dep_paths: Vec<_> = slice.deps.iter().map(|f| f.path.as_str()).collect();
        assert!(dep_paths.contains(&"src/lib.rs"));
        assert!(dep_paths.contains(&"src/utils.rs"));
    }

    #[test]
    fn test_slice_event_bridges() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig::default();

        let slice = HolographicSlice::from_path(&snapshot, "src/lib.rs", &config)
            .expect("slice src/lib.rs");

        // lib.rs emits test_event
        assert_eq!(slice.event_bridges.len(), 1);
        assert_eq!(slice.event_bridges[0], "test_event");
    }

    #[test]
    fn test_slice_not_found() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig::default();

        let slice = HolographicSlice::from_path(&snapshot, "nonexistent.rs", &config);
        assert!(slice.is_none());
    }

    #[test]
    fn test_slice_stats() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig {
            include_consumers: true,
            max_depth: 1,
        };

        let slice = HolographicSlice::from_path(&snapshot, "src/lib.rs", &config)
            .expect("slice src/lib.rs for stats");

        assert_eq!(slice.stats.core_files, 1);
        assert_eq!(slice.stats.core_loc, 150); // lib.rs
        assert_eq!(slice.stats.deps_files, 1); // utils.rs
        assert_eq!(slice.stats.deps_loc, 80);
        assert_eq!(slice.stats.consumers_files, 2); // main.rs, tests.rs
        assert_eq!(slice.stats.consumers_loc, 170); // 100 + 70
        assert_eq!(slice.stats.total_files, 4);
        assert_eq!(slice.stats.total_loc, 400);
    }

    #[test]
    fn test_slice_config_default() {
        let config = SliceConfig::default();
        assert!(!config.include_consumers);
        assert_eq!(config.max_depth, 2);
    }

    #[test]
    fn test_slice_file_fields() {
        let file = SliceFile {
            path: "src/main.rs".to_string(),
            layer: "core".to_string(),
            loc: 100,
            language: "rust".to_string(),
            depth: 0,
        };
        assert_eq!(file.path, "src/main.rs");
        assert_eq!(file.layer, "core");
        assert_eq!(file.loc, 100);
        assert_eq!(file.language, "rust");
        assert_eq!(file.depth, 0);
    }

    #[test]
    fn test_slice_stats_default() {
        let stats = SliceStats {
            core_files: 0,
            core_loc: 0,
            deps_files: 0,
            deps_loc: 0,
            consumers_files: 0,
            consumers_loc: 0,
            total_files: 0,
            total_loc: 0,
        };
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_loc, 0);
    }

    #[test]
    fn test_slice_depth_limit() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig {
            include_consumers: false,
            max_depth: 1, // Only direct deps
        };

        let slice = HolographicSlice::from_path(&snapshot, "src/main.rs", &config)
            .expect("slice src/main.rs with depth 1");

        // main.rs -> lib.rs (depth 1), but utils.rs (depth 2) should be excluded
        assert_eq!(slice.deps.len(), 1);
        assert_eq!(slice.deps[0].path, "src/lib.rs");
    }

    #[test]
    fn test_slice_no_deps() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig::default();

        // utils.rs has no outgoing edges, so no deps
        let slice = HolographicSlice::from_path(&snapshot, "src/utils.rs", &config)
            .expect("slice src/utils.rs");

        assert!(slice.deps.is_empty());
    }

    #[test]
    fn test_slice_command_bridges_empty() {
        let snapshot = create_test_snapshot();
        let config = SliceConfig::default();

        let slice = HolographicSlice::from_path(&snapshot, "src/utils.rs", &config)
            .expect("slice src/utils.rs");

        // No command bridges in this test snapshot
        assert!(slice.command_bridges.is_empty());
    }

    #[test]
    fn test_slice_serde_roundtrip() {
        let slice = HolographicSlice {
            target: "src/main.rs".to_string(),
            core: vec![SliceFile {
                path: "src/main.rs".to_string(),
                layer: "core".to_string(),
                loc: 100,
                language: "rust".to_string(),
                depth: 0,
            }],
            deps: vec![],
            consumers: vec![],
            command_bridges: vec![],
            event_bridges: vec![],
            stats: SliceStats {
                core_files: 1,
                core_loc: 100,
                deps_files: 0,
                deps_loc: 0,
                consumers_files: 0,
                consumers_loc: 0,
                total_files: 1,
                total_loc: 100,
            },
        };

        let json = serde_json::to_string(&slice).expect("serialize");
        let deser: HolographicSlice = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deser.target, "src/main.rs");
        assert_eq!(deser.core.len(), 1);
        assert_eq!(deser.stats.core_loc, 100);
    }

    #[test]
    fn test_slice_consumers_through_barrel() {
        use crate::types::{FileAnalysis, ReexportEntry, ReexportKind};

        // Create a test snapshot with barrel file re-export chain:
        // Component.tsx -> features/index.ts (barrel) -> App.tsx
        let snapshot = Snapshot {
            metadata: SnapshotMetadata {
                schema_version: "0.5.0-test".to_string(),
                generated_at: "2025-01-01T00:00:00Z".to_string(),
                roots: vec!["/test".to_string()],
                languages: ["typescript".to_string()].into_iter().collect(),
                file_count: 3,
                total_loc: 300,
                scan_duration_ms: 100,
                resolver_config: None,
                git_repo: None,
                git_branch: None,
                git_commit: None,
                git_scan_id: None,
            },
            files: vec![
                FileAnalysis {
                    path: "src/Component.tsx".to_string(),
                    loc: 100,
                    language: "typescript".to_string(),
                    ..FileAnalysis::new("src/Component.tsx".to_string())
                },
                {
                    let mut barrel = FileAnalysis {
                        path: "src/features/index.ts".to_string(),
                        loc: 10,
                        language: "typescript".to_string(),
                        ..FileAnalysis::new("src/features/index.ts".to_string())
                    };
                    barrel.reexports.push(ReexportEntry {
                        source: "../Component".to_string(),
                        kind: ReexportKind::Named(vec!["MyComponent".to_string()]),
                        resolved: Some("src/Component.tsx".to_string()),
                    });
                    barrel
                },
                FileAnalysis {
                    path: "src/App.tsx".to_string(),
                    loc: 150,
                    language: "typescript".to_string(),
                    ..FileAnalysis::new("src/App.tsx".to_string())
                },
            ],
            edges: vec![
                // App.tsx imports from barrel
                GraphEdge {
                    from: "src/App.tsx".to_string(),
                    to: "src/features/index.ts".to_string(),
                    label: "import".to_string(),
                },
                // Barrel re-exports Component
                GraphEdge {
                    from: "src/features/index.ts".to_string(),
                    to: "src/Component.tsx".to_string(),
                    label: "reexport".to_string(),
                },
            ],
            export_index: Default::default(),
            command_bridges: vec![],
            event_bridges: vec![],
            barrels: vec![],
        };

        let config = SliceConfig {
            include_consumers: true,
            max_depth: 2,
        };

        let slice = HolographicSlice::from_path(&snapshot, "src/Component.tsx", &config)
            .expect("slice Component.tsx with consumers through barrel");

        // CRITICAL TEST: App.tsx should show up as a consumer of Component.tsx
        // even though it imports through the barrel file
        assert_eq!(
            slice.consumers.len(),
            2,
            "Should have both barrel and App.tsx as consumers"
        );
        let consumer_paths: Vec<_> = slice.consumers.iter().map(|f| f.path.as_str()).collect();
        assert!(
            consumer_paths.contains(&"src/App.tsx"),
            "App.tsx should be a consumer (imports through barrel)"
        );
        assert!(
            consumer_paths.contains(&"src/features/index.ts"),
            "Barrel should be a consumer (directly re-exports)"
        );
    }
}
