use std::collections::{HashMap, HashSet};
use std::io;

use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::args::ParsedArgs;
use crate::snapshot::GitContext;
use crate::types::{FileAnalysis, ImportKind, ImportResolutionKind, OutputMode, ReexportKind};

use super::CommandGap;
use super::RankedDup;
use super::ReportSection;
use super::classify::language_from_path;
use super::graph::{MAX_GRAPH_EDGES, MAX_GRAPH_NODES, build_graph_data};
use super::html::render_html_report;
use super::insights::collect_ai_insights;
use super::open_server::current_open_base;
use super::report::CommandBridge;
use super::report::TreeNode;
use super::root_scan::{RootContext, normalize_module_id};
use super::scan::resolve_event_constants_across_files;

fn build_tree(analyses: &[FileAnalysis], root_path: &std::path::Path) -> Vec<TreeNode> {
    #[derive(Default)]
    struct TmpNode {
        loc: usize,
        children: std::collections::BTreeMap<String, TmpNode>,
    }

    let mut root = TmpNode::default();
    let mut paths: Vec<(Vec<String>, usize)> = analyses
        .iter()
        .map(|a| {
            let rel = std::path::Path::new(&a.path)
                .strip_prefix(root_path)
                .unwrap_or_else(|_| std::path::Path::new(&a.path))
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            (rel, a.loc)
        })
        .collect();
    paths.sort_by(|a, b| a.0.cmp(&b.0));

    for (parts, loc) in paths {
        let mut cursor = &mut root;
        for part in parts {
            let entry = cursor.children.entry(part).or_default();
            cursor = entry;
        }
        cursor.loc = loc;
    }

    fn finalize(name: Option<String>, node: &TmpNode) -> TreeNode {
        let mut loc_sum = node.loc;
        let mut children: Vec<TreeNode> = node
            .children
            .iter()
            .map(|(k, v)| finalize(Some(k.clone()), v))
            .collect();
        for c in &children {
            loc_sum += c.loc;
        }
        children.sort_by(|a, b| a.path.cmp(&b.path));
        TreeNode {
            path: name.unwrap_or_default(),
            loc: loc_sum,
            children,
        }
    }

    root.children
        .iter()
        .map(|(k, v)| finalize(Some(k.clone()), v))
        .collect()
}

pub struct RootArtifacts {
    pub json_items: Vec<serde_json::Value>,
    pub report_section: Option<ReportSection>,
}

#[cfg(test)]
mod tests {
    use super::build_tree;
    use crate::types::FileAnalysis;
    use std::path::Path;

    #[test]
    fn build_tree_aggregates_loc_and_hierarchy() {
        let analyses = vec![
            FileAnalysis {
                path: "src/a.ts".into(),
                loc: 10,
                ..Default::default()
            },
            FileAnalysis {
                path: "src/nested/b.ts".into(),
                loc: 20,
                ..Default::default()
            },
            FileAnalysis {
                path: "src/nested/deeper/c.ts".into(),
                loc: 30,
                ..Default::default()
            },
        ];
        let tree = build_tree(&analyses, Path::new("src"));
        // Expect top-level nodes include a.ts and nested/
        let a = tree.iter().find(|n| n.path == "a.ts").unwrap();
        assert_eq!(a.loc, 10);
        assert!(a.children.is_empty());

        let nested = tree.iter().find(|n| n.path == "nested").unwrap();
        assert_eq!(nested.loc, 50); // 20 + 30
        let b = nested.children.iter().find(|c| c.path == "b.ts").unwrap();
        assert_eq!(b.loc, 20);
        let deeper = nested.children.iter().find(|c| c.path == "deeper").unwrap();
        assert_eq!(deeper.path, "deeper");
        assert_eq!(deeper.loc, 30);
        assert_eq!(deeper.children.len(), 1);
        let leaf = &deeper.children[0];
        assert_eq!(leaf.path, "c.ts");
        assert_eq!(leaf.loc, 30);
    }

