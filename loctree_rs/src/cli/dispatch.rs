//! Dispatcher for the new command interface.
//!
//! This module converts `Command` variants into `ParsedArgs` and dispatches
//! to the existing handlers. This provides a bridge between the new CLI
//! interface and the existing implementation.

use std::path::PathBuf;

use crate::args::ParsedArgs;
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

    // Convert command-specific options
    match cmd {
        Command::Auto(opts) => {
            // Auto mode: full scan with stack detection, save to .loctree/
            // Maps to Mode::Init (which does scan + snapshot)
            // Unless --for-agent-feed is set, then use Mode::ForAi
            if opts.for_agent_feed {
                parsed.mode = Mode::ForAi;
                parsed.output = OutputMode::Json;
            } else {
                parsed.mode = Mode::Init;
                parsed.auto_outputs = true;
            }
            parsed.root_list = if opts.roots.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                opts.roots.clone()
            };
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
            parsed.root_list = if let Some(ref root) = opts.root {
                vec![root.clone()]
            } else {
                vec![PathBuf::from(".")]
            };
        }

        Command::Find(opts) => {
            parsed.mode = Mode::Search;
            parsed.search_query = opts
                .query
                .clone()
                .or_else(|| opts.symbol.clone())
                .or_else(|| opts.similar.clone())
                .or_else(|| opts.impact.clone());
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
            if let Some(top) = opts.top {
                parsed.top_dead_symbols = top;
            }
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
        }

        Command::Report(opts) => {
            parsed.mode = Mode::AnalyzeImports;
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

        Command::Diff(_) => {
            // Diff is handled specially in dispatch_command
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
        Command::Help(_) => {
            return DispatchResult::ShowHelp;
        }
        Command::Version => {
            return DispatchResult::ShowVersion;
        }
        Command::Query(opts) => {
            // Execute query and return result
            return handle_query_command(opts, &parsed_cmd.global);
        }
        Command::Diff(opts) => {
            // Execute diff and return result
            return handle_diff_command(opts, &parsed_cmd.global);
        }
        _ => {}
    }

    // Convert to ParsedArgs for the existing handlers
    let parsed_args = command_to_parsed_args(&parsed_cmd.command, &parsed_cmd.global);
    DispatchResult::Continue(Box::new(parsed_args))
}

