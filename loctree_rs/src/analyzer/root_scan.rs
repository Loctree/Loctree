use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;

use globset::GlobSet;
use serde_json::json;

use crate::args::ParsedArgs;
use crate::fs_utils::{GitIgnoreChecker, gather_files, normalise_ignore_patterns};
use crate::snapshot::Snapshot;
use crate::types::{
    ColorMode, ExportIndex, FileAnalysis, ImportKind, Options, OutputMode, PayloadMap,
};

use super::classify::is_dev_file;
use super::resolvers::{
    TsPathResolver, resolve_js_relative, resolve_python_absolute, resolve_python_relative,
};
use super::scan::{
    analyze_file, matches_focus, resolve_event_constants_across_files, strip_excluded,
};
use super::{RankedDup, coverage::CommandUsage};

pub struct ScanConfig<'a> {
    pub roots: &'a [PathBuf],
    pub parsed: &'a ParsedArgs,
    pub extensions: Option<HashSet<String>>,
    pub focus_set: &'a Option<GlobSet>,
    pub exclude_set: &'a Option<GlobSet>,
    pub ignore_exact: HashSet<String>,
    pub ignore_prefixes: Vec<String>,
    pub py_stdlib: &'a HashSet<String>,
    /// Cached file analyses from previous snapshot for incremental scanning
    pub cached_analyses: Option<&'a HashMap<String, crate::types::FileAnalysis>>,
    /// Force collection of graph edges (for Init mode snapshots)
    pub collect_edges: bool,
    /// Custom Tauri command macros from .loctree/config.toml
    pub custom_command_macros: &'a [String],
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct ScanResults {
    pub contexts: Vec<RootContext>,
    pub global_fe_commands: CommandUsage,
    pub global_be_commands: CommandUsage,
    pub global_fe_payloads: PayloadMap,
    pub global_be_payloads: PayloadMap,
    pub global_analyses: Vec<FileAnalysis>,
}

