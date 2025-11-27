use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;

use globset::GlobSet;
use serde_json::json;

use crate::args::ParsedArgs;
use crate::fs_utils::{gather_files, normalise_ignore_patterns, GitIgnoreChecker};
use crate::types::{ExportIndex, FileAnalysis, ImportKind, Options};

use super::classify::is_dev_file;
use super::resolvers::{
    resolve_js_relative, resolve_python_absolute, resolve_python_relative, TsPathResolver,
};
use super::scan::{
    analyze_file, matches_focus, resolve_event_constants_across_files, strip_excluded,
};
use super::{coverage::CommandUsage, RankedDup};

pub struct ScanConfig<'a> {
    pub roots: &'a [PathBuf],
    pub parsed: &'a ParsedArgs,
    pub extensions: Option<HashSet<String>>,
    pub focus_set: &'a Option<GlobSet>,
    pub exclude_set: &'a Option<GlobSet>,
    pub ignore_exact: HashSet<String>,
    pub ignore_prefixes: Vec<String>,
    pub py_stdlib: &'a HashSet<String>,
}

pub struct RootContext {
    pub root_path: PathBuf,
    pub options: Options,
    pub analyses: Vec<FileAnalysis>,
    pub export_index: ExportIndex,
    pub dynamic_summary: Vec<(String, Vec<String>)>,
    pub cascades: Vec<(String, String)>,
    pub filtered_ranked: Vec<RankedDup>,
    pub graph_edges: Vec<(String, String, String)>,
    pub loc_map: HashMap<String, usize>,
    pub languages: HashSet<String>,
    pub tsconfig_summary: serde_json::Value,
    pub calls_with_generics: Vec<serde_json::Value>,
    pub renamed_handlers: Vec<serde_json::Value>,
    pub barrels: Vec<BarrelInfo>,
}

#[derive(Clone)]
pub struct BarrelInfo {
    pub path: String,
    pub module_id: String,
    pub reexport_count: usize,
    pub target_count: usize,
    pub mixed: bool,
    pub targets: Vec<String>,
}

pub struct ScanResults {
    pub contexts: Vec<RootContext>,
    pub global_fe_commands: CommandUsage,
    pub global_be_commands: CommandUsage,
    pub global_fe_payloads: HashMap<String, Vec<(String, usize, Option<String>)>>,
    pub global_be_payloads: HashMap<String, Vec<(String, usize, Option<String>)>>,
    pub global_analyses: Vec<FileAnalysis>,
}