/// Handle the query command directly
fn handle_query_command(opts: &QueryOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::query::{query_component_of, query_where_symbol, query_who_imports};
    use crate::snapshot::Snapshot;
    use std::path::Path;

    // Load snapshot from current directory
    let root = Path::new(".");
    let snapshot = match Snapshot::load(root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[loct][error] Failed to load snapshot: {}", e);
            eprintln!("[loct][hint] Run 'loct scan' first to create a snapshot.");
            return DispatchResult::Exit(1);
        }
    };

    // Execute the query
    let result = match opts.kind {
        QueryKind::WhoImports => query_who_imports(&snapshot, &opts.target),
        QueryKind::WhereSymbol => query_where_symbol(&snapshot, &opts.target),
        QueryKind::ComponentOf => query_component_of(&snapshot, &opts.target),
    };

    // Output results
    if global.json {
        // JSON output
        match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize results: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        // Human-readable output
        println!("{} '{}':", result.kind, result.target);
        if result.results.is_empty() {
            println!("  (no results)");
        } else {
            for m in &result.results {
                if let Some(line) = m.line {
                    print!("  {}:{}", m.file, line);
                } else {
                    print!("  {}", m.file);
                }
                if let Some(ref ctx) = m.context {
                    print!(" - {}", ctx);
                }
                println!();
            }
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the diff command directly
fn handle_diff_command(opts: &DiffOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::diff::SnapshotDiff;
    use crate::snapshot::Snapshot;
    use std::path::Path;

    // For MVP: Load snapshots from paths or IDs
    // `--since` is required and points to a snapshot path or ID
    let since_path = if let Some(s) = opts.since.as_ref() {
        s
    } else {
        eprintln!("[loct][error] --since is required for diff.");
        eprintln!("[loct][hint] try: loct diff --since <snapshot_path|branch@sha|HEAD~N>");
        return DispatchResult::Exit(1);
    };

    // Load "from" snapshot
    let from_snapshot = match Snapshot::load(Path::new(since_path)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[loct][error] Failed to load snapshot from '{}': {}",
                since_path, e
            );
            eprintln!("[loct][hint] Provide a valid snapshot path or run 'loct scan' first.");
            return DispatchResult::Exit(1);
        }
    };

    // Load "to" snapshot (current if not specified)
    let to_snapshot = if let Some(ref to_path) = opts.to {
        match Snapshot::load(Path::new(to_path)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[loct][error] Failed to load snapshot from '{}': {}",
                    to_path, e
                );
                return DispatchResult::Exit(1);
            }
        }
    } else {
        // Load current snapshot from .loctree/
        match Snapshot::load(Path::new(".")) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[loct][error] Failed to load current snapshot: {}", e);
                eprintln!("[loct][hint] Run 'loct scan' first to create a snapshot.");
                return DispatchResult::Exit(1);
            }
        }
    };

    // For now, we don't have git commit info in this flow
    // In future, we could extract it from snapshot metadata
    let from_commit = None;
    let to_commit = None;

    // We don't have changed_files info without git integration
    // For snapshot-to-snapshot diff, we'll compute it from the diff itself
    let changed_files = vec![];

    // Compare snapshots
    let diff = SnapshotDiff::compare(
        &from_snapshot,
        &to_snapshot,
        from_commit,
        to_commit,
        &changed_files,
    );

    // If problems_only flag is set, compute NEW problems only
    if opts.problems_only {
        return handle_problems_only_diff(
            &from_snapshot,
            &to_snapshot,
            &diff,
            since_path,
            opts,
            global,
        );
    }

    // Output results (full diff)
    if global.json || opts.jsonl {
        // JSON/JSONL output
        if opts.jsonl {
            // One-line JSON (compact)
            match serde_json::to_string(&diff) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize diff: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        } else {
            // Pretty JSON
            match serde_json::to_string_pretty(&diff) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize diff: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        }
    } else {
        // Human-readable output
        println!("Snapshot Diff:");
        println!("  From: {}", since_path);
        if let Some(ref to_path) = opts.to {
            println!("  To:   {}", to_path);
        } else {
            println!("  To:   (current)");
        }
        println!();
        println!("Summary: {}", diff.impact.summary);
        println!("Risk Score: {:.2}", diff.impact.risk_score);
        println!();

        if !diff.files.added.is_empty() {
            println!("Files Added ({}):", diff.files.added.len());
            for path in &diff.files.added {
                println!("  + {}", path.display());
            }
            println!();
        }

        if !diff.files.removed.is_empty() {
            println!("Files Removed ({}):", diff.files.removed.len());
            for path in &diff.files.removed {
                println!("  - {}", path.display());
            }
            println!();
        }

        if !diff.files.modified.is_empty() {
            println!("Files Modified ({}):", diff.files.modified.len());
            for path in &diff.files.modified {
                println!("  ~ {}", path.display());
            }
            println!();
        }

        if !diff.exports.removed.is_empty() {
            println!("Exports Removed ({}):", diff.exports.removed.len());
            for export in &diff.exports.removed {
                println!(
                    "  - {} ({}) in {}",
                    export.name,
                    export.kind,
                    export.file.display()
                );
            }
            println!();
        }

        if !diff.exports.added.is_empty() {
            println!("Exports Added ({}):", diff.exports.added.len());
            for export in &diff.exports.added {
                println!(
                    "  + {} ({}) in {}",
                    export.name,
                    export.kind,
                    export.file.display()
                );
            }
            println!();
        }
    }

    DispatchResult::Exit(0)
}

