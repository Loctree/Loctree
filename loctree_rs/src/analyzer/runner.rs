use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::args::{preset_ignore_symbols, ParsedArgs};
use crate::fs_utils::{gather_files, normalise_ignore_patterns, GitIgnoreChecker};
use crate::types::{ExportIndex, FileAnalysis, ImportKind, Options, OutputMode, ReexportKind};

use globset::{Glob, GlobSet, GlobSetBuilder};

use super::css::analyze_css_file;
use super::html::render_html_report;
use super::js::analyze_js_file;
use super::open_server::{current_open_base, open_in_browser, start_open_server};
use super::py::analyze_py_file;
use super::resolvers::{resolve_js_relative, resolve_python_relative};
use super::rust::analyze_rust_file;
use super::{
    AiInsight, CommandGap, GraphComponent, GraphData, GraphNode, RankedDup, ReportSection,
};

const MAX_GRAPH_NODES: usize = 8000;
const MAX_GRAPH_EDGES: usize = 12000;
const DEFAULT_EXCLUDE_REPORT_PATTERNS: &[&str] =
    &["**/__tests__/**", "scripts/semgrep-fixtures/**"];

fn normalize_cmd_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_lower = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_uppercase() && last_was_lower && !out.is_empty() {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            last_was_lower = ch.is_ascii_lowercase();
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
            last_was_lower = false;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        name.to_lowercase()
    } else {
        out
    }
}

fn is_dev_file(path: &str) -> bool {
    path.contains("__tests__")
        || path.contains("stories")
        || path.contains(".stories.")
        || path.contains("story.")
}

fn layout_positions(comps: &[Vec<String>]) -> HashMap<String, (f32, f32)> {
    let cols = (comps.len() as f32).sqrt().ceil() as usize + 1;
    let spacing = 1200f32;
    let mut positions: HashMap<String, (f32, f32)> = HashMap::new();
    for (idx, comp) in comps.iter().enumerate() {
        let row = idx / cols;
        let col = idx % cols;
        let cx = (col as f32) * spacing;
        let cy = (row as f32) * spacing;
        let n = comp.len().max(1) as f32;
        let radius = 160.0 + 30.0 * n.sqrt();
        for (i, node) in comp.iter().enumerate() {
            let theta = (i as f32) * (std::f32::consts::TAU / n);
            let jitter = 12.0 * (i as f32 % 3.0) - 12.0;
            let x = cx + radius * theta.cos() + jitter;
            let y = cy + radius * theta.sin() - jitter;
            positions.insert(node.clone(), (x, y));
        }
    }
    positions
}

#[allow(clippy::type_complexity)]
fn compute_components(
    nodes: &[String],
    edges: &[(String, String, String)],
) -> (
    Vec<Vec<String>>,
    HashMap<String, usize>,
    HashMap<String, usize>,
) {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for n in nodes {
        adj.entry(n.clone()).or_default();
    }
    for (a, b, _) in edges {
        if a.is_empty() || b.is_empty() {
            continue;
        }
        let entry = adj.entry(a.clone()).or_default();
        if !entry.contains(b) {
            entry.push(b.clone());
        }
        let back = adj.entry(b.clone()).or_default();
        if !back.contains(a) {
            back.push(a.clone());
        }
    }

    let degrees: HashMap<String, usize> = adj.iter().map(|(k, v)| (k.clone(), v.len())).collect();

    let mut visited: HashSet<String> = HashSet::new();
    let mut comps: Vec<Vec<String>> = Vec::new();
    for n in nodes {
        if visited.contains(n) {
            continue;
        }
        let mut stack = vec![n.clone()];
        let mut comp = Vec::new();
        visited.insert(n.clone());
        while let Some(cur) = stack.pop() {
            comp.push(cur.clone());
            if let Some(neigh) = adj.get(&cur) {
                for nb in neigh {
                    if visited.insert(nb.clone()) {
                        stack.push(nb.clone());
                    }
                }
            }
        }
        comps.push(comp);
    }

    comps.sort_by(|a, b| {
        b.len().cmp(&a.len()).then(
            a.first()
                .unwrap_or(&String::new())
                .cmp(b.first().unwrap_or(&String::new())),
        )
    });

    let mut node_to_component: HashMap<String, usize> = HashMap::new();
    for (idx, comp) in comps.iter().enumerate() {
        let cid = idx + 1;
        for node in comp {
            node_to_component.insert(node.clone(), cid);
        }
    }

    (comps, node_to_component, degrees)
}

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