    #[test]
    fn build_tree_handles_root_prefix_mismatch() {
        let analyses = vec![FileAnalysis {
            path: "other/file.ts".into(),
            loc: 5,
            ..Default::default()
        }];
        // If strip_prefix fails, it should fall back to the full path parts.
        let tree = build_tree(&analyses, Path::new("src"));
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].path, "other");
        assert_eq!(tree[0].loc, 5);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn process_root_context(
    idx: usize,
    ctx: RootContext,
    parsed: &ParsedArgs,
    fe_commands: &HashMap<String, Vec<(String, usize, String)>>,
    be_commands: &HashMap<String, Vec<(String, usize, String)>>,
    global_missing_handlers: &[CommandGap],
    global_unregistered_handlers: &[CommandGap],
    global_unused_handlers: &[CommandGap],
    pipeline_summary: &serde_json::Value,
    git: Option<&GitContext>,
    schema_name: &str,
    schema_version: &str,
) -> RootArtifacts {
    let mut json_items = Vec::new();
    let RootContext {
        root_path,
        options: _options,
        mut analyses,
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
        ..
    } = ctx;

    let pipeline_summary = pipeline_summary.clone();

    resolve_event_constants_across_files(&mut analyses);

    let analysis_by_path: HashMap<String, FileAnalysis> = analyses
        .iter()
        .map(|a| (a.path.clone(), a.clone()))
        .collect();

    let duplicate_exports: Vec<_> = export_index
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();

    let reexport_files: HashSet<String> = analyses
        .iter()
        .filter(|a| !a.reexports.is_empty())
        .map(|a| a.path.clone())
        .collect();

    let missing_handlers = global_missing_handlers.to_vec();
    let unregistered_handlers = global_unregistered_handlers.to_vec();
    let unused_handlers = global_unused_handlers.to_vec();

    let (graph_data, graph_warning) = if parsed.graph && parsed.report_path.is_some() {
        build_graph_data(
            &analyses,
            &graph_edges,
            &loc_map,
            fe_commands,
            be_commands,
            parsed.max_graph_nodes.unwrap_or(MAX_GRAPH_NODES),
            parsed.max_graph_edges.unwrap_or(MAX_GRAPH_EDGES),
        )
    } else {
        (None, None)
    };

    let mut sorted_paths: Vec<String> = analyses.iter().map(|a| a.path.clone()).collect();
    sorted_paths.sort();
    let file_id_map: HashMap<String, usize> = sorted_paths
        .iter()
        .enumerate()
        .map(|(idx, p)| (p.clone(), idx + 1))
        .collect();

    let mut imports_targeted: HashSet<String> = HashSet::new();
    let mut files_json: Vec<_> = Vec::new();
    let mut casing_issues: Vec<serde_json::Value> = Vec::new();
    let mut dead_symbols_total = 0usize;
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

            let mut event_emits = a.event_emits.clone();
            event_emits.sort_by(|x, y| x.line.cmp(&y.line).then(x.name.cmp(&y.name)));

            let mut event_listens = a.event_listens.clone();
            event_listens.sort_by(|x, y| x.line.cmp(&y.line).then(x.name.cmp(&y.name)));

            for imp in &imports {
                if let Some(resolved) = &imp.resolved_path {
                    imports_targeted.insert(resolved.clone());
                    imports_targeted.insert(normalize_module_id(resolved).as_key());
                }
            }
            for re in &reexports {
                if let Some(resolved) = &re.resolved {
                    imports_targeted.insert(resolved.clone());
                    imports_targeted.insert(normalize_module_id(resolved).as_key());
                } else {
                    imports_targeted.insert(normalize_module_id(&re.source).as_key());
                    imports_targeted.insert(re.source.clone());
                }
            }
            for dyn_imp in &a.dynamic_imports {
                imports_targeted.insert(dyn_imp.clone());
                imports_targeted.insert(normalize_module_id(dyn_imp).as_key());
            }

            for issue in &a.command_payload_casing {
                casing_issues.push(json!({
                    "command": issue.command,
                    "key": issue.key,
                    "path": issue.path,
                    "line": issue.line,
                }));
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
                    "kind": match i.kind { ImportKind::Static => "static", ImportKind::SideEffect => "side-effect", ImportKind::Dynamic => "dynamic" },
                    "resolvedPath": i.resolved_path,
                    "isBareModule": i.is_bare,
                    "resolutionKind": match i.resolution {
                        ImportResolutionKind::Local => "local",
                        ImportResolutionKind::Stdlib => "stdlib",
                        ImportResolutionKind::Dynamic => "dynamic",
                        ImportResolutionKind::Unknown => "unknown",
                    },
                    "isTypeChecking": i.is_type_checking,
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
                "commandCalls": command_calls.iter().map(|c| json!({
                    "name": c.name,
                    "line": c.line,
                    "genericType": c.generic_type,
                    "payload": c.payload,
                })).collect::<Vec<_>>(),
                "commandHandlers": command_handlers.iter().map(|c| json!({
                    "name": c.name,
                    "line": c.line,
                    "exposedName": c.exposed_name,
                    "payload": c.payload,
                })).collect::<Vec<_>>(),
                "events": {
                    "emit": event_emits.iter().map(|e| json!({
                        "name": e.name,
                        "rawName": e.raw_name,
                        "line": e.line,
                        "kind": e.kind,
                        "payload": e.payload,
                        "awaited": e.awaited,
                    })).collect::<Vec<_>>(),
                    "listen": event_listens.iter().map(|e| json!({
                        "name": e.name,
                        "rawName": e.raw_name,
                        "line": e.line,
                        "kind": e.kind,
                        "payload": e.payload,
                        "awaited": e.awaited,
                    })).collect::<Vec<_>>(),
                },
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

    let missing_set: HashSet<String> = missing_handlers.iter().map(|g| g.name.clone()).collect();
    let unregistered_set: HashSet<String> = unregistered_handlers
        .iter()
        .map(|g| g.name.clone())
        .collect();
    let unused_set: HashSet<String> = unused_handlers.iter().map(|g| g.name.clone()).collect();

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
        } else if unregistered_set.contains(name) {
            "unregistered_handler"
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

    // Build command_bridges for full FE↔BE comparison table
    let mut command_bridges: Vec<CommandBridge> = Vec::new();
    for name in &all_command_names {
        let mut handlers = be_commands.get(name).cloned().unwrap_or_default();
        handlers.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let be_location = handlers
            .first()
            .map(|(path, line, symbol)| (path.clone(), *line, symbol.clone()));

        let mut call_sites = fe_commands.get(name).cloned().unwrap_or_default();
        call_sites.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let fe_locations: Vec<(String, usize)> = call_sites
            .iter()
            .map(|(path, line, _)| (path.clone(), *line))
            .collect();

        let language = be_location
            .as_ref()
            .map(|(path, _, _)| language_from_path(path))
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
        } else if unregistered_set.contains(name) {
            "unregistered_handler"
        } else {
            "ok"
        };

        command_bridges.push(CommandBridge {
            name: name.clone(),
            fe_locations,
            be_location,
            status: status.to_string(),
            language,
        });
    }

    let dup_score_map: HashMap<String, &RankedDup> = filtered_ranked
        .iter()
        .map(|d| (d.name.clone(), d))
        .collect();

    type SymbolOccurrence = (String, String, String, Option<usize>, String);
    let mut symbol_occurrences: HashMap<String, Vec<SymbolOccurrence>> = HashMap::new();
    for analysis in &analyses {
        for exp in &analysis.exports {
            if exp.kind == "reexport" {
                continue;
            }
            if analysis.is_test {
                continue;
            }
            if exp.export_type == "default" {
                continue;
            }
            let norm_path = normalize_module_id(&analysis.path).as_key();
            let entry = symbol_occurrences.entry(exp.name.clone()).or_default();
            let already_present = entry.iter().any(|(_, _, _, _, norm)| norm == &norm_path);
            if already_present {
                continue;
            }
            entry.push((
                analysis.path.clone(),
                exp.export_type.clone(),
                exp.kind.clone(),
                exp.line,
                norm_path,
            ));
        }
    }

    let mut symbols_json = Vec::new();
    let mut clusters_json = Vec::new();
    let mut sorted_symbol_names: Vec<_> = symbol_occurrences.keys().cloned().collect();
    sorted_symbol_names.sort();

    for name in &sorted_symbol_names {
        if let Some(occ_list) = symbol_occurrences.get(name) {
            let canonical_idx = occ_list
                .iter()
                .enumerate()
                .find(|(_, (path, _, _, _, _))| {
                    analysis_by_path
                        .get(path)
                        .map(|a| !a.is_test && !a.is_generated)
                        .unwrap_or(false)
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            let mut occurrences_json = Vec::new();
            let mut occurrence_ids = Vec::new();
            for (idx, (path, export_type, kind, line, norm_path)) in occ_list.iter().enumerate() {
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
                    "normalizedPath": norm_path,
                }));
            }

            let canonical_path = occ_list
                .get(canonical_idx)
                .map(|(p, _, _, _, _)| p.clone())
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
            } else if occ_list.iter().any(|(_, _, kind, _, _)| kind == "reexport") {
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
    default_export_chains.sort_by(|a, b| a["chain"].to_string().cmp(&b["chain"].to_string()));

    let barrels_json: Vec<_> = barrels
        .iter()
        .map(|b| {
            json!({
                "path": b.path,
                "module": b.module_id,
                "reexportCount": b.reexport_count,
                "targetCount": b.target_count,
                "mixed": b.mixed,
                "targets": b.targets,
            })
        })
        .collect();

    let mut suspicious_barrels = Vec::new();
    for b in &barrels {
        if b.mixed || b.reexport_count >= 20 || b.target_count >= 12 {
            let dup_in_cluster = symbol_occurrences
                .iter()
                .filter(|(_, occs)| {
                    occs.len() > 1 && occs.iter().any(|(path, _, _, _, _)| path == &b.path)
                })
                .count();
            suspicious_barrels.push(json!({
                "path": b.path,
                "module": b.module_id,
                "reexportCount": b.reexport_count,
                "targetCount": b.target_count,
                "mixed": b.mixed,
                "duplicatesInClusterCount": dup_in_cluster,
            }));
        }
    }
    suspicious_barrels.sort_by(|a, b| {
        let a_path = a["path"].as_str().unwrap_or("");
        let b_path = b["path"].as_str().unwrap_or("");
        a_path.cmp(b_path)
    });

    let mut dead_symbols = Vec::new();
    if !parsed.skip_dead_symbols {
        for (name, occs) in &symbol_occurrences {
            if occs.iter().all(|(path, _, _, _, norm)| {
                !imports_targeted.contains(path) && !imports_targeted.contains(norm)
            }) {
                let mut paths: Vec<_> = occs.iter().map(|(p, _, _, _, _)| p.clone()).collect();
                paths.sort();
                paths.dedup();
                let public_surface = paths.iter().any(|p| {
                    p.ends_with("index.ts")
                        || p.ends_with("index.tsx")
                        || p.ends_with("mod.rs")
                        || p.ends_with("lib.rs")
                });
                dead_symbols
                    .push(json!({"name": name, "paths": paths, "publicSurface": public_surface}));
            }
        }
        dead_symbols.sort_by(|a, b| {
            let a_name = a["name"].as_str().unwrap_or("");
            let b_name = b["name"].as_str().unwrap_or("");
            a_name.cmp(b_name)
        });
        dead_symbols_total = dead_symbols.len();
        dead_symbols.truncate(parsed.top_dead_symbols);
    }

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

    let ghost_events: Vec<_> = pipeline_summary["events"]["ghostEmits"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let orphan_listeners: Vec<_> = pipeline_summary["events"]["orphanListeners"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let pipeline_risks: Vec<_> = pipeline_summary["risks"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let bridge_limit = parsed.summary_limit.max(50);
    let barrel_limit = parsed.summary_limit.max(50);
    let mut bridges_for_ai = Vec::new();
    for cmd in commands2.iter().take(bridge_limit) {
        bridges_for_ai.push(cmd.clone());
    }

    if matches!(parsed.output, OutputMode::Json | OutputMode::Jsonl) {
        if parsed.ai_mode {
            let top_limit = parsed.summary_limit;
            let mut event_alerts = Vec::new();
            for item in ghost_events.iter().take(top_limit) {
                event_alerts.push(json!({
                    "type": "ghost_event",
                    "name": item.get("name"),
                    "path": item.get("path"),
                    "line": item.get("line"),
                }));
            }
            for item in orphan_listeners.iter().take(top_limit) {
                event_alerts.push(json!({
                    "type": "orphan_listener",
                    "name": item.get("name"),
                    "path": item.get("path"),
                    "line": item.get("line"),
                    "awaited": item.get("awaited"),
                }));
            }

            let ai_payload = json!({
                "schema": schema_name,
                "schemaVersion": schema_version,
                "generatedAt": generated_at,
                "rootDir": root_path,
                "git": {
                    "repo": git.and_then(|g| g.repo.clone()),
                    "branch": git.and_then(|g| g.branch.clone()),
                    "commit": git.and_then(|g| g.commit.clone()),
                    "scanId": git.and_then(|g| g.scan_id.clone()),
                },
                "languages": languages_vec,
                "filesAnalyzed": analyses.len(),
                "summary": {
                    "duplicateExports": filtered_ranked.len(),
                    "reexportFiles": reexport_files.len(),
                    "dynamicImports": dynamic_summary.len(),
                "commands": {
                    "frontendCalls": fe_commands.len(),
                    "backendHandlers": be_commands.len(),
                    "missingHandlers": missing_handlers.len(),
                    "unusedHandlers": unused_handlers.len(),
                },
                    "events": {
                        "ghost": ghost_events.len(),
                        "orphan": orphan_listeners.len(),
                        "risks": pipeline_risks.len(),
                    },
                    "clusters": {
                        "duplicateCount": duplicate_clusters_count,
                        "maxClusterSize": max_cluster_size,
                    },
                    "barrels": {
                        "count": barrels.len(),
                        "mixed": barrels.iter().filter(|b| b.mixed).count(),
                    },
                },
                "topIssues": {
                    "duplicateExports": filtered_ranked.iter().take(top_limit).map(|dup| json!({
                        "name": dup.name,
                        "canonical": dup.canonical,
                        "refactorTargets": dup.refactors,
                        "score": dup.score,
                    })).collect::<Vec<_>>(),
                    "missingHandlers": missing_handlers.iter().take(top_limit).map(|g| json!({
                        "name": g.name,
                        "locations": g.locations,
                    })).collect::<Vec<_>>(),
                    "unusedHandlers": unused_handlers.iter().take(top_limit).map(|g| {
                        let mut obj = json!({
                            "name": g.name,
                            "locations": g.locations,
                        });
                        if let Some(conf) = &g.confidence {
                            obj["confidence"] = json!(conf.to_string());
                        }
                        if !g.string_literal_matches.is_empty() {
                            obj["stringLiteralMatches"] = json!(g.string_literal_matches.len());
                        }
                        obj
                    }).collect::<Vec<_>>(),
                    "events": event_alerts,
                    "pipelineRisks": pipeline_risks.iter().take(top_limit).cloned().collect::<Vec<_>>(),
                    "deadSymbols": dead_symbols.iter().take(parsed.top_dead_symbols).cloned().collect::<Vec<_>>(),
                    "duplicateClusters": top_clusters,
                    "bridges": bridges_for_ai,
                    "barrels": barrels_json.iter().take(barrel_limit).cloned().collect::<Vec<_>>(),
                },
                "limits": {
                    "topItems": top_limit,
                    "topDeadSymbols": parsed.top_dead_symbols,
                    "bridges": bridge_limit,
                    "barrels": barrel_limit,
                }
            });

            if matches!(parsed.output, OutputMode::Jsonl) {
                if let Ok(line) = serde_json::to_string(&ai_payload) {
                    println!("{}", line);
                } else {
                    eprintln!("[loctree][warn] failed to serialize JSONL line for AI payload");
                }
            } else {
                json_items.push(ai_payload);
            }
        } else {
            let payload = json!({
                "schema": schema_name,
                "schemaVersion": schema_version,
                "generatedAt": generated_at,
                "rootDir": root_path,
                "root": root_path,
                "git": {
                    "repo": git.and_then(|g| g.repo.clone()),
                    "branch": git.and_then(|g| g.branch.clone()),
                    "commit": git.and_then(|g| g.commit.clone()),
                    "scanId": git.and_then(|g| g.scan_id.clone()),
                },
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
                "barrels": barrels_json,
                "dynamicImports": dynamic_imports_json,
                "commands": {
                    "frontend": fe_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "backend": be_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "missingHandlers": missing_handlers.iter().map(|g| json!({"name": g.name, "locations": g.locations})).collect::<Vec<_>>(),
                    "unusedHandlers": unused_handlers.iter().map(|g| {
                        let mut obj = json!({"name": g.name, "locations": g.locations});
                        if let Some(conf) = &g.confidence {
                            obj["confidence"] = json!(conf.to_string());
                        }
                        if !g.string_literal_matches.is_empty() {
                            obj["stringLiteralMatches"] = json!(g.string_literal_matches.iter().map(|m| {
                                json!({"file": m.file, "line": m.line, "context": m.context})
                            }).collect::<Vec<_>>());
                        }
                        obj
                    }).collect::<Vec<_>>(),
                    "payloadCasing": casing_issues,
                },
                "commands2": commands2,
                "tauri_analysis": {
                    "total_handlers": be_commands.len(),
                    "total_calls": fe_commands.len(),
                    "registered": be_commands.len().saturating_sub(unregistered_handlers.len()),
                    "coverage": {
                        "ok": all_command_names.len().saturating_sub(
                            missing_handlers.len() + unused_handlers.len() + unregistered_handlers.len()
                        ),
                        "missing_handler": missing_handlers.len(),
                        "unused_handler": unused_handlers.len(),
                        "unregistered_handler": unregistered_handlers.len(),
                    },
                    "missing_handlers": missing_handlers.iter().map(|g| &g.name).collect::<Vec<_>>(),
                    "unused_handlers": unused_handlers.iter().map(|g| &g.name).collect::<Vec<_>>(),
                    "unregistered_handlers": unregistered_handlers.iter().map(|g| &g.name).collect::<Vec<_>>(),
                },
                "symbols": symbols_json,
                "clusters": clusters_json,
                "pipeline": pipeline_summary,
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
                        "ghostEventCount": ghost_events.len(),
                        "orphanListenerCount": orphan_listeners.len(),
                    },
                    "tsconfig": tsconfig_summary,
                    "barrels": {
                        "count": barrels.len(),
                        "mixed": barrels.iter().filter(|b| b.mixed).count(),
                        "items": barrels_json,
                    },
                    "ciSummary": {
                        "duplicateClustersCount": duplicate_clusters_count,
                        "maxClusterSize": max_cluster_size,
                        "topClusters": top_clusters,
                    }
                },
                "files": files_json,
            });

            if matches!(parsed.output, OutputMode::Jsonl) {
                if let Ok(line) = serde_json::to_string(&payload) {
                    println!("{}", line);
                } else {
                    eprintln!("[loctree][warn] failed to serialize JSONL line");
                }
            } else {
                json_items.push(payload);
            }
        }
    } else {
        if idx > 0 {
            println!();
        }

        println!("Import/export analysis for {}/", root_path.display());
        println!("  Files analyzed: {}", analyses.len());
        println!("  Duplicate exports: {}", filtered_ranked.len());
        println!("  Files with re-exports: {}", reexport_files.len());
        println!("  Dynamic imports: {}", dynamic_summary.len());
        if dead_symbols_total > 0 {
            println!(
                "  Dead exports (high confidence): {}{}",
                dead_symbols_total,
                if dead_symbols_total > parsed.top_dead_symbols {
                    format!(" (showing top {})", parsed.top_dead_symbols)
                } else {
                    String::new()
                }
            );
        }

        if !duplicate_exports.is_empty() {
            println!(
                "
Top duplicate exports (showing up to {}):",
                parsed.analyze_limit
            );
            for dup in filtered_ranked.iter().take(parsed.analyze_limit) {
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
                parsed.analyze_limit
            );
            let mut sorted_dyn = dynamic_summary.clone();
            sorted_dyn.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
            for (file, sources) in sorted_dyn.iter().take(parsed.analyze_limit) {
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
                use crate::analyzer::report::Confidence;
                let high_conf: Vec<_> = unused_handlers
                    .iter()
                    .filter(|g| g.confidence == Some(Confidence::High))
                    .map(|g| g.name.clone())
                    .collect();
                let low_conf: Vec<_> = unused_handlers
                    .iter()
                    .filter(|g| g.confidence == Some(Confidence::Low))
                    .collect();

                if !high_conf.is_empty() {
                    println!(
                        "  Unused handlers (HIGH confidence): {}",
                        high_conf.join(", ")
                    );
                }
                if !low_conf.is_empty() {
                    println!("  Unused handlers (LOW confidence - possible dynamic usage):");
                    for g in &low_conf {
                        let matches_note = if !g.string_literal_matches.is_empty() {
                            format!(
                                " ({} string literal matches)",
                                g.string_literal_matches.len()
                            )
                        } else {
                            String::new()
                        };
                        println!("    - {}{}", g.name, matches_note);
                    }
                }
                // Fallback for handlers without confidence (shouldn't happen but be safe)
                let no_conf: Vec<_> = unused_handlers
                    .iter()
                    .filter(|g| g.confidence.is_none())
                    .map(|g| g.name.clone())
                    .collect();
                if !no_conf.is_empty() {
                    println!("  Unused handlers: {}", no_conf.join(", "));
                }
            }
        }

        println!("\nTip: rerun with --json for machine-readable output.");
    }

    let mut report_section = None;
    if parsed.report_path.is_some() {
        let mut sorted_dyn = dynamic_summary.clone();
        sorted_dyn.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        let insights = collect_ai_insights(
            &analyses,
            &filtered_ranked,
            &cascades,
            &missing_handlers,
            &unused_handlers,
        );
        let mut missing_sorted = missing_handlers.clone();
        missing_sorted.sort_by(|a, b| a.name.cmp(&b.name));
        let mut unused_sorted = unused_handlers.clone();
        unused_sorted.sort_by(|a, b| a.name.cmp(&b.name));
        let mut unregistered_sorted = unregistered_handlers.clone();
        unregistered_sorted.sort_by(|a, b| a.name.cmp(&b.name));

        // Calculate total LOC
        let total_loc: usize = analyses.iter().map(|a| a.loc).sum();

        let tree = build_tree(&analyses, &root_path);

        report_section = Some(ReportSection {
            insights,
            root: root_path.display().to_string(),
            files_analyzed: analyses.len(),
            total_loc,
            reexport_files_count: reexport_files.len(),
            dynamic_imports_count: dynamic_summary.len(),
            ranked_dups: filtered_ranked.clone(),
            cascades: cascades.clone(),
            dynamic: sorted_dyn,
            analyze_limit: parsed.analyze_limit,
            missing_handlers: missing_sorted,
            unregistered_handlers: unregistered_sorted,
            unused_handlers: unused_sorted,
            command_counts: (fe_commands.len(), be_commands.len()),
            command_bridges: command_bridges.clone(),
            open_base: if parsed.report_path.is_some() && parsed.serve {
                current_open_base()
            } else {
                None
            },
            tree: Some(tree),
            graph: graph_data.clone(),
            graph_warning: graph_warning.clone(),
            git_branch: git.and_then(|g| g.branch.clone()),
            git_commit: git.and_then(|g| g.commit.clone()),
        });
    }

    RootArtifacts {
        json_items,
        report_section,
    }
}

pub fn write_report(
    report_path: &std::path::Path,
    sections: &[ReportSection],
    verbose: bool,
) -> io::Result<()> {
    if let Some(dir) = report_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    render_html_report(report_path, sections)?;
    // Show relative path for cleaner output (with ./ prefix for consistency)
    let display_path = std::env::current_dir()
        .ok()
        .and_then(|cwd| report_path.strip_prefix(&cwd).ok())
        .map(|p| format!("./{}", p.display()))
        .unwrap_or_else(|| report_path.display().to_string());
    if verbose {
        eprintln!("[loctree][debug] wrote HTML to {}", display_path);
    } else {
        eprintln!("[loctree] HTML report written to {}", display_path);
    }
    Ok(())
}
