//! Watch-related command handlers
//!
//! Handles: scan_watch, coverage

use std::path::PathBuf;

use super::super::super::command::{Command, CoverageOptions, ScanOptions};
use super::super::{
    DispatchResult, GlobalOptions, command_to_parsed_args, load_or_create_snapshot,
};
use crate::progress::Spinner;

/// Handle the scan command with watch mode
pub fn handle_scan_watch_command(opts: &ScanOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::detect::apply_detected_stack;
    use crate::fs_utils::GitIgnoreChecker;
    use crate::watch::{WatchConfig, watch_and_rescan};
    use std::time::Duration;

    // Build ParsedArgs for scanning
    let mut parsed_args = command_to_parsed_args(&Command::Scan(opts.clone()), global);

    // Auto-detect stack if first root exists
    let roots = if opts.roots.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        opts.roots.clone()
    };

    if let Some(root) = roots.first() {
        let mut library_mode = parsed_args.library_mode;
        apply_detected_stack(
            root,
            &mut parsed_args.extensions,
            &mut parsed_args.ignore_patterns,
            &mut parsed_args.tauri_preset,
            &mut library_mode,
            &mut parsed_args.py_roots,
            parsed_args.verbose,
        );
        parsed_args.library_mode = library_mode;
    }

    // Build gitignore checker
    let gitignore = if parsed_args.use_gitignore
        && let Some(root) = roots.first()
    {
        GitIgnoreChecker::new(root)
    } else {
        None
    };

    // Convert extensions from HashSet to Vec
    let extensions = parsed_args
        .extensions
        .as_ref()
        .map(|set| set.iter().cloned().collect::<Vec<String>>());

    // Build watch config
    let config = WatchConfig {
        roots,
        debounce_duration: Duration::from_millis(500),
        extensions,
        gitignore,
    };

    // Start watching
    match watch_and_rescan(config, &parsed_args) {
        Ok(_) => DispatchResult::Exit(0),
        Err(e) => {
            eprintln!("[watch] Error: {}", e);
            DispatchResult::Exit(1)
        }
    }
}

