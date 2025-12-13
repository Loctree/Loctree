//! Analysis-related command handlers
//!
//! Handles: dead, cycles, commands, events, routes

use super::super::super::command::{
    CommandsOptions, CyclesOptions, DeadOptions, EventsOptions, RoutesOptions,
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
