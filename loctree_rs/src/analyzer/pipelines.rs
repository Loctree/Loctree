use std::collections::{HashMap, HashSet};

use globset::GlobSet;
use serde_json::json;

use crate::analyzer::coverage::CommandUsage;
use crate::types::{FileAnalysis, PayloadMap};

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
    if let Some(ex) = exclude
        && ex.is_match(pb)
    {
        return false;
    }
    if let Some(focus_globs) = focus
        && !focus_globs.is_match(pb)
    {
        return false;
    }
    true
}

pub fn build_pipeline_summary(
    analyses: &[FileAnalysis],
    focus: &Option<GlobSet>,
    exclude: &Option<GlobSet>,
    fe_commands: &CommandUsage,
    be_commands: &CommandUsage,
    fe_payloads: &PayloadMap,
    be_payloads: &PayloadMap,
) -> serde_json::Value {
    #[derive(Clone)]
    struct Site {
        norm: String,
        raw: String,
        path: String,
        line: usize,
        awaited: bool,
        payload: Option<String>,
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
            let raw_display = ev.raw_name.clone().unwrap_or_else(|| ev.name.clone());
            let norm = normalize_event(&ev.name);
            let site = Site {
                norm: norm.clone(),
                raw: raw_display.clone(),
                path: path.clone(),
                line: ev.line,
                awaited: ev.awaited,
                payload: ev.payload.clone(),
            };
            path_emit_map
                .entry(path.clone())
                .or_default()
                .push(site.clone());
            let rec = events.entry(norm).or_default();
            rec.raw_names.insert(raw_display);
            rec.emitters.push(site);
        }
        for ev in &analysis.event_listens {
            let raw_display = ev.raw_name.clone().unwrap_or_else(|| ev.name.clone());
            let norm = normalize_event(&ev.name);
            let site = Site {
                norm: norm.clone(),
                raw: raw_display.clone(),
                path: path.clone(),
                line: ev.line,
                awaited: ev.awaited,
                payload: ev.payload.clone(),
            };
            let rec = events.entry(norm).or_default();
            rec.raw_names.insert(raw_display);
            rec.listeners.push(site);
        }
    }

    let mut event_items = Vec::new();
    let mut ghost_emits = Vec::new();
    let mut orphan_listeners = Vec::new();
    let mut risks = Vec::new();
    let mut call_payloads: PayloadMap = HashMap::new();
    let mut handler_payloads: PayloadMap = HashMap::new();

    for (name, entries) in fe_payloads {
        call_payloads
            .entry(name.clone())
            .or_default()
            .extend(entries.clone());
    }
    for (name, entries) in be_payloads {
        handler_payloads
            .entry(name.clone())
            .or_default()
            .extend(entries.clone());
    }

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
                let mut confidence = "high";
                let mut recommendation = "safe_to_remove";

                let is_literal = site.raw.starts_with('"') || site.raw.starts_with('\'');
                let is_tauri = site.raw.contains("tauri://");
                let is_template = !is_literal && site.raw.contains('`');

                if is_tauri {
                    confidence = "low";
                    recommendation = "check_system_docs";
                } else if is_template || site.raw.contains("${") {
                    confidence = "low";
                    recommendation = "verify_dynamic_value";
                } else if !is_literal {
                    // Identifier or variable that wasn't resolved to a literal
                    confidence = "low";
                    recommendation = "verify_variable_value";
                }

                ghost_emits.push(json!({
                    "name": site.raw,
                    "path": site.path,
                    "line": site.line,
                    "normalized": norm,
                    "payload": site.payload,
                    "confidence": confidence,
                    "recommendation": recommendation,
                }));
            }
        }
        if status == "orphan" {
            for site in &listeners {
                if site.raw.starts_with("tauri://") {
                    continue;
                }
                orphan_listeners.push(json!({
                    "name": site.raw,
                    "path": site.path,
                    "line": site.line,
                    "normalized": norm,
                    "awaited": site.awaited,
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
            "emitters": emitters.iter().map(|s| json!({"path": s.path, "line": s.line, "name": s.raw, "payload": s.payload})).collect::<Vec<_>>(),
            "listeners": listeners.iter().map(|s| json!({"path": s.path, "line": s.line, "name": s.raw, "awaited": s.awaited})).collect::<Vec<_>>(),
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
    // and listeners that are never awaited.
    for analysis in analyses {
        if !is_in_scope(&analysis.path, focus, exclude) {
            continue;
        }
        if analysis.command_calls.is_empty() || analysis.event_listens.is_empty() {
            continue;
        }
        let first_call = analysis
            .command_calls
            .iter()
            .min_by_key(|c| c.line)
            .cloned();
        let first_listen = analysis
            .event_listens
            .iter()
            .min_by_key(|e| e.line)
            .cloned();
        let first_awaited = analysis
            .event_listens
            .iter()
            .filter(|e| e.awaited)
            .min_by_key(|e| e.line)
            .cloned();

        if let (Some(call), Some(listen)) = (first_call.clone(), first_listen.clone())
            && call.line < listen.line
        {
            risks.push(json!({
                "type": "invoke_before_listen",
                "path": analysis.path,
                "line": call.line,
                "command": call.name,
                "details": "invoke is called before any listener is registered; event may be missed"
            }));
        }

        if let Some(listen) = first_listen {
            if !listen.awaited {
                risks.push(json!({
                    "type": "listen_not_awaited",
                    "path": analysis.path,
                    "line": listen.line,
                    "details": "listener registration is not awaited; first events may race"
                }));
            } else if let Some(call) = first_call
                && let Some(aw) = first_awaited
                && call.line < aw.line
            {
                risks.push(json!({
                    "type": "invoke_before_awaited_listen",
                    "path": analysis.path,
                    "line": call.line,
                    "details": "invoke is issued before awaited listener is registered",
                    "command": call.name,
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
                        "payload": evt.payload,
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
            "calls": calls.iter().map(|(p,l,alias)| {
                let payload = call_payloads
                    .get(name)
                    .and_then(|entries| entries.iter().find(|(pp,ll,_)| pp == p && *ll == *l))
                    .and_then(|(_,_,pl)| pl.clone());
                json!({"path": p, "line": l, "alias": alias, "payload": payload})
            }).collect::<Vec<_>>(),
            "handlers": handlers.iter().map(|(p,l,alias)| {
                let payload = handler_payloads
                    .get(name)
                    .and_then(|entries| entries.iter().find(|(pp,ll,_)| pp == p && *ll == *l))
                    .and_then(|(_,_,pl)| pl.clone());
                json!({"path": p, "line": l, "name": alias, "payload": payload})
            }).collect::<Vec<_>>(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CommandRef, EventRef, FileAnalysis};

    fn mk_event(name: &str, line: usize, kind: &str, awaited: bool) -> EventRef {
        EventRef {
            raw_name: Some(name.to_string()),
            name: name.to_string(),
            line,
            kind: kind.to_string(),
            awaited,
            payload: None,
        }
    }

    #[test]
    fn detects_ghost_orphan_and_command_chain_status() {
        let mut fe_commands: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();
        fe_commands.insert(
            "unified_ai_chat".into(),
            vec![("src/frontend.ts".into(), 3, "unified_ai_chat".into())],
        );
        fe_commands.insert(
            "missing_cmd".into(),
            vec![("src/frontend.ts".into(), 4, "missing_cmd".into())],
        );

        let mut be_commands: HashMap<String, Vec<(String, usize, String)>> = HashMap::new();
        be_commands.insert(
            "unified_ai_chat".into(),
            vec![("src/backend.rs".into(), 15, "unified_ai_chat".into())],
        );
        be_commands.insert(
            "unused_cmd".into(),
            vec![("src/backend.rs".into(), 20, "unused_cmd".into())],
        );

        let fe_payloads: HashMap<String, Vec<(String, usize, Option<String>)>> = HashMap::new();
        let be_payloads: HashMap<String, Vec<(String, usize, Option<String>)>> = HashMap::new();

        // FE file with matching emit/listen
        let mut fe = FileAnalysis::new("src/frontend.ts".into());
        fe.event_emits
            .push(mk_event("vista://ok", 10, "emit_literal", false));
        fe.event_listens
            .push(mk_event("vista://ok", 5, "listen_literal", true));
        fe.command_calls.push(CommandRef {
            name: "unified_ai_chat".into(),
            exposed_name: None,
            line: 3,
            generic_type: None,
            payload: None,
        });

        // BE file emitting ghost event and handling command
        let mut be = FileAnalysis::new("src/backend.rs".into());
        be.event_emits
            .push(mk_event("vista://ghost", 20, "emit_literal", false));
        be.command_handlers.push(CommandRef {
            name: "unified_ai_chat".into(),
            exposed_name: None,
            line: 15,
            generic_type: None,
            payload: None,
        });

        // Racy file: invoke before listener and not awaited
        let mut racy = FileAnalysis::new("src/racy.ts".into());
        racy.command_calls.push(CommandRef {
            name: "racy_cmd".into(),
            exposed_name: None,
            line: 1,
            generic_type: None,
            payload: None,
        });
        racy.event_listens
            .push(mk_event("vista://racy", 10, "listen_literal", false));

        let analyses = vec![fe, be, racy];
        let summary = build_pipeline_summary(
            &analyses,
            &None,
            &None,
            &fe_commands,
            &be_commands,
            &fe_payloads,
            &be_payloads,
        );

        let events = summary["events"]
            .as_object()
            .expect("events section present");
        let ghost = events["ghostEmits"]
            .as_array()
            .expect("ghostEmits array present");
        assert!(ghost.iter().any(|g| g["name"] == "vista://ghost"));

        let orphans = events["orphanListeners"]
            .as_array()
            .expect("orphanListeners array present");
        assert!(orphans.iter().any(|o| o["name"] == "vista://racy"));

        let chains = summary["commands"]["chains"]
            .as_array()
            .expect("chains array");
        let status_map: HashMap<_, _> = chains
            .iter()
            .map(|c| {
                (
                    c.get("name").and_then(|n| n.as_str()).unwrap_or_default(),
                    c.get("status").and_then(|s| s.as_str()).unwrap_or_default(),
                )
            })
            .collect();
        assert_eq!(status_map.get("unified_ai_chat"), Some(&"ok"));
        assert_eq!(status_map.get("missing_cmd"), Some(&"missing_handler"));
        assert_eq!(status_map.get("unused_cmd"), Some(&"unused_handler"));

        let risks = summary["risks"].as_array().expect("risks array present");
        assert!(risks.iter().any(|r| r["type"] == "invoke_before_listen"));
        assert!(risks.iter().any(|r| r["type"] == "listen_not_awaited"));
    }
}