pub fn scan_roots(cfg: ScanConfig<'_>) -> io::Result<ScanResults> {
    let mut contexts: Vec<RootContext> = Vec::new();
    let mut global_fe_commands: CommandUsage = HashMap::new();
    let mut global_be_commands: CommandUsage = HashMap::new();
    let mut global_fe_payloads: PayloadMap = HashMap::new();
    let mut global_be_payloads: PayloadMap = HashMap::new();
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
            show_ignored: false, // Only used in tree mode
            loc_threshold: cfg.parsed.loc_threshold,
            analyze_limit: cfg.parsed.analyze_limit,
            report_path: cfg.parsed.report_path.clone(),
            serve: cfg.parsed.serve,
            editor_cmd: cfg.parsed.editor_cmd.clone(),
            max_graph_nodes: cfg.parsed.max_graph_nodes,
            max_graph_edges: cfg.parsed.max_graph_edges,
            verbose: cfg.parsed.verbose,
            scan_all: cfg.parsed.scan_all,
            symbol: cfg.parsed.symbol.clone(),
            impact: cfg.parsed.impact.clone(),
            find_artifacts: false, // Only used in tree mode
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
        let mut fe_payloads: PayloadMap = HashMap::new();
        let mut be_payloads: PayloadMap = HashMap::new();
        let mut graph_edges: Vec<(String, String, String)> = Vec::new();
        let mut loc_map: HashMap<String, usize> = HashMap::new();
        let mut languages: HashSet<String> = HashSet::new();
        let mut barrels: Vec<BarrelInfo> = Vec::new();

        let mut cached_hits = 0usize;
        let mut fresh_scans = 0usize;

        for file in files {
            // Get current file mtime for incremental scanning
            let current_mtime = std::fs::metadata(&file)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            // Compute relative path for cache lookup
            let rel_path = file
                .strip_prefix(&root_canon)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file.to_string_lossy().to_string())
                .replace('\\', "/");

            // Check if we can use cached analysis
            let analysis = if let Some(cache) = cfg.cached_analyses {
                if let Some(cached) = cache.get(&rel_path) {
                    if cached.mtime > 0 && cached.mtime == current_mtime {
                        // File unchanged - reuse cached analysis
                        cached_hits += 1;
                        cached.clone()
                    } else {
                        // File changed - re-analyze
                        fresh_scans += 1;
                        let mut a = analyze_file(
                            &file,
                            &root_canon,
                            options.extensions.as_ref(),
                            ts_resolver.as_ref(),
                            &py_roots,
                            cfg.py_stdlib,
                            options.symbol.as_deref(),
                            cfg.custom_command_macros,
                        )?;
                        a.mtime = current_mtime;
                        a
                    }
                } else {
                    // New file - analyze
                    fresh_scans += 1;
                    let mut a = analyze_file(
                        &file,
                        &root_canon,
                        options.extensions.as_ref(),
                        ts_resolver.as_ref(),
                        &py_roots,
                        cfg.py_stdlib,
                        options.symbol.as_deref(),
                        cfg.custom_command_macros,
                    )?;
                    a.mtime = current_mtime;
                    a
                }
            } else {
                // No cache - fresh scan
                fresh_scans += 1;
                let mut a = analyze_file(
                    &file,
                    &root_canon,
                    options.extensions.as_ref(),
                    ts_resolver.as_ref(),
                    &py_roots,
                    cfg.py_stdlib,
                    options.symbol.as_deref(),
                    cfg.custom_command_macros,
                )?;
                a.mtime = current_mtime;
                a
            };
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
                let collect_edges = cfg.collect_edges
                    || (cfg.parsed.graph && options.report_path.is_some())
                    || options.impact.is_some();
                if collect_edges && let Some(target) = &re.resolved {
                    graph_edges.push((
                        analysis.path.clone(),
                        target.clone(),
                        "reexport".to_string(),
                    ));
                }
            }
            if !analysis.dynamic_imports.is_empty() {
                dynamic_summary.push((analysis.path.clone(), analysis.dynamic_imports.clone()));
            }
            let should_collect_edges = cfg.collect_edges
                || (cfg.parsed.graph && options.report_path.is_some())
                || options.impact.is_some();
            if should_collect_edges {
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
                        // Use full paths for edges (slice module needs exact paths)
                        // Note: normalize_module_id strips /index suffix which breaks slice lookups
                        graph_edges.push((
                            analysis.path.clone(),
                            target,
                            match imp.kind {
                                ImportKind::Static | ImportKind::SideEffect => "import".to_string(),
                                ImportKind::Dynamic => "dynamic_import".to_string(),
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
                .map(|t| normalize_module_id(&t).as_key())
                .collect();
            targets.sort();
            targets.dedup();

            let module_id = normalize_module_id(&analysis.path).as_key();
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
            if let Some(target) = resolved
                && reexport_files.contains(target)
            {
                cascades.push((from.clone(), target.clone()));
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

        // Log incremental scan stats if verbose
        if options.verbose && cfg.cached_analyses.is_some() {
            let total = cached_hits + fresh_scans;
            eprintln!(
                "[loctree][incremental] {} cached, {} fresh ({} total files)",
                cached_hits, fresh_scans, total
            );
        }

        let mut calls_with_generics = Vec::new();
        for analysis in &analyses {
            for call in &analysis.command_calls {
                if let Some(gt) = &call.generic_type {
                    calls_with_generics.push(json!({
                        "name": call.name,
                        "path": analysis.path,
                        "line": call.line,
                        "genericType": gt,
                    }));
                }
            }
        }

        let mut renamed_handlers = Vec::new();
        for analysis in &analyses {
            for handler in &analysis.command_handlers {
                if let Some(exposed) = &handler.exposed_name
                    && exposed != &handler.name
                {
                    renamed_handlers.push(json!({
                        "path": analysis.path,
                        "line": handler.line,
                        "name": handler.name,
                        "exposedName": exposed,
                    }));
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

/// Normalized module identifier with language context
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedModule {
    /// Path without extension and /index suffix
    pub path: String,
    /// Language/extension identifier (ts, tsx, js, jsx, mjs, cjs, rs, py, css)
    pub lang: String,
}

impl NormalizedModule {
    /// Format as string for use as map key: "{path}:{lang}"
    pub fn as_key(&self) -> String {
        format!("{}:{}", self.path, self.lang)
    }

    /// Parse from key string created by as_key()
    pub fn from_key(key: &str) -> Option<Self> {
        let parts: Vec<&str> = key.rsplitn(2, ':').collect();
        if parts.len() == 2 {
            Some(NormalizedModule {
                path: parts[1].to_string(),
                lang: parts[0].to_string(),
            })
        } else {
            None
        }
    }
}

/// Normalize module identifier preserving language context
///
/// This prevents cross-language collisions where foo.rs, foo.ts, and foo/index.ts
/// would all map to the same key "foo".
///
/// Returns a NormalizedModule with separate path and language fields.
pub(crate) fn normalize_module_id(path: &str) -> NormalizedModule {
    let mut p = path.replace('\\', "/");
    let mut lang = String::new();

    // Extract language from extension
    for ext in [
        ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".rs", ".py", ".css",
    ] {
        if let Some(stripped) = p.strip_suffix(ext) {
            p = stripped.to_string();
            lang = ext.trim_start_matches('.').to_string();
            break;
        }
    }

    // Strip /index suffix
    if let Some(stripped) = p.strip_suffix("/index") {
        p = stripped.to_string();
    }

    NormalizedModule { path: p, lang }
}

/// Build ScanResults from a loaded Snapshot (scan once, analyze many)
pub fn scan_results_from_snapshot(snapshot: &Snapshot) -> ScanResults {
    let mut global_fe_commands: CommandUsage = HashMap::new();
    let mut global_be_commands: CommandUsage = HashMap::new();
    let mut global_fe_payloads: PayloadMap = HashMap::new();
    let mut global_be_payloads: PayloadMap = HashMap::new();

    // Build command maps from FileAnalysis
    for analysis in &snapshot.files {
        // Frontend commands (invoke calls)
        for call in &analysis.command_calls {
            let mut key = call.name.clone();
            if let Some(stripped) = key.strip_suffix("_command") {
                key = stripped.to_string();
            }
            global_fe_commands.entry(key.clone()).or_default().push((
                analysis.path.clone(),
                call.line,
                call.name.clone(),
            ));

            // Track payload types if present
            if let Some(payload) = &call.payload {
                global_fe_payloads.entry(key).or_default().push((
                    analysis.path.clone(),
                    call.line,
                    Some(payload.clone()),
                ));
            }
        }

        // Backend handlers (#[tauri::command])
        for handler in &analysis.command_handlers {
            let mut key = handler
                .exposed_name
                .as_ref()
                .unwrap_or(&handler.name)
                .clone();
            if let Some(stripped) = key.strip_suffix("_command") {
                key = stripped.to_string();
            }
            global_be_commands.entry(key.clone()).or_default().push((
                analysis.path.clone(),
                handler.line,
                handler.name.clone(),
            ));

            // Track return types if present
            if let Some(ret) = &handler.payload {
                global_be_payloads.entry(key).or_default().push((
                    analysis.path.clone(),
                    handler.line,
                    Some(ret.clone()),
                ));
            }
        }
    }

    // Build minimal RootContext for each root in snapshot
    let mut contexts = Vec::new();

    // If only one root, all files belong to it (paths are relative in snapshot)
    let single_root = snapshot.metadata.roots.len() == 1;

    for root_str in &snapshot.metadata.roots {
        let root_path = PathBuf::from(root_str);

        // Filter analyses for this root
        // Note: snapshot paths are relative, so for single root we take all files
        let root_analyses: Vec<FileAnalysis> = if single_root {
            snapshot.files.clone()
        } else {
            snapshot
                .files
                .iter()
                .filter(|a| a.path.starts_with(root_str) || root_str == ".")
                .cloned()
                .collect()
        };

        // Build export index
        let mut export_index: ExportIndex = HashMap::new();
        for analysis in &root_analyses {
            for export in &analysis.exports {
                export_index
                    .entry(export.name.clone())
                    .or_default()
                    .push(analysis.path.clone());
            }
        }

        // Build LOC map
        let loc_map: HashMap<String, usize> = root_analyses
            .iter()
            .map(|a| (a.path.clone(), a.loc))
            .collect();

        // Build dynamic summary
        let dynamic_summary: Vec<(String, Vec<String>)> = root_analyses
            .iter()
            .filter(|a| !a.dynamic_imports.is_empty())
            .map(|a| (a.path.clone(), a.dynamic_imports.clone()))
            .collect();

        // Build graph edges from snapshot
        // Note: For single root, all edges belong to it (paths in snapshot are relative)
        let graph_edges: Vec<(String, String, String)> = if single_root {
            snapshot
                .edges
                .iter()
                .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
                .collect()
        } else {
            snapshot
                .edges
                .iter()
                .filter(|e| e.from.starts_with(root_str) || root_str == ".")
                .map(|e| (e.from.clone(), e.to.clone(), e.label.clone()))
                .collect()
        };

        // Detect languages
        let languages: HashSet<String> = root_analyses.iter().map(|a| a.language.clone()).collect();

        // Calculate ranked duplicates
        let filtered_ranked = rank_duplicates(&export_index, &root_analyses, &HashSet::new(), &[]);

        // Build cascades (re-export chains)
        let cascades = build_cascades(&root_analyses);

        // Build barrel info
        let barrels: Vec<BarrelInfo> = snapshot
            .barrels
            .iter()
            .filter(|b| b.path.starts_with(root_str) || root_str == ".")
            .map(|b| BarrelInfo {
                path: b.path.clone(),
                module_id: b.module_id.clone(),
                reexport_count: b.reexport_count,
                target_count: b.targets.len(),
                mixed: false, // Can't determine from snapshot
                targets: b.targets.clone(),
            })
            .collect();

        contexts.push(RootContext {
            root_path,
            options: Options {
                extensions: None,
                ignore_paths: vec![],
                use_gitignore: false,
                max_depth: None,
                color: ColorMode::Auto,
                output: OutputMode::Human,
                summary: false,
                summary_limit: 5,
                show_hidden: false,
                show_ignored: false,
                loc_threshold: 1000,
                analyze_limit: 0,
                report_path: None,
                serve: false,
                editor_cmd: None,
                max_graph_nodes: None,
                max_graph_edges: None,
                verbose: false,
                scan_all: false,
                symbol: None,
                impact: None,
                find_artifacts: false,
            },
            analyses: root_analyses,
            export_index,
            dynamic_summary,
            cascades,
            filtered_ranked,
            graph_edges,
            loc_map,
            languages,
            tsconfig_summary: json!({}),
            calls_with_generics: vec![],
            renamed_handlers: vec![],
            barrels,
        });
    }

    ScanResults {
        contexts,
        global_fe_commands,
        global_be_commands,
        global_fe_payloads,
        global_be_payloads,
        global_analyses: snapshot.files.clone(),
    }
}

/// Rank duplicates by severity (helper for snapshot conversion)
fn rank_duplicates(
    export_index: &ExportIndex,
    _analyses: &[FileAnalysis],
    ignore_exact: &HashSet<String>,
    ignore_prefixes: &[String],
) -> Vec<RankedDup> {
    use super::classify::is_dev_file;

    let mut ranked = Vec::new();
    for (name, files) in export_index {
        if files.len() <= 1 {
            continue;
        }
        let lc = name.to_lowercase();
        if ignore_exact.contains(&lc) {
            continue;
        }
        if ignore_prefixes.iter().any(|p| lc.starts_with(p)) {
            continue;
        }

        let mut prod_count = 0;
        let mut dev_count = 0;
        for f in files {
            if is_dev_file(f) {
                dev_count += 1;
            } else {
                prod_count += 1;
            }
        }

        // Skip if all are dev files
        if prod_count == 0 {
            continue;
        }

        let score = files.len() + prod_count;
        let canonical = files.first().cloned().unwrap_or_default();

        // Refactor targets = all files except canonical
        let refactors: Vec<String> = files.iter().filter(|f| *f != &canonical).cloned().collect();

        ranked.push(RankedDup {
            name: name.clone(),
            score,
            files: files.clone(),
            prod_count,
            dev_count,
            canonical,
            refactors,
        });
    }

    ranked.sort_by(|a, b| b.score.cmp(&a.score));
    ranked
}

/// Build cascades (re-export chains) from analyses
fn build_cascades(analyses: &[FileAnalysis]) -> Vec<(String, String)> {
    let mut cascades = Vec::new();
    for analysis in analyses {
        for reexport in &analysis.reexports {
            if !reexport.source.is_empty() {
                cascades.push((analysis.path.clone(), reexport.source.clone()));
            }
        }
    }
    cascades
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_module_id_preserves_language() {
        // Test that different languages with same base path get different normalized IDs
        let rust_module = normalize_module_id("src/utils.rs");
        let ts_module = normalize_module_id("src/utils.ts");
        let tsx_module = normalize_module_id("src/utils.tsx");

        assert_eq!(rust_module.path, "src/utils");
        assert_eq!(rust_module.lang, "rs");

        assert_eq!(ts_module.path, "src/utils");
        assert_eq!(ts_module.lang, "ts");

        assert_eq!(tsx_module.path, "src/utils");
        assert_eq!(tsx_module.lang, "tsx");

        // Keys should be different to prevent cross-language collisions
        assert_ne!(rust_module.as_key(), ts_module.as_key());
        assert_ne!(rust_module.as_key(), tsx_module.as_key());
        assert_ne!(ts_module.as_key(), tsx_module.as_key());
    }

    #[test]
    fn test_normalize_module_id_index_files() {
        // Test that index files are normalized but preserve language
        let ts_index = normalize_module_id("src/components/index.ts");
        let rs_index = normalize_module_id("src/components/index.rs");

        assert_eq!(ts_index.path, "src/components");
        assert_eq!(ts_index.lang, "ts");

        assert_eq!(rs_index.path, "src/components");
        assert_eq!(rs_index.lang, "rs");

        // Should not collide
        assert_ne!(ts_index.as_key(), rs_index.as_key());
        assert_eq!(ts_index.as_key(), "src/components:ts");
        assert_eq!(rs_index.as_key(), "src/components:rs");
    }

    #[test]
    fn test_normalize_module_id_cross_language_collision() {
        // This is the core bug fix - ensure foo.rs and foo.ts don't collide
        let rust_file = normalize_module_id("src/foo.rs");
        let ts_file = normalize_module_id("src/foo.ts");
        let ts_index = normalize_module_id("src/foo/index.ts");

        // All have same base path
        assert_eq!(rust_file.path, "src/foo");
        assert_eq!(ts_file.path, "src/foo");
        assert_eq!(ts_index.path, "src/foo");

        // But different languages
        assert_eq!(rust_file.lang, "rs");
        assert_eq!(ts_file.lang, "ts");
        assert_eq!(ts_index.lang, "ts");

        // Keys should prevent collisions
        assert_eq!(rust_file.as_key(), "src/foo:rs");
        assert_eq!(ts_file.as_key(), "src/foo:ts");
        assert_eq!(ts_index.as_key(), "src/foo:ts");

        // Rust and TS files should NOT match
        assert_ne!(rust_file.as_key(), ts_file.as_key());

        // TS file and TS index SHOULD match (same language, same normalized path)
        assert_eq!(ts_file.as_key(), ts_index.as_key());
    }

    #[test]
    fn test_normalized_module_from_key() {
        let module = NormalizedModule {
            path: "src/utils".to_string(),
            lang: "ts".to_string(),
        };

        let key = module.as_key();
        assert_eq!(key, "src/utils:ts");

        let parsed = NormalizedModule::from_key(&key).unwrap();
        assert_eq!(parsed.path, "src/utils");
        assert_eq!(parsed.lang, "ts");
        assert_eq!(parsed, module);
    }

    #[test]
    fn test_normalized_module_various_extensions() {
        // Test all supported extensions
        let extensions = vec![
            ("file.ts", "ts"),
            ("file.tsx", "tsx"),
            ("file.js", "js"),
            ("file.jsx", "jsx"),
            ("file.mjs", "mjs"),
            ("file.cjs", "cjs"),
            ("file.rs", "rs"),
            ("file.py", "py"),
            ("file.css", "css"),
        ];

        for (input, expected_lang) in extensions {
            let module = normalize_module_id(input);
            assert_eq!(module.path, "file", "Failed for {}", input);
            assert_eq!(module.lang, expected_lang, "Failed for {}", input);
        }
    }

    #[test]
    fn test_normalized_module_windows_paths() {
        // Test Windows-style paths are normalized
        let module = normalize_module_id("src\\utils\\helpers.ts");
        assert_eq!(module.path, "src/utils/helpers");
        assert_eq!(module.lang, "ts");
        assert_eq!(module.as_key(), "src/utils/helpers:ts");
    }

    #[test]
    fn test_normalized_module_from_key_round_trip() {
        // Test round-trip conversion between module and key
        let test_cases = vec![
            ("src/utils:ts", "src/utils", "ts"),
            ("components/Button:tsx", "components/Button", "tsx"),
            ("lib/helpers:js", "lib/helpers", "js"),
            ("core:rs", "core", "rs"),
        ];

        for (key, expected_path, expected_lang) in test_cases {
            let module = NormalizedModule::from_key(key).unwrap();
            assert_eq!(module.path, expected_path);
            assert_eq!(module.lang, expected_lang);
            assert_eq!(module.as_key(), key);
        }
    }

    #[test]
    fn test_normalized_module_from_key_invalid() {
        // Test that invalid keys return None
        assert!(NormalizedModule::from_key("invalid_key_without_colon").is_none());
        assert!(NormalizedModule::from_key("").is_none());
    }

    #[test]
    fn test_normalized_module_hash_and_eq() {
        // Test that NormalizedModule can be used in HashSet/HashMap
        use std::collections::HashSet;

        let mut set = HashSet::new();

        let mod1 = normalize_module_id("src/utils.ts");
        let mod2 = normalize_module_id("src/utils.ts");
        let mod3 = normalize_module_id("src/utils.rs");

        set.insert(mod1.clone());

        // Same module shouldn't be inserted twice
        assert!(set.contains(&mod2));
        assert_eq!(set.len(), 1);

        // Different language should be different
        set.insert(mod3.clone());
        assert_eq!(set.len(), 2);
    }
}
