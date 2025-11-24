use std::fs;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::json;

use crate::types::FileAnalysis;

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

fn load_tsconfig(root: &Path) -> Option<serde_json::Value> {
    let ts_path = root.join("tsconfig.json");
    if !ts_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&ts_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn summarize_tsconfig(root: &Path, analyses: &[FileAnalysis]) -> serde_json::Value {
    let Some(tsconfig) = load_tsconfig(root) else {
        return json!({"found": false});
    };

    let compiler = tsconfig
        .get("compilerOptions")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let base_url = compiler
        .get("baseUrl")
        .and_then(|v| v.as_str())
        .unwrap_or(".")
        .to_string();
    let base_path = root.join(&base_url);

    let mut alias_entries = Vec::new();
    if let Some(paths) = compiler.get("paths").and_then(|p| p.as_object()) {
        for (alias, targets) in paths.iter() {
            if let Some(first) = targets.as_array().and_then(|arr| arr.first()) {
                if let Some(target_str) = first.as_str() {
                    let normalized = target_str.replace('\\', "/");
                    let target_dir = normalized.replace("/*", "").replace('*', "");
                    let resolved = base_path.join(&target_dir);
                    let exists = resolved.exists();
                    alias_entries.push(json!({
                        "alias": alias,
                        "target": target_str,
                        "resolved": resolved.display().to_string(),
                        "exists": exists,
                    }));
                }
            }
        }
    }

    let include_patterns: Vec<String> = tsconfig
        .get("include")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.replace('\\', "/")))
                .collect()
        })
        .unwrap_or_default();
    let exclude_patterns: Vec<String> = tsconfig
        .get("exclude")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.replace('\\', "/")))
                .collect()
        })
        .unwrap_or_default();

    let include_set = build_globset(&include_patterns);
    let exclude_set = build_globset(&exclude_patterns);

    let mut outside_include = Vec::new();
    let mut excluded_samples = Vec::new();
    for analysis in analyses {
        let rel = analysis.path.replace('\\', "/");
        let path_obj = PathBuf::from(&rel);
        let included = include_set
            .as_ref()
            .map(|set| set.is_match(&path_obj))
            .unwrap_or(true);
        let excluded = exclude_set
            .as_ref()
            .map(|set| set.is_match(&path_obj))
            .unwrap_or(false);

        if excluded {
            if excluded_samples.len() < 8 {
                excluded_samples.push(rel.clone());
            }
            continue;
        }
        if include_set.is_some() && !included && outside_include.len() < 8 {
            outside_include.push(rel.clone());
        }
    }

    let unresolved: Vec<_> = alias_entries
        .iter()
        .filter(|entry| {
            !entry
                .get("exists")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    json!({
        "found": true,
        "baseUrl": base_path.display().to_string(),
        "aliasCount": alias_entries.len(),
        "aliases": alias_entries,
        "unresolvedAliases": unresolved,
        "includeCount": include_patterns.len(),
        "excludeCount": exclude_patterns.len(),
        "outsideIncludeSamples": outside_include,
        "excludedSamples": excluded_samples,
    })
}
