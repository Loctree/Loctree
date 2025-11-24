use std::collections::HashSet;

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
) -> serde_json::Value {
    #[derive(Clone)]
    struct Site {
        norm: String,
        name: String,
        path: String,
        line: usize,
    }

    let mut emits: Vec<Site> = Vec::new();
    let mut listens: Vec<Site> = Vec::new();

    for analysis in analyses {
        let path = analysis.path.clone();
        for ev in &analysis.event_emits {
            emits.push(Site {
                norm: normalize_event(&ev.name),
                name: ev.name.clone(),
                path: path.clone(),
                line: ev.line,
            });
        }
        for ev in &analysis.event_listens {
            listens.push(Site {
                norm: normalize_event(&ev.name),
                name: ev.name.clone(),
                path: path.clone(),
                line: ev.line,
            });
        }
    }

    let emit_names: HashSet<String> = emits.iter().map(|s| s.norm.clone()).collect();
    let listen_names: HashSet<String> = listens.iter().map(|s| s.norm.clone()).collect();

    let mut ghost_emits = Vec::new();
    for site in &emits {
        if !listen_names.contains(&site.norm) || !is_in_scope(&site.path, focus, exclude) {
            if is_in_scope(&site.path, focus, exclude) {
                ghost_emits.push(json!({
                    "name": site.name,
                    "path": site.path,
                    "line": site.line,
                }));
            }
        }
    }

    let mut orphan_listeners = Vec::new();
    for site in &listens {
        if !emit_names.contains(&site.norm) || !is_in_scope(&site.path, focus, exclude) {
            if is_in_scope(&site.path, focus, exclude) {
                orphan_listeners.push(json!({
                    "name": site.name,
                    "path": site.path,
                    "line": site.line,
                }));
            }
        }
    }

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

    let stats = json!({
        "emitters": emits.len(),
        "listeners": listens.len(),
        "distinctEmitted": emit_names.len(),
        "distinctListened": listen_names.len(),
        "matched": emit_names.intersection(&listen_names).count(),
    });

    json!({
        "events": {
            "ghostEmits": ghost_emits,
            "orphanListeners": orphan_listeners,
            "stats": stats,
        }
    })
}
