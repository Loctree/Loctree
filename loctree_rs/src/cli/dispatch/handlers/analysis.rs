//! Analysis-related command handlers
//!
//! Handles: dead, cycles, commands, events, routes, zombie

use super::super::super::command::{
    CommandsOptions, CyclesOptions, DeadOptions, EventsOptions, FocusOptions, HealthOptions,
    HotspotsOptions, LayoutmapOptions, RoutesOptions, ZombieOptions,
};
use super::super::{DispatchResult, GlobalOptions, load_or_create_snapshot};
use crate::progress::Spinner;

/// Handle the dead command - detect dead exports
pub fn handle_dead_command(opts: &DeadOptions, global: &GlobalOptions) -> DispatchResult {
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
pub fn handle_cycles_command(opts: &CyclesOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::cycles::{
        CycleCompilability, find_cycles_classified_with_lazy, print_cycles_classified,
        print_cycles_classified_legacy,
    };
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
    let (mut classified_cycles, classified_lazy_cycles) = find_cycles_classified_with_lazy(&edges);

    // Filter to breaking-only if requested
    if opts.breaking_only {
        classified_cycles.retain(|c| c.compilability == CycleCompilability::Breaking);
    }

    // Count by compilability for spinner message
    let breaking_count = classified_cycles
        .iter()
        .filter(|c| c.compilability == CycleCompilability::Breaking)
        .count();
    let structural_count = classified_cycles
        .iter()
        .filter(|c| c.compilability == CycleCompilability::Structural)
        .count();
    let diamond_count = classified_cycles
        .iter()
        .filter(|c| c.compilability == CycleCompilability::DiamondDependency)
        .count();

    if let Some(s) = spinner {
        if opts.breaking_only {
            s.finish_success(&format!(
                "Found {} breaking cycle(s) (filtered from {} total)",
                breaking_count,
                breaking_count + structural_count + diamond_count
            ));
        } else {
            s.finish_success(&format!(
                "Found {} cycle(s) ({} breaking, {} structural, {} diamond)",
                classified_cycles.len(),
                breaking_count,
                structural_count,
                diamond_count
            ));
        }
    }

    // Output results
    let json_output = global.json;

    if opts.legacy_format {
        print_cycles_classified_legacy(&classified_cycles, json_output);
    } else {
        print_cycles_classified(&classified_cycles, json_output);
    }

    if !classified_lazy_cycles.is_empty() && !json_output && !opts.breaking_only {
        println!("\nLazy circular imports (info):");
        println!(
            "  Detected via imports inside functions/methods; usually safe but review if init order matters."
        );
        if opts.legacy_format {
            print_cycles_classified_legacy(&classified_lazy_cycles, false);
        } else {
            print_cycles_classified(&classified_lazy_cycles, false);
        }

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

    // Exit code: 1 if there are breaking cycles (for CI use)
    if breaking_count > 0 && opts.breaking_only {
        DispatchResult::Exit(1)
    } else {
        DispatchResult::Exit(0)
    }
}

/// Handle the commands command - show Tauri command bridges
pub fn handle_commands_command(opts: &CommandsOptions, global: &GlobalOptions) -> DispatchResult {
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
                        "    [!] Why: Frontend calls invoke('{}') but no #[tauri::command] found in Rust.",
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
                        "    [i] Why: #[tauri::command] defined but no invoke('{}') calls found in frontend.",
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
pub fn handle_events_command(opts: &EventsOptions, global: &GlobalOptions) -> DispatchResult {
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
        // Group events by pattern
        let fe_sync_events: Vec<_> = snapshot
            .event_bridges
            .iter()
            .filter(|e| e.is_fe_sync)
            .collect();
        let other_events: Vec<_> = snapshot
            .event_bridges
            .iter()
            .filter(|e| !e.is_fe_sync)
            .collect();

        // If --fe-sync flag, only show FE↔FE events
        if opts.fe_sync {
            if fe_sync_events.is_empty() {
                println!("No FE↔FE sync events found");
            } else {
                println!("FE↔FE Sync Events ({}):", fe_sync_events.len());
                println!("  (Window sync pattern: emit and listen both in frontend)\n");

                for event in &fe_sync_events {
                    println!("  Event: {}", event.name);

                    if event.same_file_sync {
                        println!("    Pattern: Same-file sync (emit+listen in same file)");
                    }

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

                    println!();
                }
            }
        } else {
            // Show all events, with FE↔FE sync clearly marked
            if snapshot.event_bridges.is_empty() {
                println!("No event bridges found");
            } else {
                println!("Event Bridges Analysis:\n");

                // Show FE↔FE sync events first if any exist
                if !fe_sync_events.is_empty() {
                    println!("FE↔FE Sync Events ({}):", fe_sync_events.len());
                    println!("  (Window sync: emit+listen both in frontend, not orphans)\n");

                    for event in &fe_sync_events {
                        println!(
                            "  {} {}",
                            event.name,
                            if event.same_file_sync {
                                "(same file)"
                            } else {
                                ""
                            }
                        );

                        if !event.emits.is_empty() {
                            println!(
                                "    Emit: {}",
                                event
                                    .emits
                                    .iter()
                                    .map(|(f, l, _)| format!("{}:{}", f, l))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                        }

                        if !event.listens.is_empty() {
                            println!(
                                "    Listen: {}",
                                event
                                    .listens
                                    .iter()
                                    .map(|(f, l)| format!("{}:{}", f, l))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                        }

                        println!();
                    }
                }

                // Show other events
                if !other_events.is_empty() {
                    if !fe_sync_events.is_empty() {
                        println!("Other Events ({}):\n", other_events.len());
                    } else {
                        println!("Found {} event bridge(s):\n", other_events.len());
                    }

                    for event in &other_events {
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

                        // Highlight potential issues (not FE↔FE sync)
                        if event.emits.is_empty() {
                            println!("    [!] No emitters found (orphan listener?)");
                        }
                        if event.listens.is_empty() {
                            println!("    [!] No listeners found (orphan emitter?)");
                        }

                        println!();
                    }
                }
            }
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the routes command - list backend/web routes (FastAPI/Flask)
pub fn handle_routes_command(opts: &RoutesOptions, global: &GlobalOptions) -> DispatchResult {
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

/// Handle the focus command - extract holographic context for a directory
pub fn handle_focus_command(opts: &FocusOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::focuser::{FocusConfig, HolographicFocus};
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing directory..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts.root.as_deref().unwrap_or(Path::new("."));

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

    let config = FocusConfig {
        include_consumers: opts.consumers,
        max_depth: opts.depth.unwrap_or(2),
    };

    let focus = match HolographicFocus::from_path(&snapshot, &opts.target, &config) {
        Some(f) => f,
        None => {
            if let Some(s) = spinner {
                s.finish_error(&format!("No files found in directory '{}'", opts.target));
            } else {
                eprintln!();
                eprintln!("No files found in directory '{}'.", opts.target);
                eprintln!();
                eprintln!("   Possible causes:");
                eprintln!("   - Directory path is incorrect");
                eprintln!("   - Directory was added after last snapshot (run `loctree` to update)");
                eprintln!("   - All files in directory are excluded by .gitignore");
            }
            return DispatchResult::Exit(1);
        }
    };

    if let Some(s) = spinner {
        s.finish_success(&format!(
            "Found {} files in {}",
            focus.stats.core_files, opts.target
        ));
    }

    // Output results
    if global.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&focus.to_json()).unwrap_or_default()
        );
    } else {
        focus.print();
    }

    DispatchResult::Exit(0)
}

/// Handle the hotspots command - show import frequency heatmap
pub fn handle_hotspots_command(opts: &HotspotsOptions, global: &GlobalOptions) -> DispatchResult {
    use std::collections::HashMap;
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing import hotspots..."))
    } else {
        None
    };

    // Load snapshot (auto-scan if missing)
    let root = opts.root.as_deref().unwrap_or(Path::new("."));

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

    // Calculate in-degree (how many files import this file) and out-degree (how many files this imports)
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut out_degree: HashMap<String, usize> = HashMap::new();

    // Initialize all files with 0
    for file in &snapshot.files {
        in_degree.insert(file.path.clone(), 0);
        out_degree.insert(file.path.clone(), 0);
    }

    // Count edges
    for edge in &snapshot.edges {
        *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
        *out_degree.entry(edge.from.clone()).or_insert(0) += 1;
    }

    // Build list of (path, in_degree, out_degree)
    let mut hotspots: Vec<(String, usize, usize)> = in_degree
        .iter()
        .map(|(path, &in_deg)| {
            let out_deg = out_degree.get(path).copied().unwrap_or(0);
            (path.clone(), in_deg, out_deg)
        })
        .collect();

    // Filter
    let min_imports = opts.min_imports.unwrap_or(0);
    if opts.leaves_only {
        hotspots.retain(|(_, in_deg, _)| *in_deg == 0);
    } else if min_imports > 0 {
        hotspots.retain(|(_, in_deg, _)| *in_deg >= min_imports);
    }

    // Sort by in-degree (descending)
    hotspots.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Apply limit
    let limit = opts.limit.unwrap_or(50);
    if hotspots.len() > limit {
        hotspots.truncate(limit);
    }

    if let Some(s) = spinner {
        s.finish_success(&format!("Analyzed {} files", snapshot.files.len()));
    }

    // Output
    if global.json {
        let json_output: Vec<serde_json::Value> = hotspots
            .iter()
            .map(|(path, in_deg, out_deg)| {
                let category = match *in_deg {
                    n if n >= 10 => "CORE",
                    n if n >= 3 => "SHARED",
                    n if n >= 1 => "PERIPHERAL",
                    _ => "LEAF",
                };
                serde_json::json!({
                    "path": path,
                    "in_degree": in_deg,
                    "out_degree": out_deg,
                    "category": category
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_output).unwrap_or_default()
        );
    } else {
        println!();
        println!("Import Hotspots ({} files analyzed)", snapshot.files.len());
        println!();

        // Group by category
        let core: Vec<_> = hotspots
            .iter()
            .filter(|(_, in_deg, _)| *in_deg >= 10)
            .collect();
        let shared: Vec<_> = hotspots
            .iter()
            .filter(|(_, in_deg, _)| *in_deg >= 3 && *in_deg < 10)
            .collect();
        let peripheral: Vec<_> = hotspots
            .iter()
            .filter(|(_, in_deg, _)| *in_deg >= 1 && *in_deg < 3)
            .collect();
        let leaves: Vec<_> = hotspots
            .iter()
            .filter(|(_, in_deg, _)| *in_deg == 0)
            .collect();

        if !core.is_empty() {
            println!("CORE (10+ importers):");
            for (path, in_deg, out_deg) in &core {
                if opts.coupling {
                    println!("  [in:{:<3} out:{:<3}] {}", in_deg, out_deg, path);
                } else {
                    println!("  [{:>3}] {}", in_deg, path);
                }
            }
            println!();
        }

        if !shared.is_empty() {
            println!("SHARED (3-9 importers):");
            for (path, in_deg, out_deg) in &shared {
                if opts.coupling {
                    println!("  [in:{:<3} out:{:<3}] {}", in_deg, out_deg, path);
                } else {
                    println!("  [{:>3}] {}", in_deg, path);
                }
            }
            println!();
        }

        if !peripheral.is_empty() {
            println!("PERIPHERAL (1-2 importers):");
            for (path, in_deg, out_deg) in &peripheral {
                if opts.coupling {
                    println!("  [in:{:<3} out:{:<3}] {}", in_deg, out_deg, path);
                } else {
                    println!("  [{:>3}] {}", in_deg, path);
                }
            }
            println!();
        }

        if !leaves.is_empty() {
            println!("LEAF (0 importers):");
            for (path, _, out_deg) in &leaves {
                if opts.coupling {
                    println!("  [in:0   out:{:<3}] {}", out_deg, path);
                } else {
                    println!("        {}", path);
                }
            }
            println!();
        }

        if hotspots.is_empty() {
            println!("  No files match the filter criteria.");
            println!();
        }

        // Summary
        println!(
            "Showing {} of {} files (--limit {})",
            hotspots.len(),
            snapshot.files.len(),
            limit
        );
        if opts.leaves_only {
            println!("Filtered to leaf nodes only (--leaves)");
        } else if min_imports > 0 {
            println!("Filtered to files with {} + importers (--min)", min_imports);
        }
    }

    DispatchResult::Exit(0)
}

/// Handle the layoutmap command - CSS z-index/sticky/grid analysis
pub fn handle_layoutmap_command(opts: &LayoutmapOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::layoutmap::scan_css_layout;
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Analyzing CSS layout properties..."))
    } else {
        None
    };

    let root = opts.root.as_deref().unwrap_or(Path::new("."));

    // Scan CSS files
    let findings = match scan_css_layout(root, opts) {
        Ok(f) => f,
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_error(&format!("Failed to scan CSS: {}", e));
            } else {
                eprintln!("[loct][error] {}", e);
            }
            return DispatchResult::Exit(1);
        }
    };

    if let Some(s) = spinner {
        s.finish_success(&format!("Found {} layout findings", findings.len()));
    }

    // Output
    if global.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&findings).unwrap_or_default()
        );
    } else {
        print_layoutmap_human(&findings, opts);
    }

    DispatchResult::Exit(0)
}

