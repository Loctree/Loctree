//! Query-related command handlers
//!
//! Handles: query, jq_query

use super::super::super::command::{JqQueryOptions, QueryKind, QueryOptions};
use super::super::{DispatchResult, GlobalOptions, load_or_create_snapshot};

/// Handle the query command directly
pub fn handle_query_command(opts: &QueryOptions, global: &GlobalOptions) -> DispatchResult {
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

/// Handle the jq query command - execute jaq filter on snapshot
pub fn handle_jq_query_command(opts: &JqQueryOptions, _global: &GlobalOptions) -> DispatchResult {
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