fn strip_excluded_paths(
    paths: &[(String, usize, String)],
    focus: &Option<GlobSet>,
    exclude: &Option<GlobSet>,
) -> Vec<(String, usize)> {
    paths
        .iter()
        .filter_map(|(p, line, _)| {
            let pb = Path::new(p);
            if let Some(ex) = exclude {
                if ex.is_match(pb) {
                    return None;
                }
            }
            if let Some(focus_globs) = focus {
                if !focus_globs.is_match(pb) {
                    return None;
                }
            }
            Some((p.clone(), *line))
        })
        .collect()
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
    root: &Path,
    extensions: Option<&HashSet<String>>,
) -> io::Result<FileAnalysis> {
    let content = std::fs::read_to_string(path)?;
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let loc = content.lines().count();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let mut analysis = match ext.as_str() {
        "rs" => analyze_rust_file(&content, relative),
        "css" => analyze_css_file(&content, relative),
        "py" => analyze_py_file(&content, path, root, extensions, relative),
        _ => analyze_js_file(&content, path, root, extensions, relative),
    };
    analysis.loc = loc;

    Ok(analysis)
}

pub fn default_analyzer_exts() -> HashSet<String> {
    ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn collect_ai_insights(
    files: &[FileAnalysis],
    dups: &[RankedDup],
    cascades: &[(String, String)],
    gap_missing: &[CommandGap],
    _gap_unused: &[CommandGap],
) -> Vec<AiInsight> {
    let mut insights = Vec::new();

    // 1. Huge files
    let huge_files: Vec<_> = files.iter().filter(|f| f.loc > 2000).collect();
    if !huge_files.is_empty() {
        insights.push(AiInsight {
            title: "Huge files detected".to_string(),
            severity: "medium".to_string(),
            message: format!(
                "Found {} files with > 2000 LOC (e.g. {}). Consider splitting them.",
                huge_files.len(),
                huge_files[0].path
            ),
        });
    }

    // 2. Many duplicates
    if dups.len() > 10 {
        insights.push(AiInsight {
            title: "High number of duplicate exports".to_string(),
            severity: "medium".to_string(),
            message: format!(
                "Found {} duplicate export groups. Consider refactoring.",
                dups.len()
            ),
        });
    }

    // 3. Cascades
    if cascades.len() > 20 {
        insights.push(AiInsight {
            title: "Many re-export chains".to_string(),
            severity: "low".to_string(),
            message: format!(
                "Found {} re-export cascades. This might affect tree-shaking/bundling.",
                cascades.len()
            ),
        });
    }

    // 4. Missing handlers
    if !gap_missing.is_empty() {
        insights.push(AiInsight {
            title: "Missing Tauri Handlers".to_string(),
            severity: "high".to_string(),
            message: format!(
                "Frontend calls {} commands that are missing in Backend.",
                gap_missing.len()
            ),
        });
    }

    insights
}

pub fn run_import_analyzer(root_list: &[PathBuf], parsed: &ParsedArgs) -> io::Result<()> {
    let mut json_results = Vec::new();
    let mut report_sections: Vec<ReportSection> = Vec::new();
    let mut server_handle = None;
    let mut open_base = None;

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
        if let Some((base, handle)) =
            start_open_server(root_list.to_vec(), parsed.editor_cmd.clone())
        {
            server_handle = Some(handle);
            open_base = Some(base.clone());
            eprintln!("[loctree] local open server at {}", base);
        } else {
            eprintln!("[loctree][warn] could not start open server; continue without --serve");
        }
    }

    for (idx, root_path) in root_list.iter().enumerate() {
        let ignore_paths = normalise_ignore_patterns(&parsed.ignore_patterns, root_path);
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
        };

        let git_checker = if options.use_gitignore {
            GitIgnoreChecker::new(root_path)
        } else {
            None
        };

        let mut files = Vec::new();
        gather_files(root_path, &options, 0, git_checker.as_ref(), &mut files)?;

        let mut analyses = Vec::new();
        let mut export_index: ExportIndex = HashMap::new();
        let mut reexport_edges: Vec<(String, Option<String>)> = Vec::new();
        let mut dynamic_summary: Vec<(String, Vec<String>)> = Vec::new();
        let mut fe_commands: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();
        let mut be_commands: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();
        let mut graph_edges: Vec<(String, String, String)> = Vec::new();
        let mut loc_map: HashMap<String, usize> = HashMap::new();

        for file in files {
            let analysis = analyze_file(&file, root_path, options.extensions.as_ref())?;
            let is_excluded_for_commands = exclude_set
                .as_ref()
                .map(|set| set.is_match(Path::new(&analysis.path)))
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
                    let resolved = match ext.as_str() {
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
                        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
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
                    };
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

        let fe_norms: HashMap<String, String> = fe_commands
            .keys()
            .map(|k| (k.clone(), normalize_cmd_name(k)))
            .collect();
        let be_norms: HashMap<String, String> = be_commands
            .keys()
            .map(|k| (k.clone(), normalize_cmd_name(k)))
            .collect();
        let be_norm_set: HashSet<String> = be_norms.values().cloned().collect();
        let fe_norm_set: HashSet<String> = fe_norms.values().cloned().collect();

        let missing_handlers: Vec<CommandGap> = fe_commands
            .iter()
            .filter_map(|(name, locs)| {
                let norm = fe_norms
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| normalize_cmd_name(name));
                if be_norm_set.contains(&norm) {
                    return None;
                }
                let kept = strip_excluded_paths(locs, &focus_set, &exclude_set);
                if kept.is_empty() {
                    None
                } else {
                    let impl_name = locs
                        .iter()
                        .find(|(p, l, _)| p == &kept[0].0 && *l == kept[0].1)
                        .map(|(_, _, n)| n.clone());
                    Some(CommandGap {
                        name: name.clone(),
                        implementation_name: impl_name,
                        locations: kept,
                    })
                }
            })
            .collect();
        let unused_handlers: Vec<CommandGap> = be_commands
            .iter()
            .filter_map(|(name, locs)| {
                let norm = be_norms
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| normalize_cmd_name(name));
                if fe_norm_set.contains(&norm) {
                    return None;
                }
                let kept = strip_excluded_paths(locs, &focus_set, &exclude_set);
                if kept.is_empty() {
                    None
                } else {
                    let impl_name = locs
                        .iter()
                        .find(|(p, l, _)| p == &kept[0].0 && *l == kept[0].1)
                        .map(|(_, _, n)| n.clone());
                    Some(CommandGap {
                        name: name.clone(),
                        implementation_name: impl_name,
                        locations: kept,
                    })
                }
            })
            .collect();

        let mut section_open = None;
        if options.report_path.is_some() && options.serve {
            section_open = open_base.clone().or_else(current_open_base);
        }

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
                graph: if parsed.graph && options.report_path.is_some() {
                    let mut nodes: HashSet<String> =
                        analyses.iter().map(|a| a.path.clone()).collect();
                    for (a, b, _) in &graph_edges {
                        if !a.is_empty() {
                            nodes.insert(a.clone());
                        }
                        if !b.is_empty() {
                            nodes.insert(b.clone());
                        }
                    }

                    if nodes.is_empty() {
                        None
                    } else if nodes.len() > MAX_GRAPH_NODES || graph_edges.len() > MAX_GRAPH_EDGES {
                        eprintln!(
                            "[loctree][warn] graph skipped ({} nodes, {} edges > limits)",
                            nodes.len(),
                            graph_edges.len()
                        );
                        None
                    } else {
                        let mut nodes_vec: Vec<String> = nodes.into_iter().collect();
                        nodes_vec.sort();
                        let (component_nodes, node_to_component, degrees) =
                            compute_components(&nodes_vec, &graph_edges);
                        let positions = layout_positions(&component_nodes);
                        let main_component_id = if component_nodes.is_empty() { 0 } else { 1 };

                        let mut component_meta: Vec<GraphComponent> = Vec::new();
                        for (idx, comp_nodes) in component_nodes.iter().enumerate() {
                            let mut sorted_nodes = comp_nodes.clone();
                            sorted_nodes.sort();
                            let cid = idx + 1;
                            let comp_set: HashSet<String> = sorted_nodes.iter().cloned().collect();
                            let edge_count = graph_edges
                                .iter()
                                .filter(|(a, b, _)| comp_set.contains(a) && comp_set.contains(b))
                                .count();
                            let isolated_count = sorted_nodes
                                .iter()
                                .filter(|n| degrees.get(*n).cloned().unwrap_or(0) == 0)
                                .count();
                            let loc_sum: usize = sorted_nodes
                                .iter()
                                .map(|n| loc_map.get(n).cloned().unwrap_or(0))
                                .sum();
                            let sample = sorted_nodes.first().cloned().unwrap_or_default();

                            let tauri_frontend = fe_commands
                                .values()
                                .flat_map(|locs| locs.iter())
                                .filter(|(path, _, _)| comp_set.contains(path))
                                .count();
                            let tauri_backend = be_commands
                                .values()
                                .flat_map(|locs| locs.iter())
                                .filter(|(path, _, _)| comp_set.contains(path))
                                .count();
                            let detached = main_component_id != 0 && cid != main_component_id;

                            component_meta.push(GraphComponent {
                                id: cid,
                                size: sorted_nodes.len(),
                                edge_count,
                                nodes: sorted_nodes,
                                isolated_count,
                                sample,
                                loc_sum,
                                detached,
                                tauri_frontend,
                                tauri_backend,
                            });
                        }

                        let graph_nodes: Vec<GraphNode> = nodes_vec
                            .iter()
                            .filter_map(|id| {
                                if id.is_empty() {
                                    return None;
                                }
                                let (x, y) = positions.get(id).cloned().unwrap_or((0.0, 0.0));
                                let loc = loc_map.get(id).cloned().unwrap_or(0);
                                let label =
                                    id.rsplit('/').next().unwrap_or(id.as_str()).to_string();
                                let component = *node_to_component.get(id).unwrap_or(&0);
                                let degree = *degrees.get(id).unwrap_or(&0);
                                let detached =
                                    main_component_id != 0 && component != main_component_id;
                                Some(GraphNode {
                                    id: id.clone(),
                                    label,
                                    loc,
                                    x,
                                    y,
                                    component,
                                    degree,
                                    detached,
                                })
                            })
                            .collect();
                        Some(GraphData {
                            nodes: graph_nodes,
                            edges: graph_edges.clone(),
                            components: component_meta,
                            main_component_id,
                        })
                    }
                } else {
                    None
                },
            });
        }

        if matches!(options.output, OutputMode::Json | OutputMode::Jsonl) {
            let files_json: Vec<_> = analyses
                .iter()
                .map(|a| {
                    json!({
                        "path": a.path,
                        "imports": a.imports.iter().map(|i| json!({"source": i.source, "kind": match i.kind { ImportKind::Static => "static", ImportKind::SideEffect => "side-effect" }})).collect::<Vec<_>>(),
                        "reexports": a.reexports.iter().map(|r| {
                            match &r.kind {
                                ReexportKind::Star => json!({"source": r.source, "kind": "star", "resolved": r.resolved}),
                                ReexportKind::Named(names) => json!({"source": r.source, "kind": "named", "names": names, "resolved": r.resolved})
                            }
                        }).collect::<Vec<_>>(),
                        "dynamicImports": a.dynamic_imports,
                        "exports": a.exports.iter().map(|e| json!({"name": e.name, "kind": e.kind})).collect::<Vec<_>>(),
                        "commandCalls": a.command_calls.iter().map(|c| json!({"name": c.name, "line": c.line})).collect::<Vec<_>>(),
                        "commandHandlers": a.command_handlers.iter().map(|c| json!({"name": c.name, "line": c.line})).collect::<Vec<_>>(),
                    })
                })
                .collect();

            let payload = json!({
                "root": root_path,
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
                "dynamicImports": dynamic_summary
                    .iter()
                    .map(|(file, sources)| {
                        let unique: HashSet<_> = sources.iter().collect();
                        json!({
                            "file": file,
                            "sources": sources,
                            "manySources": sources.len() > 5,
                            "selfImport": unique.len() < sources.len(),
                    })
                })
                .collect::<Vec<_>>(),
                "commands": {
                    "frontend": fe_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "backend": be_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "missingHandlers": missing_handlers.iter().map(|g| json!({"name": g.name, "locations": g.locations})).collect::<Vec<_>>(),
                    "unusedHandlers": unused_handlers.iter().map(|g| json!({"name": g.name, "locations": g.locations})).collect::<Vec<_>>(),
                },
                "files": files_json,
            });

            if matches!(options.output, OutputMode::Jsonl) {
                println!("{}", serde_json::to_string(&payload).unwrap());
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
            serde_json::to_string_pretty(&json_results[0]).unwrap()
        } else {
            serde_json::to_string_pretty(&json_results).unwrap()
        };
        if let Some(path) = parsed.json_output_path.as_ref() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Err(err) = fs::write(path, payload.as_bytes()) {
                eprintln!(
                    "[loctree][warn] failed to write JSON to {}: {}",
                    path.display(),
                    err
                );
            } else {
                eprintln!("[loctree] JSON written to {}", path.display());
            }
        } else {
            println!("{}", payload);
        }
    }

    if let Some(report_path) = parsed.report_path.as_ref() {
        render_html_report(report_path, &report_sections)?;
        eprintln!("[loctree] HTML report written to {}", report_path.display());
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
