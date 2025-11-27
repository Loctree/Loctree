use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::args::{preset_ignore_symbols, ParsedArgs};
use crate::types::OutputMode;

use super::dead_parrots::{
    analyze_impact, find_dead_exports, find_similar, print_dead_exports, print_impact_results,
    print_similarity_results, print_symbol_results, search_symbol,
};
use super::open_server::{open_in_browser, start_open_server};
use super::output::{process_root_context, write_report, RootArtifacts};
use super::root_scan::{scan_roots, ScanConfig, ScanResults};
use super::scan::{opt_globset, python_stdlib};
use super::ReportSection;
use super::{coverage::compute_command_gaps, pipelines::build_pipeline_summary};

const DEFAULT_EXCLUDE_REPORT_PATTERNS: &[&str] =
    &["**/__tests__/**", "scripts/semgrep-fixtures/**"];

const SCHEMA_NAME: &str = "loctree-json";
const SCHEMA_VERSION: &str = "1.2.0";

pub fn default_analyzer_exts() -> HashSet<String> {
    ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn styles_preset_exts() -> HashSet<String> {
    [
        "css", "scss", "sass", "less", "ts", "tsx", "js", "jsx", "mjs", "cjs",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn run_import_analyzer(root_list: &[PathBuf], parsed: &ParsedArgs) -> io::Result<()> {
    let mut json_results = Vec::new();
    let mut report_sections: Vec<ReportSection> = Vec::new();
    let mut server_handle = None;
    let mut ignore_exact: HashSet<String> = HashSet::new();
    let mut ignore_prefixes: Vec<String> = Vec::new();

    if let Some(preset_name) = parsed.ignore_symbols_preset.as_deref() {
        if let Some(set) = preset_ignore_symbols(preset_name) {
            for s in set {
                if s.ends_with('*') {
                    ignore_prefixes.push(s.trim_end_matches('*').to_string());
                } else {
                    ignore_exact.insert(s);
                }
            }
        } else {
            eprintln!(
                "[loctree][warn] unknown --ignore-symbols-preset '{}', ignoring",
                preset_name
            );
        }
    }

    if let Some(user_syms) = parsed.ignore_symbols.clone() {
        for s in user_syms {
            let lc = s.to_lowercase();
            if lc.ends_with('*') {
                ignore_prefixes.push(lc.trim_end_matches('*').to_string());
            } else {
                ignore_exact.insert(lc);
            }
        }
    }

    let focus_set = opt_globset(&parsed.focus_patterns);
    let mut exclude_patterns = parsed.exclude_report_patterns.clone();
    exclude_patterns.extend(
        DEFAULT_EXCLUDE_REPORT_PATTERNS
            .iter()
            .map(|p| p.to_string()),
    );
    let exclude_set = opt_globset(&exclude_patterns);

    let editor_cfg = super::open_server::EditorConfig::from_args(
        parsed.editor_kind.clone(),
        parsed.editor_cmd.clone(),
    );

    if parsed.serve {
        if let Some((base, handle)) = start_open_server(
            root_list.to_vec(),
            editor_cfg.clone(),
            parsed.report_path.clone(),
            parsed.serve_port,
        ) {
            server_handle = Some(handle);
            eprintln!("[loctree] local open server at {}", base);
        } else {
            eprintln!("[loctree][warn] could not start open server; continue without --serve");
        }
    }

    let py_stdlib = python_stdlib();

    let base_extensions = parsed.extensions.clone().or_else(|| {
        if parsed.styles_preset {
            Some(styles_preset_exts())
        } else {
            Some(default_analyzer_exts())
        }
    });
    let scan_results = scan_roots(ScanConfig {
        roots: root_list,
        parsed,
        extensions: base_extensions,
        focus_set: &focus_set,
        exclude_set: &exclude_set,
        ignore_exact,
        ignore_prefixes,
        py_stdlib: &py_stdlib,
    })?;
    let ScanResults {
        contexts,
        global_fe_commands,
        global_be_commands,
        global_fe_payloads,
        global_be_payloads,
        global_analyses,
    } = scan_results;

    if let Some(sym) = &parsed.symbol {
        let result = search_symbol(sym, &global_analyses);
        print_symbol_results(sym, &result, matches!(parsed.output, OutputMode::Json));
        return Ok(());
    }

    if let Some(target_path) = &parsed.impact {
        if let Some(result) = analyze_impact(target_path, &global_analyses, &contexts) {
            print_impact_results(
                target_path,
                &result,
                matches!(parsed.output, OutputMode::Json),
            );
        } else {
            eprintln!("Target file not found in scan results: {}", target_path);
        }
        return Ok(());
    }

    if let Some(query) = &parsed.check_sim {
        let candidates = find_similar(query, &global_analyses);
        print_similarity_results(
            query,
            &candidates,
            matches!(parsed.output, OutputMode::Json),
        );
        return Ok(());
    }

    if parsed.dead_exports {
        let high_confidence = parsed.dead_confidence.as_deref() == Some("high");
        let dead_exports = find_dead_exports(&global_analyses, high_confidence);
        print_dead_exports(&dead_exports, parsed.output, high_confidence);
        return Ok(());
    }

    // Cross-root command gaps (fixes multi-root FP for missing/unused handlers)
    let (global_missing_handlers, global_unused_handlers) = compute_command_gaps(
        &global_fe_commands,
        &global_be_commands,
        &focus_set,
        &exclude_set,
    );

    let pipeline_summary = build_pipeline_summary(
        &global_analyses,
        &focus_set,
        &exclude_set,
        &global_fe_commands,
        &global_be_commands,
        &global_fe_payloads,
        &global_be_payloads,
    );

    for (idx, ctx) in contexts.into_iter().enumerate() {
        let RootArtifacts {
            json_items,
            report_section,
        } = process_root_context(
            idx,
            ctx,
            parsed,
            &global_fe_commands,
            &global_be_commands,
            &global_missing_handlers,
            &global_unused_handlers,
            &pipeline_summary,
            SCHEMA_NAME,
            SCHEMA_VERSION,
        );
        json_results.extend(json_items);
        if let Some(section) = report_section {
            report_sections.push(section);
        }
    }

    if matches!(parsed.output, OutputMode::Json) {
        let payload = if json_results.len() == 1 {
            serde_json::to_string_pretty(&json_results[0])
        } else {
            serde_json::to_string_pretty(&json_results)
        }
        .map_err(io::Error::other)?;
        if let Some(path) = parsed.json_output_path.as_ref() {
            if path.exists() && path.is_dir() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("--json-out points to a directory: {}", path.display()),
                ));
            }
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            if path.exists() {
                eprintln!(
                    "[loctree][warn] JSON output will overwrite existing file: {}",
                    path.display()
                );
            }
            fs::write(path, payload.as_bytes()).map_err(|err| {
                io::Error::other(format!(
                    "failed to write JSON to {}: {}",
                    path.display(),
                    err
                ))
            })?;
            if parsed.verbose {
                eprintln!("[loctree][debug] wrote JSON to {}", path.display());
            } else {
                eprintln!("[loctree] JSON written to {}", path.display());
            }
        } else {
            println!("{}", payload);
        }
    }

    if let Some(report_path) = parsed.report_path.as_ref() {
        write_report(report_path, &report_sections, parsed.verbose)?;
        open_in_browser(report_path);
    }

    if parsed.serve && !parsed.serve_once {
        use std::io::Read;
        eprintln!("[loctree] --serve: Press Enter (Ctrl+C to interrupt) to stop the server");
        let _ = std::io::stdin().read(&mut [0u8]).ok();
    }
    drop(server_handle);

    // Check --fail-on-* flags and return appropriate exit code
    let mut fail_reasons: Vec<String> = Vec::new();

    if parsed.fail_on_missing_handlers && !global_missing_handlers.is_empty() {
        fail_reasons.push(format!(
            "{} missing handler(s): {}",
            global_missing_handlers.len(),
            global_missing_handlers
                .iter()
                .map(|h| h.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    if parsed.fail_on_ghost_events {
        let ghost_count = pipeline_summary
            .get("events")
            .and_then(|e| e.get("ghostCount"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let orphan_count = pipeline_summary
            .get("events")
            .and_then(|e| e.get("orphanCount"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if ghost_count > 0 {
            fail_reasons.push(format!(
                "{} ghost event(s) (emitted but no listener)",
                ghost_count
            ));
        }
        if orphan_count > 0 {
            fail_reasons.push(format!("{} orphan listener(s) (no emitter)", orphan_count));
        }
    }

    if parsed.fail_on_races {
        let race_count = pipeline_summary
            .get("events")
            .and_then(|e| e.get("races"))
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        if race_count > 0 {
            fail_reasons.push(format!("{} potential race(s) detected", race_count));
        }
    }

    if !fail_reasons.is_empty() {
        eprintln!("[loctree][fail] {}", fail_reasons.join("; "));
        return Err(io::Error::other(format!(
            "Pipeline check failed: {}",
            fail_reasons.join("; ")
        )));
    }

    Ok(())
}
