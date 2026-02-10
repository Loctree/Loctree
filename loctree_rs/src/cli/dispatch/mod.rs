//! Dispatcher for the new command interface.
//!
//! This module converts `Command` variants into `ParsedArgs` and dispatches
//! to the existing handlers. This provides a bridge between the new CLI
//! interface and the existing implementation.

mod handlers;

use std::path::PathBuf;

use crate::args::{ParsedArgs, SearchQueryMode};
use crate::types::{DEFAULT_LOC_THRESHOLD, Mode, OutputMode};

use super::command::*;

/// Convert a Command and GlobalOptions into ParsedArgs for backward compatibility.
///
/// This allows us to reuse existing handlers while providing the new CLI interface.
pub fn command_to_parsed_args(cmd: &Command, global: &GlobalOptions) -> ParsedArgs {
    // Initialize with global options applied
    let mut parsed = ParsedArgs {
        output: if global.json {
            OutputMode::Json
        } else {
            OutputMode::Human
        },
        verbose: global.verbose,
        color: global.color,
        ..Default::default()
    };
    parsed.library_mode = global.library_mode;
    parsed.python_library = global.python_library;
    parsed.py_roots = global.py_roots.clone();

    // Handle global --findings and --summary flags first
    // These override normal command behavior to output findings to stdout
    if global.findings {
        parsed.mode = Mode::Findings;
        parsed.output = OutputMode::Json;
        parsed.root_list = vec![PathBuf::from(".")];
        return parsed;
    }
    if global.summary_only_output {
        parsed.mode = Mode::Summary;
        parsed.output = OutputMode::Json;
        parsed.root_list = vec![PathBuf::from(".")];
        return parsed;
    }

    // Convert command-specific options
    match cmd {
        Command::Auto(opts) => {
            // Auto mode: full scan with stack detection, write cached artifacts (see LOCT_CACHE_DIR).
            // Maps to Mode::Init (which does scan + snapshot)
            // Unless --for-agent-feed is set, then use Mode::ForAi
            if opts.for_agent_feed {
                parsed.mode = Mode::ForAi;
                parsed.output = if opts.agent_json {
                    OutputMode::Json
                } else {
                    OutputMode::Jsonl
                };
                parsed.for_agent_feed = true;
                parsed.agent_json = opts.agent_json;
                parsed.force_full_scan = true; // don't reuse snapshot for agent feed
            } else {
                parsed.mode = Mode::Init;
                parsed.auto_outputs = true;
            }
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.suppress_duplicates = opts.suppress_duplicates;
            parsed.suppress_dynamic = opts.suppress_dynamic;
            parsed.full_scan = opts.full_scan;
            parsed.scan_all = opts.scan_all;
            parsed.use_gitignore = true; // Auto mode respects gitignore by default
        }

        Command::Scan(opts) => {
            parsed.mode = Mode::Init;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.full_scan = opts.full_scan;
            parsed.scan_all = opts.scan_all;
            parsed.use_gitignore = true;
        }

        Command::Tree(opts) => {
            parsed.mode = Mode::Tree;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.max_depth = opts.depth;
            if let Some(limit) = opts.summary {
                parsed.summary = true;
                parsed.summary_limit = limit;
            }
            parsed.summary_only = opts.summary_only;
            parsed.loc_threshold = opts.loc_threshold.unwrap_or(DEFAULT_LOC_THRESHOLD);
            parsed.show_hidden = opts.show_hidden;
            parsed.find_artifacts = opts.find_artifacts;
            parsed.show_ignored = opts.show_ignored;
            if opts.show_ignored {
                parsed.use_gitignore = true;
            }
        }

        Command::Slice(opts) => {
            parsed.mode = Mode::Slice;
            parsed.slice_target = Some(opts.target.clone());
            parsed.slice_consumers = opts.consumers;
            parsed.slice_rescan = opts.rescan;
            parsed.root_list = if let Some(ref root) = opts.root {
                vec![root.clone()]
            } else {
                vec![PathBuf::from(".")]
            };
        }

        Command::Find(opts) => {
            parsed.mode = Mode::Search;
            parsed.search_query_mode = SearchQueryMode::Single;
            parsed.search_queries.clear();

            parsed.search_query = opts
                .query
                .clone()
                .or_else(|| opts.symbol.clone())
                .or_else(|| opts.similar.clone())
                .or_else(|| opts.impact.clone())
                .or_else(|| {
                    if opts.queries.is_empty() {
                        return None;
                    }

                    // Multi-arg positional query handling:
                    // - `loct find A B C` => split-mode (separate subqueries + cross-match)
                    // - `loct find "A B C"` => AND-mode (intersection)
                    // - `loct find --or A B C` => legacy OR (A|B|C)
                    if opts.queries.len() >= 2 {
                        if opts.or_mode {
                            Some(opts.queries.join("|"))
                        } else {
                            parsed.search_query_mode = SearchQueryMode::Split;
                            parsed.search_queries = opts.queries.clone();
                            None
                        }
                    } else {
                        // Single positional query: if it contains whitespace, treat as AND-mode.
                        let raw = opts.queries[0].trim().to_string();
                        if raw.chars().any(|c| c.is_whitespace()) && !raw.contains('|') {
                            let terms: Vec<String> =
                                raw.split_whitespace().map(|t| t.to_string()).collect();
                            if terms.len() >= 2 {
                                parsed.search_query_mode = SearchQueryMode::And;
                                parsed.search_queries = terms;
                                None
                            } else {
                                Some(raw)
                            }
                        } else {
                            Some(raw)
                        }
                    }
                });
            parsed.symbol = opts.symbol.clone();
            parsed.impact = opts.impact.clone();
            parsed.check_sim = opts.similar.clone();
            parsed.search_dead_only = opts.dead_only;
            parsed.search_exported_only = opts.exported_only;
            parsed.search_lang = opts.lang.clone();
            parsed.search_limit = opts.limit;
            parsed.root_list = vec![PathBuf::from(".")];
        }

        Command::Dead(opts) => {
            parsed.mode = Mode::AnalyzeImports;
            parsed.dead_exports = true;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.dead_confidence = opts.confidence.clone();
            parsed.top_dead_symbols = if opts.full {
                usize::MAX
            } else if let Some(top) = opts.top {
                top
            } else {
                parsed.top_dead_symbols
            };
            parsed.use_gitignore = true;
            parsed.with_tests = opts.with_tests;
            parsed.with_helpers = opts.with_helpers;
        }

        Command::Cycles(opts) => {
            parsed.mode = Mode::AnalyzeImports;
            parsed.circular = true;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.use_gitignore = true;
        }

        Command::Trace(_) => {
            // Trace is handled specially in dispatch_command
        }

        Command::Commands(opts) => {
            // Commands shows Tauri command bridges
            parsed.mode = Mode::AnalyzeImports;
            parsed.tauri_preset = true;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.use_gitignore = true;
            parsed.commands_name_filter = opts.name_filter.clone();
            parsed.commands_missing_only = opts.missing_only;
            parsed.commands_unused_only = opts.unused_only;
            parsed.suppress_duplicates = opts.suppress_duplicates;
            parsed.suppress_dynamic = opts.suppress_dynamic;
        }

        Command::Events(opts) => {
            // Events analysis (ghost/orphan/races)
            parsed.mode = Mode::AnalyzeImports;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            // Enable race detection if specified
            parsed.py_races = opts.races;
            parsed.use_gitignore = true;
            parsed.suppress_duplicates = opts.suppress_duplicates;
            parsed.suppress_dynamic = opts.suppress_dynamic;
        }
        Command::Pipelines(opts) => {
            parsed.mode = Mode::AnalyzeImports;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.use_gitignore = true;
        }
        Command::Insights(opts) => {
            parsed.mode = Mode::AnalyzeImports;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.use_gitignore = true;
        }
        Command::Manifests(opts) => {
            parsed.mode = Mode::AnalyzeImports;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            parsed.use_gitignore = true;
        }

        Command::Info(_opts) => {
            // Info command - show snapshot metadata
            // For now, map to Init which will show info if snapshot exists
            parsed.mode = Mode::Init;
            parsed.root_list = vec![PathBuf::from(".")];
        }

        Command::Lint(opts) => {
            parsed.mode = Mode::AnalyzeImports;
            parsed.entrypoints = opts.entrypoints;
            parsed.sarif = opts.sarif;
            parsed.tauri_preset = opts.tauri;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            if opts.fail {
                parsed.fail_on_missing_handlers = true;
                parsed.fail_on_ghost_events = true;
            }
            parsed.use_gitignore = true;
            parsed.suppress_duplicates = opts.suppress_duplicates;
            parsed.suppress_dynamic = opts.suppress_dynamic;
        }

        Command::Report(opts) => {
            parsed.mode = Mode::AnalyzeImports;
            parsed.auto_outputs = true;
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
            if let Some(ref output) = opts.output {
                parsed.report_path = Some(output.clone());
            }
            parsed.serve = opts.serve;
            parsed.serve_port = opts.port;
            if let Some(ref editor) = opts.editor {
                parsed.editor_kind = Some(editor.clone());
            }
            parsed.use_gitignore = true;
        }

        Command::Help(opts) => {
            if opts.legacy {
                parsed.show_help_full = true; // Show legacy help
            } else {
                parsed.show_help = true;
            }
        }

        Command::Version => {
            parsed.show_version = true;
        }

        Command::Query(_) => {
            // Query is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Impact(_) => {
            // Impact is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Diff(_) => {
            // Diff is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Memex(_) => {
            // Memex is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Crowd(_) => {
            // Crowd is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Tagmap(_) => {
            // Tagmap is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Twins(_) => {
            // Twins is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Sniff(_) => {
            // Sniff is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Suppress(_) => {
            // Suppress is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Routes(_) => {
            // Routes is handled specially in dispatch_command
        }

        Command::Dist(_) => {
            // Dist is handled specially in dispatch_command
        }

        Command::Coverage(_) => {
            // Coverage is handled specially in dispatch_command
        }

        Command::JqQuery(_) => {
            // JqQuery is handled specially in dispatch_command
            // It doesn't use ParsedArgs, will be handled by jaq executor
        }

        Command::Focus(_) => {
            // Focus is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Hotspots(_) => {
            // Hotspots is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Layoutmap(_) => {
            // Layoutmap is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Zombie(_) => {
            // Zombie is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Health(_) => {
            // Health is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Audit(_) => {
            // Audit is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Doctor(_) => {
            // Doctor is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Plan(_) => {
            // Plan is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }
    }

    parsed
}

/// Result type for command dispatch.
pub enum DispatchResult {
    /// Command was handled, return this exit code
    Exit(i32),
    /// Show main help
    ShowHelp,
    /// Show legacy help
    ShowLegacyHelp,
    /// Show version
    ShowVersion,
    /// Continue with normal execution using ParsedArgs (boxed to reduce enum size)
    Continue(Box<ParsedArgs>),
}

/// Dispatch a parsed command.
///
/// Returns a DispatchResult indicating what action to take.
pub fn dispatch_command(parsed_cmd: &ParsedCommand) -> DispatchResult {
    // Emit deprecation warning if this was from legacy syntax
    parsed_cmd.emit_deprecation_warning();

    // Handle special cases first
    match &parsed_cmd.command {
        Command::Help(opts) if opts.legacy => {
            return DispatchResult::ShowLegacyHelp;
        }
        Command::Help(opts) if opts.full => {
            return DispatchResult::ShowLegacyHelp; // Full help shows legacy too
        }
        Command::Help(opts) if opts.command.is_some() => {
            let cmd_name = opts.command.clone().unwrap();
            if let Some(text) = Command::format_command_help(&cmd_name) {
                println!("{}", text);
                return DispatchResult::Exit(0);
            } else {
                eprintln!(
                    "Unknown command '{}'. Run 'loct --help' for available commands.",
                    cmd_name
                );
                return DispatchResult::Exit(1);
            }
        }
        Command::Help(_) => {
            return DispatchResult::ShowHelp;
        }
        Command::Version => {
            return DispatchResult::ShowVersion;
        }
        Command::Query(opts) => {
            // Execute query and return result
            return handlers::query::handle_query_command(opts, &parsed_cmd.global);
        }
        Command::Impact(opts) => {
            // Execute impact analysis and return result
            return handlers::diff::handle_impact_command(opts, &parsed_cmd.global);
        }
        Command::Diff(opts) => {
            // Execute diff and return result
            return handlers::diff::handle_diff_command(opts, &parsed_cmd.global);
        }
        Command::Memex(opts) => {
            // Execute memex and return result
            return handlers::ai::handle_memex_command(opts, &parsed_cmd.global);
        }
        Command::Crowd(opts) => {
            return handlers::ai::handle_crowd_command(opts, &parsed_cmd.global);
        }
        Command::Tagmap(opts) => {
            return handlers::ai::handle_tagmap_command(opts, &parsed_cmd.global);
        }
        Command::Twins(opts) => {
            return handlers::ai::handle_twins_command(opts, &parsed_cmd.global);
        }
        Command::Sniff(opts) => {
            return handlers::ai::handle_sniff_command(opts, &parsed_cmd.global);
        }
        Command::Suppress(opts) => {
            return handlers::ai::handle_suppress_command(opts, &parsed_cmd.global);
        }
        Command::Dead(opts) => {
            return handlers::analysis::handle_dead_command(opts, &parsed_cmd.global);
        }
        Command::Cycles(opts) => {
            return handlers::analysis::handle_cycles_command(opts, &parsed_cmd.global);
        }
        Command::Trace(opts) => {
            return handlers::analysis::handle_trace_command(opts, &parsed_cmd.global);
        }
        Command::Commands(opts) => {
            return handlers::analysis::handle_commands_command(opts, &parsed_cmd.global);
        }
        Command::Routes(opts) => {
            return handlers::analysis::handle_routes_command(opts, &parsed_cmd.global);
        }
        Command::Events(opts) => {
            return handlers::analysis::handle_events_command(opts, &parsed_cmd.global);
        }
        Command::Pipelines(opts) => {
            return handlers::analysis::handle_pipelines_command(opts, &parsed_cmd.global);
        }
        Command::Insights(opts) => {
            return handlers::analysis::handle_insights_command(opts, &parsed_cmd.global);
        }
        Command::Manifests(opts) => {
            return handlers::analysis::handle_manifests_command(opts, &parsed_cmd.global);
        }
        Command::Lint(opts) => {
            return handlers::output::handle_lint_command(opts, &parsed_cmd.global);
        }
        Command::Dist(opts) => {
            return handlers::output::handle_dist_command(opts, &parsed_cmd.global);
        }
        Command::Coverage(opts) => {
            return handlers::watch::handle_coverage_command(opts, &parsed_cmd.global);
        }
        Command::JqQuery(opts) => {
            return handlers::query::handle_jq_query_command(opts, &parsed_cmd.global);
        }
        Command::Focus(opts) => {
            return handlers::analysis::handle_focus_command(opts, &parsed_cmd.global);
        }
        Command::Hotspots(opts) => {
            return handlers::analysis::handle_hotspots_command(opts, &parsed_cmd.global);
        }
        Command::Layoutmap(opts) => {
            return handlers::analysis::handle_layoutmap_command(opts, &parsed_cmd.global);
        }
        Command::Zombie(opts) => {
            return handlers::analysis::handle_zombie_command(opts, &parsed_cmd.global);
        }
        Command::Health(opts) => {
            return handlers::analysis::handle_health_command(opts, &parsed_cmd.global);
        }
        Command::Audit(opts) => {
            return handlers::analysis::handle_audit_command(opts, &parsed_cmd.global);
        }
        Command::Doctor(opts) => {
            return handlers::analysis::handle_doctor_command(opts, &parsed_cmd.global);
        }
        Command::Plan(opts) => {
            return handlers::analysis::handle_plan_command(opts, &parsed_cmd.global);
        }
        Command::Scan(opts) if opts.watch => {
            return handlers::watch::handle_scan_watch_command(opts, &parsed_cmd.global);
        }
        // Note: Command::Report falls through to ParsedArgs flow to use full analysis pipeline
        // which includes twins data, graph visualization, and proper Leptos SSR rendering
        _ => {}
    }

    // Convert to ParsedArgs for the existing handlers
    let parsed_args = command_to_parsed_args(&parsed_cmd.command, &parsed_cmd.global);
    DispatchResult::Continue(Box::new(parsed_args))
}

/// Load existing snapshot or create one if missing (used by handler submodules)
///
/// Respects global flags:
/// - `--fresh`: Force rescan even if snapshot exists
/// - `--no-scan`: Fail if no snapshot exists (don't auto-scan)
/// - `--fail-stale`: Fail if snapshot git_head differs from current HEAD
pub(crate) fn load_or_create_snapshot_for_roots(
    roots: &[std::path::PathBuf],
    global: &GlobalOptions,
) -> std::io::Result<crate::snapshot::Snapshot> {
    use crate::args::ParsedArgs;
    use crate::snapshot::Snapshot;
    use crate::snapshot::resolve_snapshot_root;

    let snapshot_root = resolve_snapshot_root(roots);

    // If --fresh, skip loading and go straight to scan
    if !global.fresh {
        match Snapshot::load(&snapshot_root) {
            Ok(s) => {
                // Check for stale snapshot if --fail-stale is set
                if global.fail_stale
                    && let Some(snapshot_commit) = &s.metadata.git_commit
                {
                    let current_commit = get_current_git_head(&snapshot_root);
                    if let Some(ref current) = current_commit {
                        // Compare using prefix match (snapshot may have short hash)
                        let is_same = current.starts_with(snapshot_commit)
                            || snapshot_commit.starts_with(current);
                        if !is_same {
                            return Err(std::io::Error::other(format!(
                                "Snapshot is stale: snapshot commit={} but current HEAD={}. Run 'loct' to rescan or use --fresh.",
                                &snapshot_commit[..7.min(snapshot_commit.len())],
                                &current[..7.min(current.len())]
                            )));
                        }
                    }
                }
                return Ok(s);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // No snapshot - check if --no-scan forbids auto-scan
                if global.no_scan {
                    return Err(std::io::Error::other(
                        "No snapshot found and --no-scan is set. Run 'loct' first to create a snapshot.",
                    ));
                }
                if !global.quiet {
                    eprintln!("[loct] No snapshot found, running initial scan...");
                }
            }
            Err(e) => return Err(e), // Other errors (corruption, etc.) - fail
        }
    } else {
        // --fresh: force rescan
        if !global.quiet {
            eprintln!("[loct] --fresh: forcing rescan...");
        }
    }

    // Create minimal ParsedArgs for scan, propagating output mode
    let parsed = ParsedArgs {
        verbose: global.verbose,
        use_gitignore: true,
        output: if global.json {
            OutputMode::Json
        } else {
            OutputMode::Human
        },
        ..Default::default()
    };

    // Run scan (suppress summary in json/quiet mode to keep stdout clean)
    crate::snapshot::run_init_with_options(roots, &parsed, global.json || global.quiet)?;

    // Now load the freshly created snapshot
    Snapshot::load(&snapshot_root)
}

pub(crate) fn load_or_create_snapshot(
    root: &std::path::Path,
    global: &GlobalOptions,
) -> std::io::Result<crate::snapshot::Snapshot> {
    let root_list = vec![root.to_path_buf()];
    load_or_create_snapshot_for_roots(&root_list, global)
}

/// Get current git HEAD commit hash
fn get_current_git_head(root: &std::path::Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Check if a file is a test file (used by handler submodules)
pub(crate) fn is_test_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Directory patterns: tests/, __tests__/, test/, spec/
    if path_lower.contains("/tests/")
        || path_lower.contains("/__tests__/")
        || path_lower.contains("/test/")
        || path_lower.contains("/spec/")
        || path_lower.contains("/fixtures/")
        || path_lower.contains("/mocks/")
    {
        return true;
    }

    // File patterns: *_test.*, *.test.*, *_spec.*, *.spec.*, test_*, tests.*
    if filename.contains("_test.")
        || filename.contains(".test.")
        || filename.contains("_spec.")
        || filename.contains(".spec.")
        || filename.contains("_tests.") // Rust: module_tests.rs
        || filename.starts_with("test_")
        || filename.starts_with("spec_")
        || filename.starts_with("tests.") // tests.rs
        || filename == "conftest.py"
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_command_to_parsed_args() {
        let cmd = Command::Auto(AutoOptions {
            roots: vec![PathBuf::from(".")],
            full_scan: true,
            scan_all: false,
            for_agent_feed: false,
            agent_json: false,
            suppress_duplicates: false,
            suppress_dynamic: false,
        });
        let global = GlobalOptions::default();
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::Init));
        assert!(parsed.full_scan);
        assert!(!parsed.scan_all);
    }

    #[test]
    fn test_dead_command_to_parsed_args() {
        let cmd = Command::Dead(DeadOptions {
            roots: vec![],
            confidence: Some("high".into()),
            top: Some(10),
            full: false,
            path_filter: None,
            with_tests: false,
            with_helpers: false,
            with_shadows: false,
            with_ambient: false,
            with_dynamic: false,
        });
        let global = GlobalOptions {
            json: true,
            ..Default::default()
        };
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::AnalyzeImports));
        assert!(parsed.dead_exports);
        assert_eq!(parsed.dead_confidence, Some("high".into()));
        assert_eq!(parsed.top_dead_symbols, 10);
        assert!(!parsed.with_tests);
        assert!(!parsed.with_helpers);
        assert!(matches!(parsed.output, OutputMode::Json));
    }

    #[test]
    fn test_tree_command_to_parsed_args() {
        let cmd = Command::Tree(TreeOptions {
            roots: vec![PathBuf::from("src")],
            depth: Some(3),
            summary: Some(5),
            summary_only: false,
            loc_threshold: Some(500),
            show_hidden: true,
            find_artifacts: false,
            show_ignored: false,
        });
        let global = GlobalOptions::default();
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::Tree));
        assert_eq!(parsed.max_depth, Some(3));
        assert!(parsed.summary);
        assert_eq!(parsed.summary_limit, 5);
        assert_eq!(parsed.loc_threshold, 500);
        assert!(parsed.show_hidden);
    }

    #[test]
    fn test_slice_command_to_parsed_args() {
        let cmd = Command::Slice(SliceOptions {
            target: "src/main.rs".into(),
            root: None,
            consumers: true,
            depth: None,
            rescan: false,
        });
        let global = GlobalOptions {
            json: true,
            ..Default::default()
        };
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::Slice));
        assert_eq!(parsed.slice_target, Some("src/main.rs".into()));
        assert!(parsed.slice_consumers);
        assert!(matches!(parsed.output, OutputMode::Json));
    }

    #[test]
    fn test_find_multi_arg_defaults_to_split_mode() {
        let cmd = Command::Find(FindOptions {
            queries: vec!["Props".into(), "Options".into(), "ViewModel".into()],
            ..Default::default()
        });
        let global = GlobalOptions::default();
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::Search));
        assert!(matches!(parsed.search_query_mode, SearchQueryMode::Split));
        assert_eq!(parsed.search_queries, vec!["Props", "Options", "ViewModel"]);
        assert!(parsed.search_query.is_none());
    }

    #[test]
    fn test_find_single_arg_with_spaces_defaults_to_and_mode() {
        let cmd = Command::Find(FindOptions {
            queries: vec!["Props Options ViewModel".into()],
            ..Default::default()
        });
        let global = GlobalOptions::default();
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::Search));
        assert!(matches!(parsed.search_query_mode, SearchQueryMode::And));
        assert_eq!(parsed.search_queries, vec!["Props", "Options", "ViewModel"]);
        assert!(parsed.search_query.is_none());
    }

    #[test]
    fn test_find_multi_arg_can_force_legacy_or_mode() {
        let cmd = Command::Find(FindOptions {
            queries: vec!["Props".into(), "Options".into()],
            or_mode: true,
            ..Default::default()
        });
        let global = GlobalOptions::default();
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::Search));
        assert!(matches!(parsed.search_query_mode, SearchQueryMode::Single));
        assert_eq!(parsed.search_query.as_deref(), Some("Props|Options"));
        assert!(parsed.search_queries.is_empty());
    }

    #[test]
    fn test_find_single_arg_with_pipe_stays_single_mode() {
        let cmd = Command::Find(FindOptions {
            queries: vec!["Props|Options|ViewModel".into()],
            ..Default::default()
        });
        let global = GlobalOptions::default();
        let parsed = command_to_parsed_args(&cmd, &global);

        assert!(matches!(parsed.mode, Mode::Search));
        assert!(matches!(parsed.search_query_mode, SearchQueryMode::Single));
        assert_eq!(
            parsed.search_query.as_deref(),
            Some("Props|Options|ViewModel")
        );
        assert!(parsed.search_queries.is_empty());
    }

    #[test]
    fn test_dispatch_help_command() {
        let parsed_cmd = ParsedCommand::new(
            Command::Help(HelpOptions::default()),
            GlobalOptions::default(),
        );
        let result = dispatch_command(&parsed_cmd);
        assert!(matches!(result, DispatchResult::ShowHelp));
    }

    #[test]
    fn test_dispatch_legacy_help_command() {
        let parsed_cmd = ParsedCommand::new(
            Command::Help(HelpOptions {
                legacy: true,
                ..Default::default()
            }),
            GlobalOptions::default(),
        );
        let result = dispatch_command(&parsed_cmd);
        assert!(matches!(result, DispatchResult::ShowLegacyHelp));
    }

    #[test]
    fn test_dispatch_version_command() {
        let parsed_cmd = ParsedCommand::new(Command::Version, GlobalOptions::default());
        let result = dispatch_command(&parsed_cmd);
        assert!(matches!(result, DispatchResult::ShowVersion));
    }

    #[test]
    fn test_crowd_command_to_dispatch() {
        let parsed_cmd = ParsedCommand::new(
            Command::Crowd(CrowdOptions {
                pattern: Some("message".into()),
                ..Default::default()
            }),
            GlobalOptions::default(),
        );
        // Just verify it doesn't panic and returns Exit (will fail without snapshot, but that's OK)
        let result = dispatch_command(&parsed_cmd);
        // Should be Exit(1) because no snapshot exists in test env
        assert!(matches!(result, DispatchResult::Exit(_)));
    }
}
