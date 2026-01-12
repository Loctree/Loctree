//! Output-related command handlers
//!
//! Handles: lint, dist

use super::super::super::command::{DistOptions, LintOptions};
use super::super::{DispatchResult, GlobalOptions, load_or_create_snapshot};
use crate::progress::Spinner;

/// Handle the lint command - run linting checks
pub fn handle_lint_command(opts: &LintOptions, global: &GlobalOptions) -> DispatchResult {
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
pub fn handle_dist_command(opts: &DistOptions, global: &GlobalOptions) -> DispatchResult {
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