pub fn scan_roots(cfg: ScanConfig<'_>) -> io::Result<ScanResults> {
    let mut contexts: Vec<RootContext> = Vec::new();
    let mut global_fe_commands: CommandUsage = HashMap::new();
    let mut global_be_commands: CommandUsage = HashMap::new();
    let mut global_fe_payloads: HashMap<String, Vec<(String, usize, Option<String>)>> =
        HashMap::new();
    let mut global_be_payloads: HashMap<String, Vec<(String, usize, Option<String>)>> =
        HashMap::new();
    let mut global_analyses: Vec<FileAnalysis> = Vec::new();

    for root_path in cfg.roots.iter() {
        let ignore_paths = normalise_ignore_patterns(&cfg.parsed.ignore_patterns, root_path);
        let root_canon = root_path
            .canonicalize()
            .unwrap_or_else(|_| root_path.clone());

        let options = Options {
            extensions: cfg.extensions.clone(),
            ignore_paths,
            use_gitignore: cfg.parsed.use_gitignore,
            max_depth: cfg.parsed.max_depth,
            color: cfg.parsed.color,
            output: cfg.parsed.output,
            summary: cfg.parsed.summary,
            summary_limit: cfg.parsed.summary_limit,
            show_hidden: cfg.parsed.show_hidden,
            loc_threshold: cfg.parsed.loc_threshold,
            analyze_limit: cfg.parsed.analyze_limit,
            report_path: cfg.parsed.report_path.clone(),
            serve: cfg.parsed.serve,
            editor_cmd: cfg.parsed.editor_cmd.clone(),
            max_graph_nodes: cfg.parsed.max_graph_nodes,
            max_graph_edges: cfg.parsed.max_graph_edges,
            verbose: cfg.parsed.verbose,
        };

        if options.verbose {
            eprintln!("[loctree][debug] analyzing root {}", root_path.display());
        }

        let git_checker = if options.use_gitignore {
            GitIgnoreChecker::new(root_path)
        } else {
            None
        };

        let ts_resolver = TsPathResolver::from_tsconfig(&root_canon);
        let mut py_roots: Vec<PathBuf> = vec![root_canon.clone()];
        for extra in &cfg.parsed.py_roots {
            let candidate = if extra.is_absolute() {
                extra.clone()
            } else {
                root_canon.join(extra)
            };
            if candidate.exists() {
                py_roots.push(candidate.canonicalize().unwrap_or(candidate));
            } else {
                eprintln!(
                    "[loctree][warn] --py-root '{}' not found under {}; skipping",
                    extra.display(),
                    root_canon.display()
                );
            }
        }

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

        if let (Some(focus), Some(exclude)) = (cfg.focus_set, cfg.exclude_set) {
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
        let mut fe_payloads: HashMap<String, Vec<(String, usize, Option<String>)>> = HashMap::new();
        let mut be_payloads: HashMap<String, Vec<(String, usize, Option<String>)>> = HashMap::new();
        let mut graph_edges: Vec<(String, String, String)> = Vec::new();
        let mut loc_map: HashMap<String, usize> = HashMap::new();
        let mut languages: HashSet<String> = HashSet::new();
        let mut barrels: Vec<BarrelInfo> = Vec::new();

        for file in files {
            let analysis = analyze_file(
                &file,
                &root_canon,
                options.extensions.as_ref(),
                ts_resolver.as_ref(),
                &py_roots,
                cfg.py_stdlib,
            )?;
            let abs_for_match = root_canon.join(&analysis.path);
            let is_excluded_for_commands = cfg
                .exclude_set
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
                if exp.kind == "reexport" {
                    continue;
                }
                if exp.export_type == "default" {
                    continue;
                }
                let name_lc = exp.name.to_lowercase();
                let is_decl = [".d.ts", ".d.tsx", ".d.mts", ".d.cts"]
                    .iter()
                    .any(|ext| analysis.path.ends_with(ext));
                if is_decl && name_lc == "default" {
                    continue;
                }
                let ignored = cfg.ignore_exact.contains(&name_lc)
                    || cfg.ignore_prefixes.iter().any(|p| name_lc.starts_with(p));
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
                if cfg.parsed.graph && options.report_path.is_some() {
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
            if cfg.parsed.graph && options.report_path.is_some() {
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
                                resolve_python_absolute(
                                    &imp.source,
                                    &py_roots,
                                    root_path,
                                    options.extensions.as_ref(),
                                )
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
                                ts_resolver.as_ref().and_then(|r| {
                                    r.resolve(&imp.source, options.extensions.as_ref())
                                })
                            }
                        }
                        _ => None,
                    });
                    if let Some(target) = resolved {
                        let src_id = normalize_module_id(&analysis.path);
                        let tgt_id = normalize_module_id(&target);
                        graph_edges.push((
                            src_id,
                            tgt_id,
                            match imp.kind {
                                ImportKind::Static | ImportKind::SideEffect => "import".to_string(),
                            },
                        ));
                    }
                }
            }
            if !is_excluded_for_commands {
                for call in &analysis.command_calls {
                    let mut key = call.name.clone();
                    if let Some(stripped) = key.strip_suffix("_command") {
                        key = stripped.to_string();
                    } else if let Some(stripped) = key.strip_suffix("_cmd") {
                        key = stripped.to_string();
                    }
                    fe_commands.entry(key.clone()).or_default().push((
                        analysis.path.clone(),
                        call.line,
                        call.name.clone(),
                    ));
                    fe_payloads.entry(key).or_default().push((
                        analysis.path.clone(),
                        call.line,
                        call.payload.clone(),
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
                    be_payloads.entry(key.clone()).or_default().push((
                        analysis.path.clone(),
                        handler.line,
                        handler.payload.clone(),
                    ));
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

        let mut barrel_map: HashMap<String, BarrelInfo> = HashMap::new();
        for analysis in &analyses {
            if analysis.reexports.is_empty() {
                continue;
            }
            if !is_index_like(&analysis.path) {
                continue;
            }

            let mut targets: Vec<String> = analysis
                .reexports
                .iter()
                .filter_map(|r| r.resolved.clone().or_else(|| Some(r.source.clone())))
                .map(|t| normalize_module_id(&t))
                .collect();
            targets.sort();
            targets.dedup();

            let module_id = normalize_module_id(&analysis.path);
            let entry = barrel_map.entry(module_id.clone()).or_insert(BarrelInfo {
                path: analysis.path.clone(),
                module_id,
                reexport_count: 0,
                target_count: 0,
                mixed: false,
                targets: Vec::new(),
            });

            entry.reexport_count += analysis.reexports.len();
            let has_own_defs = analysis.exports.iter().any(|e| e.kind != "reexport");
            entry.mixed |= has_own_defs;
            entry.targets.extend(targets);
        }
        for (_, mut info) in barrel_map {
            info.targets.sort();
            info.targets.dedup();
            info.target_count = info.targets.len();
            barrels.push(info);
        }
        barrels.sort_by(|a, b| a.path.cmp(&b.path));

        for (name, entries) in &fe_commands {
            global_fe_commands
                .entry(name.clone())
                .or_default()
                .extend(entries.clone());
        }
        for (name, entries) in &be_commands {
            global_be_commands
                .entry(name.clone())
                .or_default()
                .extend(entries.clone());
        }
        for (name, entries) in &fe_payloads {
            global_fe_payloads
                .entry(name.clone())
                .or_default()
                .extend(entries.clone());
        }
        for (name, entries) in &be_payloads {
            global_be_payloads
                .entry(name.clone())
                .or_default()
                .extend(entries.clone());
        }
        global_analyses.extend(analyses.iter().cloned());
        let duplicate_exports: Vec<_> = export_index
            .iter()
            .filter(|(_, files)| files.len() > 1)
            .map(|(name, files)| (name.clone(), files.clone()))
            .collect();

        let reexport_files: HashSet<String> = analyses
            .iter()
            .filter(|a| !a.reexports.is_empty())
            .map(|a| a.path.clone())
            .collect();

        resolve_event_constants_across_files(&mut analyses);

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
            let kept_files = strip_excluded(&dup.files, cfg.exclude_set);
            if kept_files.len() <= 1 {
                continue;
            }
            if !matches_focus(&kept_files, cfg.focus_set) {
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

        let tsconfig_summary = super::tsconfig::summarize_tsconfig(root_path, &analyses);

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

        contexts.push(RootContext {
            root_path: root_path.clone(),
            options,
            analyses,
            export_index,
            dynamic_summary,
            cascades,
            filtered_ranked,
            graph_edges,
            loc_map,
            languages,
            tsconfig_summary,
            calls_with_generics,
            renamed_handlers,
            barrels,
        });
    }

    Ok(ScanResults {
        contexts,
        global_fe_commands,
        global_be_commands,
        global_fe_payloads,
        global_be_payloads,
        global_analyses,
    })
}

fn is_index_like(path: &str) -> bool {
    let lowered = path.to_lowercase();
    lowered.ends_with("/index.ts")
        || lowered.ends_with("/index.tsx")
        || lowered.ends_with("/index.js")
        || lowered.ends_with("/index.jsx")
        || lowered.ends_with("/index.mjs")
        || lowered.ends_with("/index.cjs")
        || lowered.ends_with("/index.rs")
}

pub(crate) fn normalize_module_id(path: &str) -> String {
    let mut p = path.replace('\\', "/");
    for ext in [
        ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".rs", ".py", ".css",
    ] {
        if let Some(stripped) = p.strip_suffix(ext) {
            p = stripped.to_string();
            break;
        }
    }
    if let Some(stripped) = p.strip_suffix("/index") {
        return stripped.to_string();
    }
    p
}