/// Handle the coverage command - analyze test coverage gaps
pub fn handle_coverage_command(opts: &CoverageOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::coverage_gaps::{GapKind, Severity, find_coverage_gaps};
    use crate::analyzer::test_coverage::{CoverageStatus, analyze_test_coverage};
    use std::path::Path;

    let include_gaps = opts.gaps
        || !opts.tests
        || opts.handlers_only
        || opts.events_only
        || opts.min_severity.is_some();
    let include_tests = opts.tests;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        let label = if include_gaps && include_tests {
            "Analyzing coverage (gaps + tests)..."
        } else if include_tests {
            "Analyzing structural test coverage..."
        } else {
            "Analyzing test coverage gaps..."
        };
        Some(Spinner::new(label))
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
    let gaps = if include_gaps {
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
        gaps
    } else {
        Vec::new()
    };

    let test_report = if include_tests {
        Some(analyze_test_coverage(&snapshot))
    } else {
        None
    };

    if let Some(s) = spinner {
        if include_gaps && include_tests {
            s.finish_success(&format!(
                "Found {} gap(s), coverage {:.1}%",
                gaps.len(),
                test_report
                    .as_ref()
                    .map(|r| r.coverage_percent)
                    .unwrap_or(0.0)
            ));
        } else if include_tests {
            s.finish_success(&format!(
                "Coverage {:.1}% ({} test file(s))",
                test_report
                    .as_ref()
                    .map(|r| r.coverage_percent)
                    .unwrap_or(0.0),
                test_report.as_ref().map(|r| r.test_file_count).unwrap_or(0)
            ));
        } else {
            s.finish_success(&format!("Found {} coverage gap(s)", gaps.len()));
        }
    }

    // Output results
    if global.json {
        if include_gaps && include_tests {
            let combined = serde_json::json!({
                "gaps": gaps,
                "tests": test_report,
            });
            match serde_json::to_string_pretty(&combined) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize coverage output: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        } else if include_tests {
            match serde_json::to_string_pretty(&test_report) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize test coverage: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        } else {
            match serde_json::to_string_pretty(&gaps) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("[loct][error] Failed to serialize coverage gaps: {}", e);
                    return DispatchResult::Exit(1);
                }
            }
        }
    } else {
        if include_tests {
            if let Some(report) = &test_report {
                let missing_tests: Vec<_> = report
                    .handlers
                    .iter()
                    .filter(|h| h.coverage_status == CoverageStatus::MissingTests)
                    .collect();
                let test_only: Vec<_> = report
                    .handlers
                    .iter()
                    .filter(|h| h.coverage_status == CoverageStatus::TestOnly)
                    .collect();
                let uncovered: Vec<_> = report
                    .handlers
                    .iter()
                    .filter(|h| h.coverage_status == CoverageStatus::Uncovered)
                    .collect();

                println!("Structural Test Coverage:");
                println!("  Test files:   {}", report.test_file_count);
                println!("  Prod files:   {}", report.prod_file_count);
                println!("  Coverage:     {:.1}%", report.coverage_percent);
                println!(
                    "  Handlers:     {} missing tests, {} test-only, {} uncovered",
                    missing_tests.len(),
                    test_only.len(),
                    uncovered.len()
                );
                println!(
                    "  Exports w/o tests: {}",
                    report.exports_without_tests.len()
                );

                if !missing_tests.is_empty() {
                    println!("\nHandlers missing tests ({}):", missing_tests.len());
                    for handler in missing_tests.iter().take(10) {
                        println!(
                            "  [!] {} ({}:{})",
                            handler.name,
                            handler.backend_file.display(),
                            handler.line
                        );
                    }
                    if missing_tests.len() > 10 {
                        println!("  ... and {} more", missing_tests.len() - 10);
                    }
                }
                if !report.exports_without_tests.is_empty() {
                    println!(
                        "\nExports without tests ({}):",
                        report.exports_without_tests.len()
                    );
                    for export in report.exports_without_tests.iter().take(10) {
                        println!(
                            "  [?] {} ({}:{})",
                            export.symbol,
                            export.defined_in.display(),
                            export.line
                        );
                    }
                    if report.exports_without_tests.len() > 10 {
                        println!("  ... and {} more", report.exports_without_tests.len() - 10);
                    }
                }
            }
            if include_gaps {
                println!();
            }
        }

        if include_gaps {
            if gaps.is_empty() {
                println!("[OK] No coverage gaps found - all production code is tested!");
                return DispatchResult::Exit(0);
            }
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
                    println!("  [!!] {} ({})", gap.target, gap.location);
                    println!("       {}", gap.recommendation);
                }
                if critical.len() > 10 {
                    println!("  ... and {} more", critical.len() - 10);
                }
                println!();
            }

            if !high.is_empty() {
                println!("HIGH - Events without tests ({}):", high.len());
                for gap in high.iter().take(10) {
                    println!("  [!] {} ({})", gap.target, gap.location);
                    println!("      {}", gap.recommendation);
                }
                if high.len() > 10 {
                    println!("  ... and {} more", high.len() - 10);
                }
                println!();
            }

            if !medium.is_empty() {
                println!("MEDIUM - Exports without tests ({}):", medium.len());
                for gap in medium.iter().take(5) {
                    println!("  [?] {} ({})", gap.target, gap.location);
                }
                if medium.len() > 5 {
                    println!("  ... and {} more", medium.len() - 5);
                }
                println!();
            }

            if !low.is_empty() {
                println!("LOW - Tested but unused ({}):", low.len());
                for gap in low.iter().take(5) {
                    println!("  [-] {} ({})", gap.target, gap.location);
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
    }

    DispatchResult::Exit(0)
}