fn print_layoutmap_human(findings: &[crate::layoutmap::LayoutFinding], opts: &LayoutmapOptions) {
    use crate::layoutmap::LayoutFinding;

    if findings.is_empty() {
        println!("\nNo CSS layout findings detected.\n");
        return;
    }

    // Group by type
    let zindex: Vec<_> = findings
        .iter()
        .filter(|f| matches!(f, LayoutFinding::ZIndex { .. }))
        .collect();
    let sticky: Vec<_> = findings
        .iter()
        .filter(|f| matches!(f, LayoutFinding::Sticky { .. }))
        .collect();
    let grid: Vec<_> = findings
        .iter()
        .filter(|f| matches!(f, LayoutFinding::Grid { .. }))
        .collect();
    let flex: Vec<_> = findings
        .iter()
        .filter(|f| matches!(f, LayoutFinding::Flex { .. }))
        .collect();

    println!();

    // Z-Index section
    if !opts.sticky_only && !opts.grid_only && !zindex.is_empty() {
        println!("Z-INDEX LAYERS (sorted by z-index):");
        let mut zindex_sorted: Vec<_> = zindex.iter().collect();
        zindex_sorted.sort_by(|a, b| {
            let za = match a {
                LayoutFinding::ZIndex { z_index, .. } => *z_index,
                _ => 0,
            };
            let zb = match b {
                LayoutFinding::ZIndex { z_index, .. } => *z_index,
                _ => 0,
            };
            zb.cmp(&za)
        });

        for finding in zindex_sorted {
            if let LayoutFinding::ZIndex {
                file,
                line,
                selector,
                z_index,
            } = finding
            {
                println!(
                    "  z-index: {:>6}  {}  ({}:{})",
                    z_index, selector, file, line
                );
            }
        }
        println!();
    }

    // Sticky section
    if !opts.zindex_only && !opts.grid_only && !sticky.is_empty() {
        println!("STICKY/FIXED ELEMENTS:");
        for finding in &sticky {
            if let LayoutFinding::Sticky {
                file,
                line,
                selector,
                position,
            } = finding
            {
                println!("  {} {:>6}  ({}:{})", selector, position, file, line);
            }
        }
        println!();
    }

    // Grid section
    if !opts.zindex_only && !opts.sticky_only && !grid.is_empty() {
        println!("CSS GRID CONTAINERS:");
        for finding in &grid {
            if let LayoutFinding::Grid {
                file,
                line,
                selector,
            } = finding
            {
                println!("  {}  ({}:{})", selector, file, line);
            }
        }
        println!();
    }

    // Flex section (only if not filtering)
    if !opts.zindex_only && !opts.sticky_only && !opts.grid_only && !flex.is_empty() {
        println!("FLEX CONTAINERS:");
        for finding in &flex {
            if let LayoutFinding::Flex {
                file,
                line,
                selector,
            } = finding
            {
                println!("  {}  ({}:{})", selector, file, line);
            }
        }
        println!();
    }

    // Summary
    println!(
        "Total: {} z-index, {} sticky/fixed, {} grid, {} flex",
        zindex.len(),
        sticky.len(),
        grid.len(),
        flex.len()
    );
}
/// Handle the zombie command - find all zombie code
pub fn handle_zombie_command(opts: &ZombieOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports};
    use crate::analyzer::twins::{build_symbol_registry, detect_exact_twins};
    use std::collections::HashMap;
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Hunting for zombie code..."))
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

    // 1. Find dead exports
    let dead_exports = find_dead_exports(
        &snapshot.files,
        false,
        None,
        DeadFilterConfig {
            include_tests: opts.include_tests,
            include_helpers: false,
            library_mode: global.library_mode,
            example_globs: Vec::new(),
            python_library_mode: global.python_library,
        },
    );

    // 2. Find orphan files (files with 0 importers)
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Initialize all files with 0
    for file in &snapshot.files {
        in_degree.insert(file.path.clone(), 0);
    }

    // Count edges
    for edge in &snapshot.edges {
        *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
    }

    // Filter to orphan files (0 importers, non-entry-points, non-tests unless requested)
    let mut orphan_files: Vec<(String, usize)> = in_degree
        .iter()
        .filter(|(path, count)| {
            if **count > 0 {
                return false;
            }
            // Skip entry points
            if is_entry_point(path.as_str()) {
                return false;
            }
            // Skip tests unless --include-tests
            if !opts.include_tests && is_test_file_path(path.as_str()) {
                return false;
            }
            true
        })
        .map(|(path, _)| {
            let loc = snapshot
                .files
                .iter()
                .find(|f| &f.path == path)
                .map(|f| f.loc)
                .unwrap_or(0);
            (path.clone(), loc)
        })
        .collect();

    // Sort by LOC descending (biggest files first - most impact)
    orphan_files.sort_by(|a, b| b.1.cmp(&a.1));

    // 3. Find shadow exports (same symbol exported by multiple files where some have 0 imports)
    let twins = detect_exact_twins(&snapshot.files, opts.include_tests);
    let registry = build_symbol_registry(&snapshot.files, opts.include_tests);

    // Shadow exports: twins where at least one location has 0 imports but not all
    let mut shadow_exports: Vec<(String, usize, usize)> = Vec::new(); // (symbol, total_locations, dead_locations)

    for twin in &twins {
        let mut total_locations = 0;
        let mut dead_count = 0;

        for loc in &twin.locations {
            total_locations += 1;
            let key = (loc.file_path.clone(), twin.name.clone());
            if let Some(entry) = registry.get(&key)
                && entry.import_count == 0
            {
                dead_count += 1;
            }
        }

        // Shadow if: multiple locations, at least one dead, not all dead
        if total_locations >= 2 && dead_count > 0 && dead_count < total_locations {
            shadow_exports.push((twin.name.clone(), total_locations, dead_count));
        }
    }

    // Calculate total LOC for orphan files
    let orphan_loc: usize = orphan_files.iter().map(|(_, loc)| loc).sum();

    if let Some(s) = spinner {
        s.finish_success(&format!(
            "Found {} dead exports, {} orphan files, {} shadow exports",
            dead_exports.len(),
            orphan_files.len(),
            shadow_exports.len()
        ));
    }

    // Output results
    if global.json {
        let json = serde_json::json!({
            "dead_exports": dead_exports.iter().map(|d| {
                serde_json::json!({
                    "file": d.file,
                    "line": d.line,
                    "symbol": d.symbol,
                    "confidence": d.confidence
                })
            }).collect::<Vec<_>>(),
            "orphan_files": orphan_files.iter().map(|(path, loc)| {
                serde_json::json!({
                    "path": path,
                    "loc": loc
                })
            }).collect::<Vec<_>>(),
            "shadow_exports": shadow_exports.iter().map(|(symbol, total, dead)| {
                serde_json::json!({
                    "symbol": symbol,
                    "total_locations": total,
                    "dead_locations": dead
                })
            }).collect::<Vec<_>>(),
            "summary": {
                "dead_exports_count": dead_exports.len(),
                "orphan_files_count": orphan_files.len(),
                "orphan_files_loc": orphan_loc,
                "shadow_exports_count": shadow_exports.len(),
                "total_zombie_items": dead_exports.len() + orphan_files.len() + shadow_exports.len()
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
    } else {
        // Human-readable output
        println!();
        println!("🧟 Zombie Code Report");
        println!();

        // Dead Exports section
        println!("Dead Exports ({}):", dead_exports.len());
        if dead_exports.is_empty() {
            println!("  (none)");
        } else {
            for (i, dead) in dead_exports.iter().take(10).enumerate() {
                let line_str = dead
                    .line
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "  {}:{}  {} [{}]",
                    dead.file, line_str, dead.symbol, dead.confidence
                );
                if i == 9 && dead_exports.len() > 10 {
                    println!("  ... and {} more", dead_exports.len() - 10);
                }
            }
        }
        println!();

        // Orphan Files section
        println!(
            "Orphan Files (0 importers, {} files, {} LOC):",
            orphan_files.len(),
            orphan_loc
        );
        if orphan_files.is_empty() {
            println!("  (none)");
        } else {
            for (i, (path, loc)) in orphan_files.iter().take(10).enumerate() {
                println!("  {} ({} LOC)", path, loc);
                if i == 9 && orphan_files.len() > 10 {
                    println!("  ... and {} more", orphan_files.len() - 10);
                }
            }
        }
        println!();

        // Shadow Exports section
        println!("Shadow Exports ({}):", shadow_exports.len());
        if shadow_exports.is_empty() {
            println!("  (none)");
        } else {
            for (symbol, total, dead) in &shadow_exports {
                println!("  {} exported by {} files, {} dead", symbol, total, dead);
            }
        }
        println!();

        // Summary
        let total_items = dead_exports.len() + orphan_files.len() + shadow_exports.len();
        println!(
            "Total: {} zombie items, ~{} LOC to review",
            total_items, orphan_loc
        );
        println!();
    }

    DispatchResult::Exit(0)
}

/// Check if a file is an entry point
fn is_entry_point(path: &str) -> bool {
    path.ends_with("/main.rs")
        || path.ends_with("/lib.rs")
        || path.ends_with("/main.ts")
        || path.ends_with("/main.tsx")
        || path.ends_with("/main.js")
        || path.ends_with("/main.jsx")
        || path.ends_with("/index.ts")
        || path.ends_with("/index.tsx")
        || path.ends_with("/index.js")
        || path.ends_with("/index.jsx")
        || path.ends_with("/App.tsx")
        || path.ends_with("/App.jsx")
        || path.ends_with("/_app.tsx")
        || path.ends_with("/_app.jsx")
        || path.ends_with("/__init__.py")
        || path == "main.rs"
        || path == "lib.rs"
        || path == "main.ts"
        || path == "index.ts"
}

/// Check if a file path looks like a test file
fn is_test_file_path(path: &str) -> bool {
    path.contains("/test/")
        || path.contains("/tests/")
        || path.contains("/__tests__/")
        || path.contains("/spec/")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
        || path.ends_with(".test.js")
        || path.ends_with(".test.jsx")
        || path.ends_with(".spec.ts")
        || path.ends_with(".spec.tsx")
        || path.ends_with(".spec.js")
        || path.ends_with(".spec.jsx")
        || path.ends_with("_test.rs")
        || path.ends_with("_test.py")
        || path.starts_with("test_")
        || path.contains("/test_")
}

/// Handle the health command - quick summary of cycles + dead + twins
pub fn handle_health_command(opts: &HealthOptions, global: &GlobalOptions) -> DispatchResult {
    use crate::analyzer::cycles::{CycleCompilability, find_cycles_classified_with_lazy};
    use crate::analyzer::dead_parrots::{DeadFilterConfig, find_dead_exports};
    use crate::analyzer::twins::detect_exact_twins;
    use std::path::Path;

    // Show spinner unless in quiet/json mode
    let spinner = if !global.quiet && !global.json {
        Some(Spinner::new("Running health check..."))
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

    // 1. Cycles analysis
    let edges: Vec<(String, String, String)> = snapshot
        .edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
        .collect();

    let (classified_cycles, _) = find_cycles_classified_with_lazy(&edges);

    let hard_cycles = classified_cycles
        .iter()
        .filter(|c| c.compilability == CycleCompilability::Breaking)
        .count();
    let structural_cycles = classified_cycles
        .iter()
        .filter(|c| c.compilability == CycleCompilability::Structural)
        .count();
    let total_cycles = classified_cycles.len();

    // 2. Dead exports analysis
    let dead_exports = find_dead_exports(
        &snapshot.files,
        false,
        None,
        DeadFilterConfig {
            include_tests: opts.include_tests,
            include_helpers: false,
            library_mode: global.library_mode,
            example_globs: Vec::new(),
            python_library_mode: global.python_library,
        },
    );

    // Count by confidence
    let high_confidence = dead_exports
        .iter()
        .filter(|d| d.confidence == "high")
        .count();
    let low_confidence = dead_exports.len() - high_confidence;

    // 3. Twins analysis
    let twins = detect_exact_twins(&snapshot.files, opts.include_tests);
    let twin_count = twins.len();

    if let Some(s) = spinner {
        s.finish_success("Health check complete");
    }

    // Output results
    if global.json {
        let json = serde_json::json!({
            "cycles": {
                "total": total_cycles,
                "hard": hard_cycles,
                "structural": structural_cycles
            },
            "dead_exports": {
                "total": dead_exports.len(),
                "high_confidence": high_confidence,
                "low_confidence": low_confidence
            },
            "twins": {
                "total": twin_count
            }
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("\nHealth Check Summary\n");

        // Cycles
        if total_cycles == 0 {
            println!("Cycles:      OK (none detected)");
        } else {
            println!(
                "Cycles:      {} total ({} hard, {} structural)",
                total_cycles, hard_cycles, structural_cycles
            );
        }

        // Dead exports
        if dead_exports.is_empty() {
            println!("Dead:        OK (none detected)");
        } else {
            println!(
                "Dead:        {} high confidence, {} low",
                high_confidence, low_confidence
            );
        }

        // Twins
        if twin_count == 0 {
            println!("Twins:       OK (none detected)");
        } else {
            println!("Twins:       {} duplicate symbol groups", twin_count);
        }

        println!();
        println!("Run `loct cycles`, `loct dead`, `loct twins` for details.");
        println!();
    }

    DispatchResult::Exit(0)
}
