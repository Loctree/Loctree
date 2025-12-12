//! Dispatcher for the new command interface.
//!
//! This module converts `Command` variants into `ParsedArgs` and dispatches
//! to the existing handlers. This provides a bridge between the new CLI
//! interface and the existing implementation.

use std::path::PathBuf;

use crate::args::ParsedArgs;
use crate::progress::Spinner;
use crate::types::{DEFAULT_LOC_THRESHOLD, Mode, OutputMode};

use super::command::CrowdOptions;
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

    // Convert command-specific options
    match cmd {
        Command::Auto(opts) => {
            // Auto mode: full scan with stack detection, save to .loctree/
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

        Command::Twins(_) => {
            // Twins is handled specially in dispatch_command
            // as it doesn't go through ParsedArgs
        }

        Command::Sniff(_) => {
            // Sniff is handled specially in dispatch_command
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
            return handle_query_command(opts, &parsed_cmd.global);
        }
        Command::Diff(opts) => {
            // Execute diff and return result
            return handle_diff_command(opts, &parsed_cmd.global);
        }
        Command::Memex(opts) => {
            // Execute memex and return result
            return handle_memex_command(opts, &parsed_cmd.global);
        }
        Command::Crowd(opts) => {
            return handle_crowd_command(opts, &parsed_cmd.global);
        }
        Command::Twins(opts) => {
            return handle_twins_command(opts, &parsed_cmd.global);
        }
        Command::Sniff(opts) => {
            return handle_sniff_command(opts, &parsed_cmd.global);
        }
        Command::Dead(opts) => {
            return handle_dead_command(opts, &parsed_cmd.global);
        }
        Command::Cycles(opts) => {
            return handle_cycles_command(opts, &parsed_cmd.global);
        }
        Command::Commands(opts) => {
            return handle_commands_command(opts, &parsed_cmd.global);
        }
        Command::Routes(opts) => {
            return handle_routes_command(opts, &parsed_cmd.global);
        }
        Command::Events(opts) => {
            return handle_events_command(opts, &parsed_cmd.global);
        }
        Command::Lint(opts) => {
            return handle_lint_command(opts, &parsed_cmd.global);
        }
        Command::Dist(opts) => {
            return handle_dist_command(opts, &parsed_cmd.global);
        }
        Command::Coverage(opts) => {
            return handle_coverage_command(opts, &parsed_cmd.global);
        }
        Command::JqQuery(opts) => {
            return handle_jq_query_command(opts, &parsed_cmd.global);
        }
        // Note: Command::Report falls through to ParsedArgs flow to use full analysis pipeline
        // which includes twins data, graph visualization, and proper Leptos SSR rendering
        _ => {}
    }

    // Convert to ParsedArgs for the existing handlers
    let parsed_args = command_to_parsed_args(&parsed_cmd.command, &parsed_cmd.global);
    DispatchResult::Continue(Box::new(parsed_args))
}

/// Handle the query command directly
fn handle_query_command(opts: &QueryOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::query::{query_component_of, query_where_symbol, query_who_imports};
    use std::path::Path;

    // Load snapshot (auto-scan if missing)
    let root = Path::new(".");
    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[loct][error] {}", e);
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
    use crate::analyzer::cycles::find_cycles_with_lazy;
    use crate::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports};
    use serde_json::json;
    use std::collections::HashSet;

    // 1. Find dead exports in both snapshots
    let dead_config = DeadFilterConfig::default();
    let from_dead = find_dead_exports(&from_snapshot.files, true, None, dead_config.clone());
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

    let from_cycles = find_cycles_with_lazy(&from_edges).0;
    let to_cycles = find_cycles_with_lazy(&to_edges).0;

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

/// Handle the memex command - index analysis into AI memory
#[cfg(feature = "memex")]
fn handle_memex_command(opts: &MemexOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::memex;

    // Run the async memex indexer using a blocking runtime
    let result = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create async runtime: {}", e))
        .and_then(|rt| {
            rt.block_on(async { memex::run_memex(opts, global.json, global.verbose).await })
        });

    match result {
        Ok(indexed_count) => {
            if !global.quiet {
                if global.json {
                    let json = serde_json::json!({
                        "status": "success",
                        "indexed_documents": indexed_count,
                        "namespace": &opts.namespace,
                    });
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                } else {
                    eprintln!(
                        "[loct][memex] Successfully indexed {} documents into namespace '{}'",
                        indexed_count, opts.namespace
                    );
                }
            }
            DispatchResult::Exit(0)
        }
        Err(e) => {
            eprintln!("[loct][memex][error] {}", e);
            DispatchResult::Exit(1)
        }
    }
}

