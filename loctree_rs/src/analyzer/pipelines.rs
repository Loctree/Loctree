use std::collections::{HashMap, HashSet};

use globset::GlobSet;
use serde_json::json;

use crate::types::FileAnalysis;

fn normalize_event(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
}

fn is_in_scope(path: &str, focus: &Option<GlobSet>, exclude: &Option<GlobSet>) -> bool {
    let pb = std::path::Path::new(path);
    if let Some(ex) = exclude {
        if ex.is_match(pb) {
            return false;
        }
    }
    if let Some(focus_globs) = focus {
        if !focus_globs.is_match(pb) {
            return false;
        }
    }
    true
}

pub fn build_pipeline_summary(
    analyses: &[FileAnalysis],
    focus: &Option<GlobSet>,
    exclude: &Option<GlobSet>,
    fe_commands: &HashMap<String, Vec<(String, usize, String)>>,
    be_commands: &HashMap<String, Vec<(String, usize, String)>>,
) -> serde_json::Value {
    #[derive(Clone)]
    struct Site {
        norm: String,
        raw: String,
        path: String,
        line: usize,
    }

    #[derive(Default, Clone)]
    struct EventRecord {
        raw_names: HashSet<String>,
        emitters: Vec<Site>,
        listeners: Vec<Site>,
    }

    let mut events: HashMap<String, EventRecord> = HashMap::new();
    let mut path_emit_map: HashMap<String, Vec<Site>> = HashMap::new();

    for analysis in analyses {
        let path = analysis.path.clone();
        if !is_in_scope(&path, focus, exclude) {
            continue;
        }
        for ev in &analysis.event_emits {
            let norm = normalize_event(&ev.name);
            let site = Site {
                norm: norm.clone(),
                raw: ev.name.clone(),
                path: path.clone(),
                line: ev.line,
            };
            path_emit_map
                .entry(path.clone())
                .or_default()
                .push(site.clone());
            let rec = events.entry(norm).or_default();
            rec.raw_names.insert(site.raw.clone());
            rec.emitters.push(site);
        }
        for ev in &analysis.event_listens {
            let norm = normalize_event(&ev.name);
            let site = Site {
                norm: norm.clone(),
                raw: ev.name.clone(),
                path: path.clone(),
                line: ev.line,
            };
            let rec = events.entry(norm).or_default();
            rec.raw_names.insert(site.raw.clone());
            rec.listeners.push(site);
        }
    }

    let mut event_items = Vec::new();
    let mut ghost_emits = Vec::new();
    let mut orphan_listeners = Vec::new();
    let mut risks = Vec::new();

    for (norm, rec) in &events {
        let mut emitters = rec.emitters.clone();
        emitters.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
        let mut listeners = rec.listeners.clone();
        listeners.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));

        let has_emit = !emitters.is_empty();
        let has_listen = !listeners.is_empty();
        let status = match (has_emit, has_listen) {
            (true, true) => "ok",
            (true, false) => "ghost",
            (false, true) => "orphan",
            _ => "unknown",
        };

        let mut aliases: Vec<String> = rec.raw_names.iter().cloned().collect();
        aliases.sort();
        if aliases.len() > 1 {
            risks.push(json!({
                "type": "name_mismatch",
                "normalized": norm,
                "aliases": aliases,
            }));
        }

        if status == "ghost" {
            for site in &emitters {
                ghost_emits.push(json!({
                    "name": site.raw,
                    "path": site.path,
                    "line": site.line,
                    "normalized": norm,
                }));
            }
        }
        if status == "orphan" {
            for site in &listeners {
                orphan_listeners.push(json!({
                    "name": site.raw,
                    "path": site.path,
                    "line": site.line,
                    "normalized": norm,
                }));
            }
        }

        let canonical = aliases.first().cloned().unwrap_or_else(|| norm.clone());
        event_items.push(json!({
            "name": canonical,
            "normalized": norm,
            "aliases": aliases,
            "status": status,
            "emitCount": emitters.len(),
            "listenCount": listeners.len(),
            "emitters": emitters.iter().map(|s| json!({"path": s.path, "line": s.line, "name": s.raw})).collect::<Vec<_>>(),
            "listeners": listeners.iter().map(|s| json!({"path": s.path, "line": s.line, "name": s.raw})).collect::<Vec<_>>(),
        }));
    }

    event_items.sort_by(|a, b| {
        let a_name = a["normalized"].as_str().unwrap_or("");
        let b_name = b["normalized"].as_str().unwrap_or("");
        a_name.cmp(b_name)
    });
    ghost_emits.sort_by(|a, b| {
        let a_name = a["name"].as_str().unwrap_or("");
        let b_name = b["name"].as_str().unwrap_or("");
        a_name
            .cmp(b_name)
            .then(
                a["path"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["path"].as_str().unwrap_or("")),
            )
            .then(
                a["line"]
                    .as_u64()
                    .unwrap_or(0)
                    .cmp(&b["line"].as_u64().unwrap_or(0)),
            )
    });
    orphan_listeners.sort_by(|a, b| {
        let a_name = a["name"].as_str().unwrap_or("");
        let b_name = b["name"].as_str().unwrap_or("");
        a_name
            .cmp(b_name)
            .then(
                a["path"]
                    .as_str()
                    .unwrap_or("")
                    .cmp(b["path"].as_str().unwrap_or("")),
            )
            .then(
                a["line"]
                    .as_u64()
                    .unwrap_or(0)
                    .cmp(&b["line"].as_u64().unwrap_or(0)),
            )
    });

    // Heuristic race detection: invoke appears before any listener in same file
    for analysis in analyses {
        if !is_in_scope(&analysis.path, focus, exclude) {
            continue;
        }
        if analysis.command_calls.is_empty() || analysis.event_listens.is_empty() {
            continue;
        }
        let min_call = analysis
            .command_calls
            .iter()
            .map(|c| c.line)
            .min()
            .unwrap_or(usize::MAX);
        let min_listen = analysis
            .event_listens
            .iter()
            .map(|e| e.line)
            .min()
            .unwrap_or(usize::MAX);
        if min_call < min_listen {
            let first_call = analysis
                .command_calls
                .iter()
                .min_by_key(|c| c.line)
                .cloned();
            if let Some(call) = first_call {
                risks.push(json!({
                    "type": "invoke_before_listen",
                    "path": analysis.path,
                    "line": call.line,
                    "command": call.name,
                    "details": "invoke is called before any listener is registered; event may be missed"
                }));
            }
        }
    }

    // Command chains: where calls/handlers live and what they emit
    let command_names: HashSet<String> = fe_commands
        .keys()
        .chain(be_commands.keys())
        .cloned()
        .collect();
    let mut chains = Vec::new();
    let total_commands = command_names.len();
    for name in &command_names {
        let calls: Vec<_> = fe_commands
            .get(name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|(p, _, _)| is_in_scope(p, focus, exclude))
            .collect();
        let handlers: Vec<_> = be_commands
            .get(name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|(p, _, _)| is_in_scope(p, focus, exclude))
            .collect();

        let mut handler_emits = Vec::new();
        for (path, _line, handler_name) in &handlers {
            if let Some(evts) = path_emit_map.get(path) {
                for evt in evts {
                    handler_emits.push(json!({
                        "name": evt.raw,
                        "normalized": evt.norm,
                        "path": path,
                        "line": evt.line,
                        "handler": handler_name,
                    }));
                }
            }
        }

        let status = if handlers.is_empty() && !calls.is_empty() {
            "missing_handler"
        } else if calls.is_empty() && !handlers.is_empty() {
            "unused_handler"
        } else {
            "ok"
        };

        chains.push(json!({
            "name": name,
            "status": status,
            "callCount": calls.len(),
            "handlerCount": handlers.len(),
            "calls": calls.iter().map(|(p,l,alias)| json!({"path": p, "line": l, "alias": alias})).collect::<Vec<_>>(),
            "handlers": handlers.iter().map(|(p,l,alias)| json!({"path": p, "line": l, "name": alias})).collect::<Vec<_>>(),
            "handlerEmits": handler_emits,
        }));
    }
    chains.sort_by(|a, b| {
        let a_name = a["name"].as_str().unwrap_or("");
        let b_name = b["name"].as_str().unwrap_or("");
        a_name.cmp(b_name)
    });

    let stats = json!({
        "emitters": events.values().map(|r| r.emitters.len()).sum::<usize>(),
        "listeners": events.values().map(|r| r.listeners.len()).sum::<usize>(),
        "distinctEmitted": events.values().filter(|r| !r.emitters.is_empty()).count(),
        "distinctListened": events.values().filter(|r| !r.listeners.is_empty()).count(),
        "matched": events.values().filter(|r| !r.emitters.is_empty() && !r.listeners.is_empty()).count(),
        "ghostCount": ghost_emits.len(),
        "orphanCount": orphan_listeners.len(),
    });

    json!({
        "events": {
            "items": event_items,
            "ghostEmits": ghost_emits,
            "orphanListeners": orphan_listeners,
            "stats": stats,
        },
        "commands": {
            "chains": chains,
            "stats": {
                "total": total_commands,
                "withCalls": fe_commands.len(),
                "withHandlers": be_commands.len(),
            }
        },
        "risks": risks,
    })
}
