use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::args::{preset_ignore_symbols, ParsedArgs};
use crate::fs_utils::{gather_files, normalise_ignore_patterns, GitIgnoreChecker};
use crate::types::{ExportIndex, FileAnalysis, ImportKind, Options, OutputMode, ReexportKind};

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::classify::{detect_language, file_kind, is_dev_file, language_from_path};
use super::css::analyze_css_file;
use super::html::render_html_report;
use super::js::analyze_js_file;
use super::open_server::{current_open_base, open_in_browser, start_open_server};
use super::py::analyze_py_file;
use super::resolvers::{resolve_js_relative, resolve_python_relative};
use super::rust::analyze_rust_file;
use super::tsconfig::summarize_tsconfig;
use super::{
    coverage::{compute_command_gaps, CommandUsage},
    graph::{build_graph_data, MAX_GRAPH_EDGES, MAX_GRAPH_NODES},
    insights::collect_ai_insights,
};
use super::{RankedDup, ReportSection};

const DEFAULT_EXCLUDE_REPORT_PATTERNS: &[&str] =
    &["**/__tests__/**", "scripts/semgrep-fixtures/**"];

const SCHEMA_NAME: &str = "loctree-json";
const SCHEMA_VERSION: &str = "1.1.0";

fn build_globset(patterns: &[String]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let mut added = false;
    for pat in patterns {
        if pat.trim().is_empty() {
            continue;
        }
        match Glob::new(pat) {
            Ok(glob) => {
                builder.add(glob);
                added = true;
            }
            Err(err) => eprintln!("[loctree][warn] invalid glob '{}': {}", pat, err),
        }
    }
    if !added {
        None
    } else {
        builder.build().ok()
    }
}

fn opt_globset(globs: &[String]) -> Option<GlobSet> {
    build_globset(globs).and_then(|g| if g.is_empty() { None } else { Some(g) })
}

fn strip_excluded(files: &[String], exclude: &Option<GlobSet>) -> Vec<String> {
    match exclude {
        None => files.to_vec(),
        Some(set) => files.iter().filter(|p| !set.is_match(p)).cloned().collect(),
    }
}

fn matches_focus(files: &[String], focus: &Option<GlobSet>) -> bool {
    match focus {
        None => true,
        Some(set) => files.iter().any(|p| set.is_match(p)),
    }
}

fn analyze_file(
    path: &Path,
    root_canon: &Path,
    extensions: Option<&HashSet<String>>,
) -> io::Result<FileAnalysis> {
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(root_canon) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "analyzed file escapes provided root",
        ));
    }

    // nosemgrep:rust.actix.path-traversal.tainted-path.tainted-path - canonicalized and bounded to root_canon above
    let content = std::fs::read_to_string(&canonical)?;
    let relative = canonical
        .strip_prefix(root_canon)
        .unwrap_or(&canonical)
        .to_string_lossy()
        .to_string();
    let loc = content.lines().count();
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let mut analysis = match ext.as_str() {
        "rs" => analyze_rust_file(&content, relative),
        "css" => analyze_css_file(&content, relative),
        "py" => analyze_py_file(&content, &canonical, root_canon, extensions, relative),
        _ => analyze_js_file(&content, &canonical, root_canon, extensions, relative),
    };
    analysis.loc = loc;
    analysis.language = detect_language(&ext);
    let (kind, is_test, is_generated) = file_kind(&analysis.path);
    analysis.kind = kind;
    analysis.is_test = is_test;
    analysis.is_generated = is_generated;

    for imp in analysis.imports.iter_mut() {
        if imp.resolved_path.is_none() && imp.source.starts_with('.') {
            let resolved = match ext.as_str() {
                "py" => resolve_python_relative(&imp.source, &canonical, root_canon, extensions),
                "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "css" => {
                    resolve_js_relative(&canonical, root_canon, &imp.source, extensions)
                }
                _ => None,
            };
            imp.resolved_path = resolved;
        }
    }

    Ok(analysis)
}