/// Handle the memex command - stub when feature not enabled
#[cfg(not(feature = "memex"))]
fn handle_memex_command(_opts: &MemexOptions, _global: &GlobalOptions) -> DispatchResult {
    eprintln!(
        "[loct][memex][error] memex feature not enabled. Rebuild with: cargo build --features memex"
    );
    DispatchResult::Exit(1)
}

/// Load snapshot from disk, or auto-create one if missing.
/// This provides a better UX for commands that depend on snapshots.
fn load_or_create_snapshot(
    root: &std::path::Path,
    global: &GlobalOptions,
) -> std::io::Result<crate::snapshot::Snapshot> {
    use crate::args::ParsedArgs;
    use crate::snapshot::Snapshot;

    // Try to load existing snapshot
    match Snapshot::load(root) {
        Ok(s) => return Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No snapshot - auto-create one
            if !global.quiet {
                eprintln!("[loct] No snapshot found, running initial scan...");
            }
        }
        Err(e) => return Err(e), // Other errors (corruption, etc.) - fail
    }

    // Create minimal ParsedArgs for scan
    let parsed = ParsedArgs {
        verbose: global.verbose,
        use_gitignore: true,
        ..Default::default()
    };

    // Run scan
    let root_list = vec![root.to_path_buf()];
    crate::snapshot::run_init(&root_list, &parsed)?;

    // Now load the freshly created snapshot
    Snapshot::load(root)
}

/// Check if a file path looks like a test file
fn is_test_file(path: &str) -> bool {
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
        || filename.contains("_tests.")  // Rust: module_tests.rs
        || filename.starts_with("test_")
        || filename.starts_with("spec_")
        || filename.starts_with("tests.")  // tests.rs
        || filename == "conftest.py"
    {
        return true;
    }

    false
}

/// Handle the crowd command - detect functional crowds
fn handle_crowd_command(opts: &CrowdOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::crowd::{
        detect_all_crowds_with_edges, detect_crowd_with_edges, format_crowd, format_crowds_summary,
    };

    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Detecting functional crowds..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));
    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    // Filter out test files unless --include-tests is specified
    let files: Vec<_> = if opts.include_tests {
        snapshot.files.clone()
    } else {
        snapshot
            .files
            .iter()
            .filter(|f| !is_test_file(&f.path))
            .cloned()
            .collect()
    };

    // Detect crowds (using edges for accurate transitive importer counting)
    let crowds = if let Some(ref pattern) = opts.pattern {
        // Single pattern mode
        vec![detect_crowd_with_edges(&files, pattern, &snapshot.edges)]
    } else {
        // Auto-detect mode
        let mut all_crowds = detect_all_crowds_with_edges(&files, &snapshot.edges);

        // Apply min_size filter
        if let Some(min_size) = opts.min_size {
            all_crowds.retain(|c| c.members.len() >= min_size);
        }

        // Apply limit
        if let Some(limit) = opts.limit {
            all_crowds.truncate(limit);
        }

        all_crowds
    };

    // Filter out empty crowds
    let crowds: Vec<_> = crowds
        .into_iter()
        .filter(|c| !c.members.is_empty())
        .collect();

    if crowds.is_empty() {
        if let Some(s) = spinner {
            if let Some(ref pattern) = opts.pattern {
                s.finish_warning(&format!("No files found matching pattern '{}'", pattern));
            } else {
                s.finish_warning("No crowds detected in codebase");
            }
        } else if !global.quiet {
            if let Some(ref pattern) = opts.pattern {
                eprintln!(
                    "[loct][crowd] No files found matching pattern '{}'",
                    pattern
                );
            } else {
                eprintln!("[loct][crowd] No crowds detected in codebase");
            }
        }
        return DispatchResult::Exit(0);
    }

    // Finish spinner with success message
    if let Some(s) = spinner {
        let total_members: usize = crowds.iter().map(|c| c.members.len()).sum();
        s.finish_success(&format!(
            "Found {} crowd(s) with {} total members",
            crowds.len(),
            total_members
        ));
    }

    // Output results
    if global.json {
        match serde_json::to_string_pretty(&crowds) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize crowds: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        // Human-readable output
        if crowds.len() == 1 {
            // Single crowd - detailed view
            println!("{}", format_crowd(&crowds[0], global.verbose));
        } else {
            // Multiple crowds - summary view
            println!("{}", format_crowds_summary(&crowds));
        }
    }

    DispatchResult::Exit(0)
}

