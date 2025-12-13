//! AI and code analysis command handlers
//!
//! Handles: memex, crowd, twins, sniff

use super::super::super::command::{CrowdOptions, MemexOptions, SniffOptions, TwinsOptions};
use super::super::{DispatchResult, GlobalOptions, is_test_file, load_or_create_snapshot};
use crate::progress::Spinner;

/// Handle the memex command - index analysis into AI memory
#[cfg(feature = "memex")]
pub fn handle_memex_command(opts: &MemexOptions, global: &GlobalOptions) -> DispatchResult {
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
pub fn handle_memex_command(_opts: &MemexOptions, _global: &GlobalOptions) -> DispatchResult {
    eprintln!(
        "[loct][memex][error] memex feature not enabled. Rebuild with: cargo build --features memex"
    );
    DispatchResult::Exit(1)
}

/// Handle the crowd command - detect functional crowds
pub fn handle_crowd_command(opts: &CrowdOptions, global: &GlobalOptions) -> DispatchResult {
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

pub fn handle_twins_command(opts: &TwinsOptions, global: &GlobalOptions) -> DispatchResult {
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
pub fn handle_sniff_command(opts: &SniffOptions, global: &GlobalOptions) -> DispatchResult {
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