pub fn default_analyzer_exts() -> HashSet<String> {
    ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
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

    if parsed.serve {
        if let Some((base, handle)) = start_open_server(
            root_list.to_vec(),
            parsed.editor_cmd.clone(),
            parsed.report_path.clone(),
        ) {
            server_handle = Some(handle);
            eprintln!("[loctree] local open server at {}", base);
        } else {
            eprintln!("[loctree][warn] could not start open server; continue without --serve");
        }
    }

    for (idx, root_path) in root_list.iter().enumerate() {
        let ignore_paths = normalise_ignore_patterns(&parsed.ignore_patterns, root_path);
        let root_canon = root_path
            .canonicalize()
            .unwrap_or_else(|_| root_path.clone());
        let mut extensions = parsed.extensions.clone();
        if extensions.is_none() {
            extensions = Some(default_analyzer_exts());
        }

        let options = Options {
            extensions: extensions.clone(),
            ignore_paths,
            use_gitignore: parsed.use_gitignore,
            max_depth: parsed.max_depth,
            color: parsed.color,
            output: parsed.output,
            summary: parsed.summary,
            summary_limit: parsed.summary_limit,
            show_hidden: parsed.show_hidden,
            loc_threshold: parsed.loc_threshold,
            analyze_limit: parsed.analyze_limit,
            report_path: parsed.report_path.clone(),
            serve: parsed.serve,
            editor_cmd: parsed.editor_cmd.clone(),
            max_graph_nodes: parsed.max_graph_nodes,
            max_graph_edges: parsed.max_graph_edges,
            verbose: parsed.verbose,
        };

        let effective_max_nodes = options.max_graph_nodes.unwrap_or(MAX_GRAPH_NODES);
        let effective_max_edges = options.max_graph_edges.unwrap_or(MAX_GRAPH_EDGES);

        if options.verbose {
            eprintln!("[loctree][debug] analyzing root {}", root_path.display());
        }

        let git_checker = if options.use_gitignore {
            GitIgnoreChecker::new(root_path)
        } else {
            None
        };

        let mut files = Vec::new();
        let mut visited = HashSet::new();
        gather_files(
            root_path,
            &options,
            0,
            git_checker.as_ref(),
            &mut visited,
            &mut files,
        )?;

        if let (Some(focus), Some(exclude)) = (&focus_set, &exclude_set) {
            let mut overlapping = Vec::new();
            for path in &files {
                let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
                if focus.is_match(&canon) && exclude.is_match(&canon) {
                    overlapping.push(canon.display().to_string());
                    if overlapping.len() >= 5 {
                        break;
                    }
                }
            }
            if !overlapping.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "--focus and --exclude-report overlap on: {}",
                        overlapping.join(", ")
                    ),
                ));
            }
        }

        let mut analyses = Vec::new();
        let mut export_index: ExportIndex = HashMap::new();
        let mut reexport_edges: Vec<(String, Option<String>)> = Vec::new();
        let mut dynamic_summary: Vec<(String, Vec<String>)> = Vec::new();
        let mut fe_commands: CommandUsage = HashMap::new();
        let mut be_commands: CommandUsage = HashMap::new();
        let mut graph_edges: Vec<(String, String, String)> = Vec::new();
        let mut loc_map: HashMap<String, usize> = HashMap::new();
        let mut languages: HashSet<String> = HashSet::new();

        for file in files {
            let analysis = analyze_file(&file, &root_canon, options.extensions.as_ref())?;
            let abs_for_match = root_canon.join(&analysis.path);
            let is_excluded_for_commands = exclude_set
                .as_ref()
                .map(|set| {
                    let canon = abs_for_match
                        .canonicalize()
                        .unwrap_or_else(|_| abs_for_match.clone());
                    set.is_match(&canon)
                })
                .unwrap_or(false);

            loc_map.insert(analysis.path.clone(), analysis.loc);
            for exp in &analysis.exports {
                let name_lc = exp.name.to_lowercase();
                let ignored = ignore_exact.contains(&name_lc)
                    || ignore_prefixes.iter().any(|p| name_lc.starts_with(p));
                if ignored {
                    continue;
                }
                export_index
                    .entry(exp.name.clone())
                    .or_default()
                    .push(analysis.path.clone());
            }
            for re in &analysis.reexports {
                reexport_edges.push((analysis.path.clone(), re.resolved.clone()));
                if parsed.graph && options.report_path.is_some() {
                    if let Some(target) = &re.resolved {
                        graph_edges.push((
                            analysis.path.clone(),
                            target.clone(),
                            "reexport".to_string(),
                        ));
                    }
                }
            }
            if !analysis.dynamic_imports.is_empty() {
                dynamic_summary.push((analysis.path.clone(), analysis.dynamic_imports.clone()));
            }
            if parsed.graph && options.report_path.is_some() {
                let ext = file
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                for imp in &analysis.imports {
                    let resolved = imp.resolved_path.clone().or_else(|| match ext.as_str() {
                        "py" => {
                            if imp.source.starts_with('.') {
                                resolve_python_relative(
                                    &imp.source,
                                    &file,
                                    root_path,
                                    options.extensions.as_ref(),
                                )
                            } else {
                                None
                            }
                        }
                        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "css" => {
                            if imp.source.starts_with('.') {
                                resolve_js_relative(
                                    &file,
                                    root_path,
                                    &imp.source,
                                    options.extensions.as_ref(),
                                )
                            } else {
                                None
                            }
                        }
                        _ => None,
                    });
                    if let Some(target) = resolved {
                        graph_edges.push((
                            analysis.path.clone(),
                            target,
                            match imp.kind {
                                ImportKind::Static | ImportKind::SideEffect => "import".to_string(),
                            },
                        ));
                    }
                }
            }
            if !is_excluded_for_commands {
                for call in &analysis.command_calls {
                    fe_commands.entry(call.name.clone()).or_default().push((
                        analysis.path.clone(),
                        call.line,
                        call.name.clone(),
                    ));
                }
                for handler in &analysis.command_handlers {
                    let mut key = handler
                        .exposed_name
                        .as_ref()
                        .unwrap_or(&handler.name)
                        .clone();
                    if let Some(stripped) = key.strip_suffix("_command") {
                        key = stripped.to_string();
                    } else if let Some(stripped) = key.strip_suffix("_cmd") {
                        key = stripped.to_string();
                    }
                    be_commands.entry(key).or_default().push((
                        analysis.path.clone(),
                        handler.line,
                        handler.name.clone(),
                    ));
                }
            }
            languages.insert(analysis.language.clone());
            analyses.push(analysis);
        }
        let duplicate_exports: Vec<_> = export_index
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .collect();

        let reexport_files: HashSet<String> = analyses
            .iter()
            .filter(|a| !a.reexports.is_empty())
            .map(|a| a.path.clone())
            .collect();

        let mut cascades = Vec::new();
        for (from, resolved) in &reexport_edges {
            if let Some(target) = resolved {
                if reexport_files.contains(target) {
                    cascades.push((from.clone(), target.clone()));
                }
            }
        }

        let mut ranked_dups: Vec<RankedDup> = Vec::new();
        for (name, files) in &duplicate_exports {
            // Filter out well-known noisy dupes
            let all_rs = files.iter().all(|f| f.ends_with(".rs"));
            let all_d_ts = files.iter().all(|f| {
                f.ends_with(".d.ts")
                    || f.ends_with(".d.tsx")
                    || f.ends_with(".d.mts")
                    || f.ends_with(".d.cts")
            });
            if (name == "new" && all_rs) || (name == "default" && all_d_ts) {
                continue;
            }
            let dev_count = files.iter().filter(|f| is_dev_file(f)).count();
            let prod_count = files.len().saturating_sub(dev_count);
            let score = prod_count * 2 + dev_count;
            let canonical = files
                .iter()
                .find(|f| !is_dev_file(f))
                .cloned()
                .unwrap_or_else(|| files[0].clone());
            let mut refactors: Vec<String> =
                files.iter().filter(|f| *f != &canonical).cloned().collect();
            refactors.sort();
            ranked_dups.push(RankedDup {
                name: name.clone(),
                files: files.clone(),
                score,
                prod_count,
                dev_count,
                canonical,
                refactors,
            });
        }
        ranked_dups.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(b.files.len().cmp(&a.files.len()))
        });

        let mut filtered_ranked: Vec<RankedDup> = Vec::new();
        for dup in ranked_dups.into_iter() {
            let kept_files = strip_excluded(&dup.files, &exclude_set);
            if kept_files.len() <= 1 {
                continue;
            }
            if !matches_focus(&kept_files, &focus_set) {
                continue;
            }
            let canonical = if kept_files.contains(&dup.canonical) {
                dup.canonical.clone()
            } else {
                kept_files
                    .iter()
                    .find(|f| !is_dev_file(f))
                    .cloned()
                    .unwrap_or_else(|| kept_files[0].clone())
            };
            let dev_count = kept_files.iter().filter(|f| is_dev_file(f)).count();
            let prod_count = kept_files.len().saturating_sub(dev_count);
            let score = prod_count * 2 + dev_count;
            let mut refactors: Vec<String> = kept_files
                .iter()
                .filter(|f| *f != &canonical)
                .cloned()
                .collect();
            refactors.sort();
            filtered_ranked.push(RankedDup {
                name: dup.name,
                files: kept_files,
                score,
                prod_count,
                dev_count,
                canonical,
                refactors,
            });
        }
        filtered_ranked.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(b.files.len().cmp(&a.files.len()))
        });

        let (missing_handlers, unused_handlers) =
            compute_command_gaps(&fe_commands, &be_commands, &focus_set, &exclude_set);

        let mut section_open = None;
        if options.report_path.is_some() && options.serve {
            section_open = current_open_base();
        }

        let mut graph_warning = None;
        let mut graph_data = None;

        if options.report_path.is_some() {
            let mut sorted_dyn = dynamic_summary.clone();
            sorted_dyn.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

            let insights = collect_ai_insights(
                &analyses,
                &filtered_ranked,
                &cascades,
                &missing_handlers,
                &unused_handlers,
            );

            if parsed.graph && options.report_path.is_some() {
                let (graph, warn) = build_graph_data(
                    &analyses,
                    &graph_edges,
                    &loc_map,
                    &fe_commands,
                    &be_commands,
                    effective_max_nodes,
                    effective_max_edges,
                );
                graph_warning = warn;
                graph_data = graph;
                if options.verbose {
                    if let Some(w) = &graph_warning {
                        eprintln!("[loctree][debug] graph warning: {}", w);
                    } else {
                        eprintln!(
                            "[loctree][debug] graph ready: nodes={}, edges={}",
                            graph_data.as_ref().map(|g| g.nodes.len()).unwrap_or(0),
                            graph_data.as_ref().map(|g| g.edges.len()).unwrap_or(0)
                        );
                    }
                }
            }

            report_sections.push(ReportSection {
                insights,
                root: root_path.display().to_string(),
                files_analyzed: analyses.len(),
                ranked_dups: filtered_ranked.clone(),
                cascades: cascades.clone(),
                dynamic: sorted_dyn,
                analyze_limit: options.analyze_limit,
                missing_handlers: {
                    let mut v = missing_handlers.clone();
                    v.sort_by(|a, b| a.name.cmp(&b.name));
                    v
                },
                unused_handlers: {
                    let mut v = unused_handlers.clone();
                    v.sort_by(|a, b| a.name.cmp(&b.name));
                    v
                },
                command_counts: (fe_commands.len(), be_commands.len()),
                open_base: section_open,
                graph: graph_data,
                graph_warning,
            });
        }

        if matches!(options.output, OutputMode::Json | OutputMode::Jsonl) {
            let mut sorted_paths: Vec<String> = analyses.iter().map(|a| a.path.clone()).collect();
            sorted_paths.sort();
            let file_id_map: HashMap<String, usize> = sorted_paths
                .iter()
                .enumerate()
                .map(|(idx, p)| (p.clone(), idx + 1))
                .collect();

            let analysis_by_path: HashMap<String, FileAnalysis> = analyses
                .iter()
                .map(|a| (a.path.clone(), a.clone()))
                .collect();

            let mut imports_targeted: HashSet<String> = HashSet::new();

            let mut files_json: Vec<_> = Vec::new();
            for path in &sorted_paths {
                if let Some(a) = analysis_by_path.get(path) {
                    let mut imports = a.imports.clone();
                    imports.sort_by(|x, y| x.source.cmp(&y.source));

                    let mut reexports = a.reexports.clone();
                    reexports.sort_by(|x, y| x.source.cmp(&y.source));

                    let mut exports = a.exports.clone();
                    exports.sort_by(|x, y| x.name.cmp(&y.name));

                    let mut command_calls = a.command_calls.clone();
                    command_calls.sort_by(|x, y| x.line.cmp(&y.line).then(x.name.cmp(&y.name)));

                    let mut command_handlers = a.command_handlers.clone();
                    command_handlers.sort_by(|x, y| x.line.cmp(&y.line).then(x.name.cmp(&y.name)));

                    for imp in &imports {
                        if let Some(resolved) = &imp.resolved_path {
                            imports_targeted.insert(resolved.clone());
                        }
                    }

                    files_json.push(json!({
                        "id": file_id_map.get(&a.path).cloned().unwrap_or(0),
                        "path": a.path,
                        "loc": a.loc,
                        "language": a.language,
                        "kind": a.kind,
                        "isTest": a.is_test,
                        "isGenerated": a.is_generated,
                        "imports": imports.iter().map(|i| json!({
                            "source": i.source,
                            "sourceRaw": i.source_raw,
                            "kind": match i.kind { ImportKind::Static => "static", ImportKind::SideEffect => "side-effect" },
                            "resolvedPath": i.resolved_path,
                            "isBareModule": i.is_bare,
                            "symbols": i.symbols.iter().map(|s| json!({"name": s.name, "alias": s.alias})).collect::<Vec<_>>(),
                        })).collect::<Vec<_>>(),
                        "reexports": reexports.iter().map(|r| {
                            match &r.kind {
                                ReexportKind::Star => json!({"source": r.source, "kind": "star", "resolved": r.resolved}),
                                ReexportKind::Named(names) => json!({"source": r.source, "kind": "named", "names": names, "resolved": r.resolved})
                            }
                        }).collect::<Vec<_>>(),
                        "dynamicImports": a.dynamic_imports,
                        "exports": exports.iter().map(|e| json!({
                            "name": e.name,
                            "kind": e.kind,
                            "exportType": e.export_type,
                            "line": e.line,
                        })).collect::<Vec<_>>(),
                        "commandCalls": command_calls.iter().map(|c| json!({"name": c.name, "line": c.line, "genericType": c.generic_type})).collect::<Vec<_>>(),
                        "commandHandlers": command_handlers.iter().map(|c| json!({"name": c.name, "line": c.line, "exposedName": c.exposed_name})).collect::<Vec<_>>(),
                    }));
                }
            }

            let mut languages_vec: Vec<_> = languages.iter().cloned().collect();
            languages_vec.sort();

            let mut all_command_names: Vec<String> = fe_commands
                .keys()
                .chain(be_commands.keys())
                .cloned()
                .collect();
            all_command_names.sort();
            all_command_names.dedup();

            let missing_set: HashSet<String> =
                missing_handlers.iter().map(|g| g.name.clone()).collect();
            let unused_set: HashSet<String> =
                unused_handlers.iter().map(|g| g.name.clone()).collect();

            let mut commands2 = Vec::new();
            for name in &all_command_names {
                let mut handlers = be_commands.get(name).cloned().unwrap_or_default();
                handlers.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                let canonical = handlers.first().map(|(path, line, symbol)| {
                    json!({
                        "path": path,
                        "line": line,
                        "symbol": symbol,
                        "language": language_from_path(path),
                    })
                });

                let mut call_sites = fe_commands.get(name).cloned().unwrap_or_default();
                call_sites.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

                let language = canonical
                    .as_ref()
                    .and_then(|c| c.get("language").and_then(|v| v.as_str()))
                    .map(|s| s.to_string())
                    .or_else(|| {
                        call_sites
                            .first()
                            .map(|(path, _, _)| language_from_path(path))
                    })
                    .unwrap_or_default();

                let status = if missing_set.contains(name) {
                    "missing_handler"
                } else if unused_set.contains(name) {
                    "unused_handler"
                } else {
                    "ok"
                };

                commands2.push(json!({
                    "name": name,
                    "kind": if canonical.is_some() { "tauri_command" } else { "custom" },
                    "language": language,
                    "canonicalLocation": canonical,
                    "callSites": call_sites.iter().map(|(path, line, symbol)| json!({
                        "path": path,
                        "line": line,
                        "symbol": symbol,
                        "language": language_from_path(path),
                    })).collect::<Vec<_>>(),
                    "status": status,
                }));
            }

            let dup_score_map: HashMap<String, &RankedDup> = filtered_ranked
                .iter()
                .map(|d| (d.name.clone(), d))
                .collect();

            type SymbolOccurrence = (String, String, String, Option<usize>);
            let mut symbol_occurrences: HashMap<String, Vec<SymbolOccurrence>> = HashMap::new();
            for analysis in &analyses {
                for exp in &analysis.exports {
                    symbol_occurrences
                        .entry(exp.name.clone())
                        .or_default()
                        .push((
                            analysis.path.clone(),
                            exp.export_type.clone(),
                            exp.kind.clone(),
                            exp.line,
                        ));
                }
            }

            let mut symbols_json = Vec::new();
            let mut clusters_json = Vec::new();
            let mut sorted_symbol_names: Vec<_> = symbol_occurrences.keys().cloned().collect();
            sorted_symbol_names.sort();

            let tsconfig_summary = summarize_tsconfig(root_path, &analyses);

            let mut calls_with_generics = Vec::new();
            for analysis in &analyses {
                for call in &analysis.command_calls {
                    if let Some(gen) = &call.generic_type {
                        calls_with_generics.push(json!({
                            "name": call.name,
                            "path": analysis.path,
                            "line": call.line,
                            "genericType": gen,
                        }));
                    }
                }
            }

            let mut renamed_handlers = Vec::new();
            for analysis in &analyses {
                for handler in &analysis.command_handlers {
                    if let Some(exposed) = &handler.exposed_name {
                        if exposed != &handler.name {
                            renamed_handlers.push(json!({
                                "path": analysis.path,
                                "line": handler.line,
                                "name": handler.name,
                                "exposedName": exposed,
                            }));
                        }
                    }
                }
            }

            for name in &sorted_symbol_names {
                if let Some(occ_list) = symbol_occurrences.get(name) {
                    let canonical_idx = occ_list
                        .iter()
                        .enumerate()
                        .find(|(_, (path, _, _, _))| {
                            analysis_by_path
                                .get(path)
                                .map(|a| !a.is_test && !a.is_generated)
                                .unwrap_or(false)
                        })
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);

                    let mut occurrences_json = Vec::new();
                    let mut occurrence_ids = Vec::new();
                    for (idx, (path, export_type, kind, line)) in occ_list.iter().enumerate() {
                        let analysis_meta = analysis_by_path.get(path);
                        let id = format!("symbol:{}#{}", name, idx + 1);
                        occurrence_ids.push(id.clone());
                        occurrences_json.push(json!({
                            "id": id,
                            "fileId": file_id_map.get(path).cloned().unwrap_or(0),
                            "path": path,
                            "exportType": export_type,
                            "kind": kind,
                            "line": line,
                            "isCanonical": idx == canonical_idx,
                            "viaReexport": kind == "reexport",
                            "isTestFile": analysis_meta.map(|a| a.is_test).unwrap_or(false),
                            "isGenerated": analysis_meta.map(|a| a.is_generated).unwrap_or(false),
                        }));
                    }

                    let canonical_path = occ_list
                        .get(canonical_idx)
                        .map(|(p, _, _, _)| p.clone())
                        .unwrap_or_default();
                    let public_surface = canonical_path.ends_with("index.ts")
                        || canonical_path.ends_with("index.tsx")
                        || canonical_path.ends_with("mod.rs")
                        || canonical_path.ends_with("lib.rs");

                    let score = dup_score_map
                        .get(name)
                        .map(|d| d.score)
                        .unwrap_or(occ_list.len());
                    let mut severity = if occ_list.len() > 5 {
                        "high"
                    } else if occ_list.len() > 2 {
                        "medium"
                    } else {
                        "low"
                    };
                    if public_surface && occ_list.len() > 1 {
                        severity = "high";
                    }
                    let reason = if occ_list.len() == 1 {
                        "single_export"
                    } else if occ_list.iter().any(|(_, _, kind, _)| kind == "reexport") {
                        "reexport_chain"
                    } else {
                        "multiple_exports"
                    };

                    symbols_json.push(json!({
                        "id": format!("symbol:{}", name),
                        "name": name,
                        "occurrences": occurrences_json,
                        "duplicateScore": score,
                        "severity": severity,
                        "reason": reason,
                        "publicSurface": public_surface,
                    }));

                    if occ_list.len() > 1 {
                        clusters_json.push(json!({
                            "symbolName": name,
                            "symbolId": format!("symbol:{}", name),
                            "occurrenceIds": occurrence_ids,
                            "canonicalOccurrenceId": format!("symbol:{}#{}", name, canonical_idx + 1),
                            "size": occ_list.len(),
                            "severity": severity,
                            "reason": reason,
                            "publicSurface": public_surface,
                        }));
                    }
                }
            }

            let mut default_export_chains: Vec<_> = cascades
                .iter()
                .map(|(from, to)| json!({"chain": [from, to], "length": 2}))
                .collect();
            default_export_chains
                .sort_by(|a, b| a["chain"].to_string().cmp(&b["chain"].to_string()));

            let mut suspicious_barrels = Vec::new();
            for analysis in &analyses {
                if !analysis.reexports.is_empty() {
                    let star_count = analysis
                        .reexports
                        .iter()
                        .filter(|r| matches!(r.kind, ReexportKind::Star))
                        .count();
                    if analysis.reexports.len() >= 3 || star_count > 0 {
                        let dup_in_cluster = symbol_occurrences
                            .iter()
                            .filter(|(_, occs)| {
                                occs.len() > 1
                                    && occs.iter().any(|(path, _, _, _)| path == &analysis.path)
                            })
                            .count();
                        suspicious_barrels.push(json!({
                            "path": analysis.path,
                            "reexportCount": analysis.reexports.len(),
                            "exportStarCount": star_count,
                            "duplicatesInClusterCount": dup_in_cluster,
                        }));
                    }
                }
            }
            suspicious_barrels.sort_by(|a, b| {
                let a_path = a["path"].as_str().unwrap_or("");
                let b_path = b["path"].as_str().unwrap_or("");
                a_path.cmp(b_path)
            });

            let mut dead_symbols = Vec::new();
            for (name, occs) in &symbol_occurrences {
                if occs
                    .iter()
                    .all(|(path, _, _, _)| !imports_targeted.contains(path))
                {
                    let mut paths: Vec<_> = occs.iter().map(|(p, _, _, _)| p.clone()).collect();
                    paths.sort();
                    paths.dedup();
                    let public_surface = paths.iter().any(|p| {
                        p.ends_with("index.ts")
                            || p.ends_with("index.tsx")
                            || p.ends_with("mod.rs")
                            || p.ends_with("lib.rs")
                    });
                    dead_symbols.push(
                        json!({"name": name, "paths": paths, "publicSurface": public_surface}),
                    );
                }
            }
            dead_symbols.sort_by(|a, b| {
                let a_name = a["name"].as_str().unwrap_or("");
                let b_name = b["name"].as_str().unwrap_or("");
                a_name.cmp(b_name)
            });
            dead_symbols.truncate(50);

            let duplicate_clusters_count = clusters_json.len();
            let max_cluster_size = symbol_occurrences
                .values()
                .map(|v| v.len())
                .max()
                .unwrap_or(0);
            let mut top_clusters = Vec::new();
            let mut sorted_by_size: Vec<_> = symbol_occurrences
                .iter()
                .filter(|(_, v)| v.len() > 1)
                .collect();
            sorted_by_size.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
            for (name, occs) in sorted_by_size.into_iter().take(5) {
                let severity = if occs.len() > 5 {
                    "high"
                } else if occs.len() > 2 {
                    "medium"
                } else {
                    "low"
                };
                top_clusters.push(json!({
                    "symbolName": name,
                    "size": occs.len(),
                    "severity": severity,
                }));
            }

            let mut dynamic_imports_json = Vec::new();
            for (file, sources) in &dynamic_summary {
                let unique: HashSet<_> = sources.iter().collect();
                dynamic_imports_json.push(json!({
                    "file": file,
                    "sources": sources,
                    "manySources": sources.len() > 5,
                    "selfImport": unique.len() < sources.len(),
                }));
            }

            let generated_at = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| String::new());

            let payload = json!({
                "schema": SCHEMA_NAME,
                "schemaVersion": SCHEMA_VERSION,
                "generatedAt": generated_at,
                "rootDir": root_path,
                "root": root_path,
                "languages": languages_vec,
                "filesAnalyzed": analyses.len(),
                "duplicateExports": filtered_ranked
                    .iter()
                    .map(|dup| json!({"name": dup.name, "files": dup.files}))
                    .collect::<Vec<_>>(),
                "duplicateExportsRanked": filtered_ranked
                    .iter()
                    .map(|dup| json!({
                        "name": dup.name,
                        "files": dup.files,
                        "score": dup.score,
                        "nonDevCount": dup.prod_count,
                        "devCount": dup.dev_count,
                        "canonical": dup.canonical,
                        "refactorTargets": dup.refactors,
                    }))
                    .collect::<Vec<_>>(),
                "reexportCascades": cascades
                    .iter()
                    .map(|(from, to)| json!({"from": from, "to": to}))
                    .collect::<Vec<_>>(),
                "dynamicImports": dynamic_imports_json,
                "commands": {
                    "frontend": fe_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "backend": be_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "missingHandlers": missing_handlers.iter().map(|g| json!({"name": g.name, "locations": g.locations})).collect::<Vec<_>>(),
                    "unusedHandlers": unused_handlers.iter().map(|g| json!({"name": g.name, "locations": g.locations})).collect::<Vec<_>>(),
                },
                "commands2": commands2,
                "symbols": symbols_json,
                "clusters": clusters_json,
                "aiViews": {
                    "defaultExportChains": default_export_chains,
                    "suspiciousBarrels": suspicious_barrels,
                    "deadSymbols": dead_symbols,
                    "coverage": {
                        "frontendCommandCount": fe_commands.len(),
                        "backendHandlerCount": be_commands.len(),
                        "missingCount": missing_handlers.len(),
                        "unusedCount": unused_handlers.len(),
                        "renamedHandlers": renamed_handlers,
                        "callsWithGenerics": calls_with_generics,
                    },
                    "tsconfig": tsconfig_summary,
                    "ciSummary": {
                        "duplicateClustersCount": duplicate_clusters_count,
                        "maxClusterSize": max_cluster_size,
                        "topClusters": top_clusters,
                    }
                },
                "files": files_json,
            });

            if matches!(options.output, OutputMode::Jsonl) {
                match serde_json::to_string(&payload) {
                    Ok(line) => println!("{}", line),
                    Err(err) => {
                        eprintln!("[loctree][warn] failed to serialize JSONL line: {}", err);
                    }
                }
            } else {
                json_results.push(payload);
            }
            continue;
        }

        if idx > 0 {
            println!();
        }

        println!("Import/export analysis for {}/", root_path.display());
        println!("  Files analyzed: {}", analyses.len());
        println!("  Duplicate exports: {}", filtered_ranked.len());
        println!("  Files with re-exports: {}", reexport_files.len());
        println!("  Dynamic imports: {}", dynamic_summary.len());

        if !duplicate_exports.is_empty() {
            println!(
                "\nTop duplicate exports (showing up to {}):",
                options.analyze_limit
            );
            for dup in filtered_ranked.iter().take(options.analyze_limit) {
                println!(
                    "  - {} (score {}, {} files: {} prod, {} dev) canonical: {} | refs: {}",
                    dup.name,
                    dup.score,
                    dup.files.len(),
                    dup.prod_count,
                    dup.dev_count,
                    dup.canonical,
                    dup.refactors.join(", ")
                );
            }
        }

        if !cascades.is_empty() {
            println!("\nRe-export cascades:");
            for (from, to) in &cascades {
                println!("  - {} -> {}", from, to);
            }
        }

        if !dynamic_summary.is_empty() {
            println!(
                "\nDynamic imports (showing up to {}):",
                options.analyze_limit
            );
            let mut sorted_dyn = dynamic_summary.clone();
            sorted_dyn.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
            for (file, sources) in sorted_dyn.iter().take(options.analyze_limit) {
                println!(
                    "  - {}: {}{}",
                    file,
                    sources.join(", "),
                    if sources.len() > 5 {
                        "  [many sources]"
                    } else {
                        ""
                    }
                );
            }
        }

        if !missing_handlers.is_empty() || !unused_handlers.is_empty() {
            println!("\nTauri command coverage:");
            if !missing_handlers.is_empty() {
                println!(
                    "  Missing handlers (frontend calls without backend): {}",
                    missing_handlers
                        .iter()
                        .map(|g| g.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if !unused_handlers.is_empty() {
                println!(
                    "  Unused handlers (backend not called by FE): {}",
                    unused_handlers
                        .iter()
                        .map(|g| g.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        println!("\nTip: rerun with --json for machine-readable output.");
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
        if let Some(dir) = report_path.parent() {
            fs::create_dir_all(dir)?;
        }
        render_html_report(report_path, &report_sections)?;
        if parsed.verbose {
            eprintln!("[loctree][debug] wrote HTML to {}", report_path.display());
        } else {
            eprintln!("[loctree] HTML report written to {}", report_path.display());
        }
        open_in_browser(report_path);
    }

    drop(server_handle);

    if parsed.serve {
        use std::io::Read;
        eprintln!("[loctree] --serve: Press Enter (Ctrl+C to interrupt) to stop the server");
        let _ = std::io::stdin().read(&mut [0u8]).ok();
    }
    Ok(())
}