fn handle_twins_command(
    opts: &super::command::TwinsOptions,
    global: &GlobalOptions,
) -> DispatchResult {
    use crate::analyzer::barrels::{analyze_barrel_chaos, format_barrel_analysis};
    use crate::analyzer::twins::{
        detect_exact_twins, find_dead_parrots, print_exact_twins, print_twins_result,
    };
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing semantic duplicates..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts.path.as_deref().unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    let output_mode = if global.json {
        crate::types::OutputMode::Json
    } else {
        crate::types::OutputMode::Human
    };

    // Run dead parrot analysis
    let result = find_dead_parrots(&snapshot.files, opts.dead_only);

    // Finish spinner before printing results
    if let Some(s) = spinner {
        s.finish_success(&format!(
            "Found {} dead parrot(s)",
            result.dead_parrots.len()
        ));
    }

    print_twins_result(&result, output_mode);

    // Run exact twins detection (unless dead_only)
    if !opts.dead_only {
        let twins = detect_exact_twins(&snapshot.files);
        if !twins.is_empty() {
            print_exact_twins(&twins, output_mode);
        }

        // Run barrel chaos analysis
        let barrel_analysis = analyze_barrel_chaos(&snapshot);
        let has_issues = !barrel_analysis.missing_barrels.is_empty()
            || !barrel_analysis.deep_chains.is_empty()
            || !barrel_analysis.inconsistent_paths.is_empty();

        if has_issues && output_mode == crate::types::OutputMode::Human {
            println!("{}", format_barrel_analysis(&barrel_analysis));
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the sniff command - aggregate code smells
fn handle_sniff_command(
    opts: &super::command::SniffOptions,
    global: &GlobalOptions,
) -> DispatchResult {
    use crate::analyzer::crowd::detect_all_crowds_with_edges;
    use crate::analyzer::twins::{detect_exact_twins, find_dead_parrots};
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Sniffing for code smells..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts.path.as_deref().unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    // Filter files based on include_tests
    let files: Vec<_> = if opts.include_tests {
        snapshot.files.clone()
    } else {
        snapshot
            .files
            .iter()
            .filter(|f| !is_test_file(&f.path))
            .cloned()
            .collect()
    };

    // Collect all findings
    let mut twins_count = 0;
    let mut dead_count = 0;
    let mut crowds_count = 0;

    let twins = if !opts.crowds_only && !opts.dead_only {
        let t = detect_exact_twins(&files);
        twins_count = t.len();
        Some(t)
    } else {
        None
    };

    let dead_parrots = if !opts.crowds_only && !opts.twins_only {
        let result = find_dead_parrots(&files, false);
        dead_count = result.dead_parrots.len();
        Some(result)
    } else {
        None
    };

    let crowds = if !opts.twins_only && !opts.dead_only {
        let mut c = detect_all_crowds_with_edges(&files, &snapshot.edges);
        if let Some(min_size) = opts.min_crowd_size {
            c.retain(|crowd| crowd.members.len() >= min_size);
        }
        crowds_count = c.len();
        Some(c)
    } else {
        None
    };

    let total_smells = twins_count + dead_count + crowds_count;

    if let Some(s) = spinner {
        s.finish_success(&format!("Found {} code smell(s)", total_smells));
    }

    // Output results
    if global.json {
        // JSON output
        let output = serde_json::json!({
            "twins": twins.as_ref().map(|t| t.iter().map(|twin| {
                serde_json::json!({
                    "name": twin.name,
                    "locations": twin.locations.iter().map(|loc| {
                        serde_json::json!({
                            "file": loc.file_path,
                            "line": loc.line,
                            "kind": loc.kind,
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>()),
            "dead_parrots": dead_parrots.as_ref().map(|dp| dp.dead_parrots.iter().map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "file": e.file_path,
                    "line": e.line,
                    "kind": e.kind,
                })
            }).collect::<Vec<_>>()),
            "crowds": crowds.as_ref().map(|c| c.iter().map(|crowd| {
                serde_json::json!({
                    "pattern": crowd.pattern,
                    "size": crowd.members.len(),
                    "members": crowd.members.iter().map(|m| m.file.clone()).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>()),
            "summary": {
                "twins": twins_count,
                "dead_parrots": dead_count,
                "crowds": crowds_count,
                "total": total_smells,
            }
        });

        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize results: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        // Human-readable output with friendly tone
        println!("🐕 SNIFFING FOR CODE SMELLS...\n");

        // Twins section
        if let Some(ref twins_list) = twins
            && !twins_list.is_empty()
        {
            println!(
                "📍 TWINS (same name, different files) - {} found",
                twins_count
            );
            println!("   Consider: consolidate or rename to avoid import confusion\n");

            for twin in twins_list.iter().take(20) {
                println!("   {} ({} locations)", twin.name, twin.locations.len());
                for loc in &twin.locations {
                    println!("   ├─ {}:{}", loc.file_path, loc.line);
                }
                println!();
            }

            if twins_list.len() > 20 {
                println!("   ... and {} more twin groups\n", twins_list.len() - 20);
            }
        }

        // Dead parrots section
        if let Some(ref dp_result) = dead_parrots
            && !dp_result.dead_parrots.is_empty()
        {
            println!("📍 DEAD PARROTS (unused exports) - {} found", dead_count);
            println!("   Consider: remove if truly unused, or document if external API\n");

            // Group by file
            let mut by_file: std::collections::HashMap<
                String,
                Vec<&crate::analyzer::twins::SymbolEntry>,
            > = std::collections::HashMap::new();
            for entry in &dp_result.dead_parrots {
                by_file
                    .entry(entry.file_path.clone())
                    .or_default()
                    .push(entry);
            }

            let mut files: Vec<_> = by_file.keys().collect();
            files.sort();

            for file in files.iter().take(10) {
                let entries = &by_file[*file];
                for entry in entries.iter().take(3) {
                    println!("   {} in {}:{}", entry.name, entry.file_path, entry.line);
                }
                if entries.len() > 3 {
                    println!("   ... and {} more in {}", entries.len() - 3, file);
                }
            }

            if files.len() > 10 {
                println!(
                    "   ... and {} more files with dead exports",
                    files.len() - 10
                );
            }
            println!();
        }

        // Crowds section
        if let Some(ref crowds_list) = crowds
            && !crowds_list.is_empty()
        {
            println!("📍 CROWDS (similar files) - {} groups", crowds_count);
            println!("   Consider: these files share many dependencies, possible duplication\n");

            for (idx, crowd) in crowds_list.iter().take(5).enumerate() {
                println!("   Group {}: {} pattern", idx + 1, crowd.pattern);
                for member in crowd.members.iter().take(5) {
                    println!("   ├─ {}", member.file);
                }
                if crowd.members.len() > 5 {
                    println!("   └─ ... and {} more", crowd.members.len() - 5);
                }
                println!();
            }

            if crowds_list.len() > 5 {
                println!("   ... and {} more crowd groups\n", crowds_list.len() - 5);
            }
        }

        // Summary
        println!(
            "Summary: {} smells found. These are hints, not verdicts - you decide what matters.",
            total_smells
        );

        if total_smells == 0 {
            println!("\n✅ No code smells detected - codebase looks clean!");
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the dead command - detect dead exports
fn handle_dead_command(opts: &DeadOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports, print_dead_exports};
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing dead exports..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    // Determine confidence level
    let high_confidence = opts.confidence.as_deref() == Some("high");

    // Find dead exports
    let dead_exports = find_dead_exports(
        &snapshot.files,
        high_confidence,
        None,
        DeadFilterConfig {
            include_tests: opts.with_tests,
            include_helpers: opts.with_helpers,
            library_mode: global.library_mode,
            example_globs: Vec::new(),
            python_library_mode: global.python_library,
        },
    );

    if let Some(s) = spinner {
        s.finish_success(&format!("Found {} dead export(s)", dead_exports.len()));
    }

    // Output results
    let output_mode = if global.json {
        crate::types::OutputMode::Json
    } else {
        crate::types::OutputMode::Human
    };

    print_dead_exports(
        &dead_exports,
        output_mode,
        high_confidence,
        if opts.full {
            dead_exports.len()
        } else {
            opts.top.unwrap_or(20)
        },
    );

    DispatchResult::Exit(0)
}

/// Handle the cycles command - detect circular imports
fn handle_cycles_command(opts: &CyclesOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::cycles::{find_cycles_classified_with_lazy, print_cycles_classified};
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Detecting circular imports..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    // Extract edges from snapshot
    let edges: Vec<(String, String, String)> = snapshot
        .edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
        .collect();

    // Find and classify cycles
    let (classified_cycles, classified_lazy_cycles) = find_cycles_classified_with_lazy(&edges);

    if let Some(s) = spinner {
        let total = classified_cycles.len() + classified_lazy_cycles.len();
        s.finish_success(&format!(
            "Found {} cycle(s) ({} strict, {} lazy)",
            total,
            classified_cycles.len(),
            classified_lazy_cycles.len()
        ));
    }

    // Output results
    let json_output = global.json;
    print_cycles_classified(&classified_cycles, json_output);

    if !classified_lazy_cycles.is_empty() && !json_output {
        println!("\nLazy circular imports (info):");
        println!(
            "  Detected via imports inside functions/methods; usually safe but review if init order matters."
        );
        print_cycles_classified(&classified_lazy_cycles, false);

        // Show the lazy edges that participated (sample)
        let lazy_edges: Vec<_> = edges
            .iter()
            .filter(|(_, _, kind)| kind.contains("lazy"))
            .take(5)
            .collect();
        if !lazy_edges.is_empty() {
            println!("  Lazy edges (sample):");
            for (from, to, kind) in lazy_edges {
                println!("    {} -> {} [{}]", from, to, kind);
            }
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the commands command - show Tauri command bridges
fn handle_commands_command(opts: &CommandsOptions, global: &GlobalOptions) -> DispatchResult {
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing Tauri commands..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    // Filter command bridges based on options
    let mut bridges: Vec<_> = snapshot.command_bridges.clone();

    // Apply name filter
    if let Some(ref filter) = opts.name_filter {
        bridges.retain(|b| b.name.contains(filter));
    }

    // Apply missing-only filter
    if opts.missing_only {
        bridges.retain(|b| !b.has_handler && b.is_called);
    }

    // Apply unused-only filter
    if opts.unused_only {
        bridges.retain(|b| b.has_handler && !b.is_called);
    }

    // Apply limit if specified
    let total_before_limit = bridges.len();
    if let Some(limit) = opts.limit {
        bridges.truncate(limit);
    }

    if let Some(s) = spinner {
        if opts.limit.is_some() && total_before_limit > bridges.len() {
            s.finish_success(&format!(
                "Showing {} of {} command bridge(s)",
                bridges.len(),
                total_before_limit
            ));
        } else {
            s.finish_success(&format!("Found {} command bridge(s)", bridges.len()));
        }
    }

    // Output results
    if global.json {
        match serde_json::to_string_pretty(&bridges) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize command bridges: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        // Human-readable output
        if bridges.is_empty() {
            println!("No command bridges found matching criteria");
        } else {
            println!("Tauri Command Bridges ({} total):\n", bridges.len());

            for bridge in &bridges {
                let status = if !bridge.has_handler && bridge.is_called {
                    "MISSING"
                } else if bridge.has_handler && !bridge.is_called {
                    "UNUSED"
                } else if bridge.has_handler && bridge.is_called {
                    "OK"
                } else {
                    "?"
                };

                println!("  [{}] {}", status, bridge.name);

                if !bridge.frontend_calls.is_empty() {
                    println!("    Frontend calls ({}):", bridge.frontend_calls.len());
                    for (file, line) in bridge.frontend_calls.iter().take(3) {
                        println!("      {}:{}", file, line);
                    }
                    if bridge.frontend_calls.len() > 3 {
                        println!("      ... and {} more", bridge.frontend_calls.len() - 3);
                    }
                }

                if let Some((ref backend_file, backend_line)) = bridge.backend_handler {
                    println!("    Backend: {}:{}", backend_file, backend_line);
                }

                if !bridge.has_handler && bridge.is_called {
                    println!(
                        "    ⚠️  Why: Frontend calls invoke('{}') but no #[tauri::command] found in Rust.",
                        bridge.name
                    );
                    println!(
                        "    Impact: This command will fail at runtime with 'command not found' error."
                    );
                    if let Some((file, line)) = bridge.frontend_calls.first() {
                        println!("    First callsite: {}:{}", file, line);
                    }
                    println!(
                        "    Suggested fix: Add handler to src-tauri/src/commands/ and register in invoke_handler![]"
                    );
                    println!(
                        "    Stub: #[tauri::command] pub async fn {}(...) -> Result<(), String> {{ todo!() }}",
                        bridge.name
                    );
                } else if bridge.has_handler && !bridge.is_called {
                    println!(
                        "    ℹ️  Why: #[tauri::command] defined but no invoke('{}') calls found in frontend.",
                        bridge.name
                    );
                    println!(
                        "    Consider: If intentionally unused, remove handler. If needed, add frontend call."
                    );
                }

                println!();
            }
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the events command - analyze event flow
fn handle_events_command(opts: &EventsOptions, global: &GlobalOptions) -> DispatchResult {
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing event flow..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    if let Some(s) = spinner {
        s.finish_success(&format!(
            "Found {} event bridge(s)",
            snapshot.event_bridges.len()
        ));
    }

    // Output results
    if global.json {
        match serde_json::to_string_pretty(&snapshot.event_bridges) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize events: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else {
        println!("Event Bridges Analysis:\n");

        if snapshot.event_bridges.is_empty() {
            println!("No event bridges found");
        } else {
            println!("Found {} event bridge(s):\n", snapshot.event_bridges.len());

            for event in &snapshot.event_bridges {
                println!("  Event: {}", event.name);

                if !event.emits.is_empty() {
                    println!("    Emit locations ({}):", event.emits.len());
                    for (file, line, kind) in event.emits.iter().take(3) {
                        println!("      {}:{} [{}]", file, line, kind);
                    }
                    if event.emits.len() > 3 {
                        println!("      ... and {} more", event.emits.len() - 3);
                    }
                }

                if !event.listens.is_empty() {
                    println!("    Listen locations ({}):", event.listens.len());
                    for (file, line) in event.listens.iter().take(3) {
                        println!("      {}:{}", file, line);
                    }
                    if event.listens.len() > 3 {
                        println!("      ... and {} more", event.listens.len() - 3);
                    }
                }

                // Highlight potential issues
                if event.emits.is_empty() {
                    println!("    ⚠️  No emitters found (orphan listener?)");
                }
                if event.listens.is_empty() {
                    println!("    ⚠️  No listeners found (orphan emitter?)");
                }

                println!();
            }
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the routes command - list backend/web routes (FastAPI/Flask)
fn handle_routes_command(opts: &RoutesOptions, global: &GlobalOptions) -> DispatchResult {
    use std::path::Path;

    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Detecting backend routes..."))
    } else {
        None
    };

    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    let framework_filter = opts.framework.as_ref().map(|f| f.to_lowercase());
    let path_filter = opts.path_filter.as_ref().map(|p| p.to_lowercase());

    let mut routes: Vec<serde_json::Value> = Vec::new();

    for file in &snapshot.files {
        for r in &file.routes {
            if let Some(ff) = &framework_filter
                && r.framework.to_lowercase() != *ff
            {
                continue;
            }
            if let Some(pf) = &path_filter {
                let path_match = r
                    .path
                    .as_ref()
                    .map(|p| p.to_lowercase().contains(pf))
                    .unwrap_or(false);
                if !path_match && !file.path.to_lowercase().contains(pf) {
                    continue;
                }
            }

            routes.push(serde_json::json!({
                "framework": r.framework,
                "method": r.method,
                "path": r.path,
                "handler": r.name,
                "file": file.path,
                "line": r.line,
            }));
        }
    }

    routes.sort_by(|a, b| {
        let af = a.get("framework").and_then(|v| v.as_str()).unwrap_or("");
        let bf = b.get("framework").and_then(|v| v.as_str()).unwrap_or("");
        let ap = a.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let bp = b.get("path").and_then(|v| v.as_str()).unwrap_or("");
        af.cmp(bf).then_with(|| ap.cmp(bp))
    });

    if let Some(s) = spinner {
        s.finish_success(&format!("Found {} route(s)", routes.len()));
    }

    if global.json {
        let output = serde_json::json!({
            "routes": routes,
            "summary": { "count": routes.len() }
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else if routes.is_empty() {
        println!("No routes detected.");
    } else {
        println!("Detected routes ({}):", routes.len());
        for r in &routes {
            let framework = r.get("framework").and_then(|v| v.as_str()).unwrap_or("-");
            let method = r.get("method").and_then(|v| v.as_str()).unwrap_or("-");
            let path = r
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("(no path)");
            let file = r.get("file").and_then(|v| v.as_str()).unwrap_or("");
            let line = r.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
            let handler = r
                .get("handler")
                .and_then(|v| v.as_str())
                .unwrap_or("(anon)");
            println!(
                "  [{}] {} {} -> {}:{} ({})",
                framework, method, path, file, line, handler
            );
        }
        println!("\nTip: use --framework fastapi or --path <substr> to filter.");
    }

    DispatchResult::Exit(0)
}

/// Handle the lint command - run linting checks
fn handle_lint_command(opts: &LintOptions, global: &GlobalOptions) -> DispatchResult {
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Running lint checks..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    let mut issues_found = 0;

    // Check for missing handlers if requested
    if opts.fail || opts.tauri {
        let missing_handlers: Vec<_> = snapshot
            .command_bridges
            .iter()
            .filter(|b| !b.has_handler && b.is_called)
            .collect();

        if !missing_handlers.is_empty() {
            issues_found += missing_handlers.len();

            if !global.quiet {
                eprintln!(
                    "[loct][lint] {} missing Tauri handlers:",
                    missing_handlers.len()
                );
                for bridge in &missing_handlers {
                    eprintln!("  - {}", bridge.name);
                }
            }
        }
    }

    // Check for problematic event bridges if requested
    if opts.fail {
        // Count events with no emitters or no listeners
        let orphan_events = snapshot
            .event_bridges
            .iter()
            .filter(|e| e.emits.is_empty() || e.listens.is_empty())
            .collect::<Vec<_>>();

        if !orphan_events.is_empty() {
            issues_found += orphan_events.len();

            if !global.quiet {
                eprintln!("[loct][lint] {} orphan events:", orphan_events.len());
                for event in orphan_events.iter().take(5) {
                    if event.emits.is_empty() {
                        eprintln!("  - {} (no emitters)", event.name);
                    } else {
                        eprintln!("  - {} (no listeners)", event.name);
                    }
                }
                if orphan_events.len() > 5 {
                    eprintln!("  ... and {} more", orphan_events.len() - 5);
                }
            }
        }
    }

    // Determine exit code based on findings and --fail flag
    let exit_code = if opts.fail && issues_found > 0 { 1 } else { 0 };

    if let Some(s) = spinner {
        if issues_found > 0 {
            s.finish_warning(&format!("Found {} issue(s)", issues_found));
        } else {
            s.finish_success("No issues found");
        }
    } else if !global.quiet {
        if issues_found == 0 {
            println!("[loct][lint] No issues found");
        } else {
            println!("[loct][lint] Found {} issue(s)", issues_found);
        }
    }

    // Output SARIF format if requested
    if opts.sarif {
        // TODO: Implement SARIF output using crate::analyzer::sarif
        eprintln!("[loct][warn] SARIF output not yet implemented for unified lint command");
    }

    DispatchResult::Exit(exit_code)
}

/// Handle the dist command - analyze bundle using source maps
fn handle_dist_command(opts: &DistOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::dist::analyze_distribution;

    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing bundle distribution..."))
    } else {
        None
    };

    let source_map_path = match &opts.source_map {
        Some(p) => p.clone(),
        None => {
            if let Some(s) = spinner {
                s.finish_error("--source-map is required");
            } else {
                eprintln!("[loct][error] --source-map is required");
            }
            return DispatchResult::Exit(1);
        }
    };

    let src_path = match &opts.src {
        Some(p) => p.clone(),
        None => {
            if let Some(s) = spinner {
                s.finish_error("--src is required");
            } else {
                eprintln!("[loct][error] --src is required");
            }
            return DispatchResult::Exit(1);
        }
    };

    // Run dist analysis (uses its own scanning, doesn't need snapshot)
    match analyze_distribution(&source_map_path, &src_path) {
        Ok(result) => {
            if let Some(s) = spinner {
                s.finish_success(&format!(
                    "Found {} dead export(s) ({})",
                    result.dead_exports.len(),
                    result.reduction
                ));
            }

            if global.json {
                match serde_json::to_string_pretty(&result) {
                    Ok(json) => println!("{}", json),
                    Err(e) => {
                        eprintln!("[loct][error] Failed to serialize results: {}", e);
                        return DispatchResult::Exit(1);
                    }
                }
            } else {
                println!("Bundle Analysis:");
                println!("  Source exports:  {}", result.source_exports);
                println!("  Bundled exports: {}", result.bundled_exports);
                println!("  Dead exports:    {}", result.dead_exports.len());
                println!("  Reduction:       {}", result.reduction);
                println!(
                    "  Analysis level:  {}",
                    if result.symbol_level {
                        "symbol"
                    } else {
                        "file"
                    }
                );
                println!();

                if !result.dead_exports.is_empty() {
                    println!("Dead Exports (not in bundle):");
                    for export in result.dead_exports.iter().take(20) {
                        println!(
                            "  {} ({}) in {}:{}",
                            export.name, export.kind, export.file, export.line
                        );
                    }
                    if result.dead_exports.len() > 20 {
                        println!("  ... and {} more", result.dead_exports.len() - 20);
                    }
                }
            }

            DispatchResult::Exit(0)
        }
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Analysis failed: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            DispatchResult::Exit(1)
        }
    }
}

/// Handle the coverage command - analyze test coverage gaps
fn handle_coverage_command(opts: &CoverageOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::coverage_gaps::{GapKind, Severity, find_coverage_gaps};
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing test coverage gaps..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts
        .roots
        .first()
        .map(|p| p.as_path())
        .unwrap_or(Path::new("."));

    let snapshot = match load_or_create_snapshot(root, global) {
        Ok(s) => s,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to load snapshot: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    // Find coverage gaps
    let mut gaps = find_coverage_gaps(&snapshot);

    // Apply filters
    if opts.handlers_only {
        gaps.retain(|g| matches!(g.kind, GapKind::HandlerWithoutTest));
    }
    if opts.events_only {
        gaps.retain(|g| matches!(g.kind, GapKind::EventWithoutTest));
    }
    if let Some(ref min_sev) = opts.min_severity {
        let min_level = match min_sev.to_lowercase().as_str() {
            "critical" => 0,
            "high" => 1,
            "medium" => 2,
            "low" => 3,
            _ => 4, // show all
        };
        gaps.retain(|g| {
            let level = match g.severity {
                Severity::Critical => 0,
                Severity::High => 1,
                Severity::Medium => 2,
                Severity::Low => 3,
            };
            level <= min_level
        });
    }

    if let Some(s) = spinner {
        s.finish_success(&format!("Found {} coverage gap(s)", gaps.len()));
    }

    // Output results
    if global.json {
        match serde_json::to_string_pretty(&gaps) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("[loct][error] Failed to serialize coverage gaps: {}", e);
                return DispatchResult::Exit(1);
            }
        }
    } else if gaps.is_empty() {
        println!("✅ No coverage gaps found - all production code is tested!");
    } else {
        println!("Test Coverage Gaps ({} found):\n", gaps.len());

        // Group by severity
        let critical: Vec<_> = gaps
            .iter()
            .filter(|g| matches!(g.severity, Severity::Critical))
            .collect();
        let high: Vec<_> = gaps
            .iter()
            .filter(|g| matches!(g.severity, Severity::High))
            .collect();
        let medium: Vec<_> = gaps
            .iter()
            .filter(|g| matches!(g.severity, Severity::Medium))
            .collect();
        let low: Vec<_> = gaps
            .iter()
            .filter(|g| matches!(g.severity, Severity::Low))
            .collect();

        if !critical.is_empty() {
            println!("CRITICAL - Handlers without tests ({}):", critical.len());
            for gap in critical.iter().take(10) {
                println!("  ❌ {} ({})", gap.target, gap.location);
                println!("     {}", gap.recommendation);
            }
            if critical.len() > 10 {
                println!("  ... and {} more", critical.len() - 10);
            }
            println!();
        }

        if !high.is_empty() {
            println!("HIGH - Events without tests ({}):", high.len());
            for gap in high.iter().take(10) {
                println!("  ⚠️  {} ({})", gap.target, gap.location);
                println!("     {}", gap.recommendation);
            }
            if high.len() > 10 {
                println!("  ... and {} more", high.len() - 10);
            }
            println!();
        }

        if !medium.is_empty() {
            println!("MEDIUM - Exports without tests ({}):", medium.len());
            for gap in medium.iter().take(5) {
                println!("  📦 {} ({})", gap.target, gap.location);
            }
            if medium.len() > 5 {
                println!("  ... and {} more", medium.len() - 5);
            }
            println!();
        }

        if !low.is_empty() {
            println!("LOW - Tested but unused ({}):", low.len());
            for gap in low.iter().take(5) {
                println!("  🧪 {} ({})", gap.target, gap.location);
            }
            if low.len() > 5 {
                println!("  ... and {} more", low.len() - 5);
            }
            println!();
        }

        // Summary
        let handler_count = gaps
            .iter()
            .filter(|g| matches!(g.kind, GapKind::HandlerWithoutTest))
            .count();
        let event_count = gaps
            .iter()
            .filter(|g| matches!(g.kind, GapKind::EventWithoutTest))
            .count();
        println!(
            "Summary: {} handlers, {} events without test coverage",
            handler_count, event_count
        );
        println!("\nRun `loct coverage --json` for machine-readable output.");
    }

    DispatchResult::Exit(0)
}

/// Handle the jq query command - execute jaq filter on snapshot
fn handle_jq_query_command(
    opts: &super::command::JqQueryOptions,
    _global: &GlobalOptions,
) -> DispatchResult {
    use crate::jaq_query::{JaqExecutor, format_output};
    use crate::snapshot::Snapshot;

    // Find snapshot path
    let snapshot_path = match Snapshot::find_latest_snapshot(opts.snapshot_path.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[loct][error] {}", e);
            eprintln!("[loct][hint] Run 'loct scan' first to create a snapshot.");
            return DispatchResult::Exit(1);
        }
    };

    // Load snapshot from file directly
    // snapshot_path is like .loctree/branch@commit/snapshot.json
    // We need to read it directly, not through Snapshot::load which expects project root
    let snapshot_json = match std::fs::read_to_string(&snapshot_path) {
        Ok(content) => match serde_json::from_str::<crate::snapshot::Snapshot>(&content) {
            Ok(snap) => match serde_json::to_value(&snap) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize snapshot: {}", e);
                    return DispatchResult::Exit(1);
                }
            },
            Err(e) => {
                eprintln!("[loct][error] Failed to parse snapshot: {}", e);
                return DispatchResult::Exit(1);
            }
        },
        Err(e) => {
            eprintln!("[loct][error] Failed to read snapshot file: {}", e);
            return DispatchResult::Exit(1);
        }
    };

    // Execute the jaq filter
    let executor = JaqExecutor::new();
    let results = match executor.execute(
        &opts.filter,
        &snapshot_json,
        &opts.string_args,
        &opts.json_args,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[loct][error] Filter execution failed: {}", e);
            return DispatchResult::Exit(1);
        }
    };

    // Output results
    for result in &results {
        let output = format_output(result, opts.raw_output, opts.compact_output);
        println!("{}", output);
    }

    // Exit status mode: exit 1 if no results or all results are false/null
    if opts.exit_status {
        if results.is_empty() {
            return DispatchResult::Exit(1);
        }

        // Check if all results are false or null
        let all_false_or_null = results
            .iter()
            .all(|v| v.is_null() || (v.as_bool().is_some() && !v.as_bool().unwrap()));

        if all_false_or_null {
            return DispatchResult::Exit(1);
        }
    }

    DispatchResult::Exit(0)
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