/// Handle problems-only diff output: show only NEW problems
fn handle_problems_only_diff(
    from_snapshot: &crate::snapshot::Snapshot,
    to_snapshot: &crate::snapshot::Snapshot,
    _diff: &crate::diff::SnapshotDiff,
    since_path: &str,
    opts: &DiffOptions,
    global: &GlobalOptions,
) -> DispatchResult {
    use crate::analyzer::cycles::find_cycles;
    use crate::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports};
    use serde_json::json;
    use std::collections::HashSet;

    // 1. Find dead exports in both snapshots
    let dead_config = DeadFilterConfig::default();
    let from_dead = find_dead_exports(&from_snapshot.files, true, None, dead_config);
    let to_dead = find_dead_exports(&to_snapshot.files, true, None, dead_config);

    // Build sets for comparison (use symbol, not name)
    let from_dead_set: HashSet<(&str, &str)> = from_dead
        .iter()
        .map(|d| (d.file.as_str(), d.symbol.as_str()))
        .collect();

    let new_dead_exports: Vec<_> = to_dead
        .iter()
        .filter(|d| !from_dead_set.contains(&(d.file.as_str(), d.symbol.as_str())))
        .collect();

    // 2. Find circular imports (cycles) in both snapshots
    // Extract edges from snapshots
    let from_edges: Vec<(String, String, String)> = from_snapshot
        .edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
        .collect();
    let to_edges: Vec<(String, String, String)> = to_snapshot
        .edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
        .collect();

    let from_cycles = find_cycles(&from_edges);
    let to_cycles = find_cycles(&to_edges);

    // Build cycle signature sets for comparison
    let from_cycle_sigs: HashSet<String> = from_cycles
        .iter()
        .map(|cycle| {
            let mut sorted = cycle.clone();
            sorted.sort();
            sorted.join("|")
        })
        .collect();

    let new_cycles: Vec<_> = to_cycles
        .iter()
        .filter(|cycle| {
            let mut sorted = (*cycle).clone();
            sorted.sort();
            let sig = sorted.join("|");
            !from_cycle_sigs.contains(&sig)
        })
        .collect();

    // 3. Find missing handlers in both snapshots
    let from_missing: HashSet<String> = from_snapshot
        .command_bridges
        .iter()
        .filter(|b| !b.has_handler && b.is_called)
        .map(|b| b.name.clone())
        .collect();

    let new_missing_handlers: Vec<_> = to_snapshot
        .command_bridges
        .iter()
        .filter(|b| !b.has_handler && b.is_called && !from_missing.contains(&b.name))
        .collect();

    let total_problems = new_dead_exports.len() + new_cycles.len() + new_missing_handlers.len();

    // Output results
    if global.json || opts.jsonl {
        let problems = json!({
            "from": since_path,
            "to": opts.to.as_deref().unwrap_or("(current)"),
            "new_problems": {
                "dead_exports": new_dead_exports.iter().map(|d| json!({
                    "file": d.file,
                    "symbol": d.symbol,
                    "confidence": d.confidence,
                    "line": d.line,
                    "reason": d.reason,
                })).collect::<Vec<_>>(),
                "circular_imports": new_cycles.iter().map(|cycle| json!({
                    "path": cycle,
                    "length": cycle.len(),
                })).collect::<Vec<_>>(),
                "missing_handlers": new_missing_handlers.iter().map(|b| json!({
                    "name": b.name,
                    "frontend_calls": b.frontend_calls,
                })).collect::<Vec<_>>(),
            },
            "summary": {
                "new_dead_exports": new_dead_exports.len(),
                "new_circular_imports": new_cycles.len(),
                "new_missing_handlers": new_missing_handlers.len(),
            }
        });

        if opts.jsonl {
            match serde_json::to_string(&problems) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize problems: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        } else {
            match serde_json::to_string_pretty(&problems) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize problems: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        }
    } else {
        // Human-readable output
        println!("New Problems Since Last Snapshot:");
        println!("  From: {}", since_path);
        if let Some(ref to_path) = opts.to {
            println!("  To:   {}", to_path);
        } else {
            println!("  To:   (current)");
        }
        println!();

        if total_problems == 0 {
            println!("✓ No new problems detected!");
        } else {
            if !new_dead_exports.is_empty() {
                println!("New Dead Exports ({}):", new_dead_exports.len());
                for export in &new_dead_exports {
                    let confidence_indicator = match export.confidence.as_str() {
                        "high" => "🔴",
                        "medium" => "🟡",
                        _ => "⚪",
                    };
                    let line_info = export.line.map(|l| format!(":{}", l)).unwrap_or_default();
                    println!(
                        "  {} {} in {}{} [{}]",
                        confidence_indicator,
                        export.symbol,
                        export.file,
                        line_info,
                        export.confidence
                    );
                }
                println!();
            }

            if !new_cycles.is_empty() {
                println!("New Circular Imports ({}):", new_cycles.len());
                for cycle in &new_cycles {
                    println!("  Cycle of {} files:", cycle.len());
                    for (i, file) in cycle.iter().enumerate() {
                        if i == cycle.len() - 1 {
                            println!("    {} → (back to {})", file, cycle[0]);
                        } else {
                            println!("    {}", file);
                        }
                    }
                }
                println!();
            }

            if !new_missing_handlers.is_empty() {
                println!("New Missing Handlers ({}):", new_missing_handlers.len());
                for bridge in &new_missing_handlers {
                    println!("  Command: {}", bridge.name);
                    println!("    Frontend calls ({}):", bridge.frontend_calls.len());
                    for (file, line) in &bridge.frontend_calls {
                        println!("      {}:{}", file, line);
                    }
                }
                println!();
            }

            println!("Summary: {} new problem(s) detected", total_problems);
        }

        return DispatchResult::Exit(if total_problems > 0 { 1 } else { 0 });
    }

    // For JSON output, exit with non-zero if problems found
    DispatchResult::Exit(if total_problems > 0 { 1 } else { 0 })
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
            path_filter: None,
            with_tests: false,
            with_helpers: false,
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
}
