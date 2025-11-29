use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::fs_utils::{GitIgnoreChecker, gather_files};
use crate::types::{FileAnalysis, Options};

use super::classify::{detect_language, file_kind};
use super::css::analyze_css_file;
use super::js::analyze_js_file;
use super::py::{analyze_py_file, python_stdlib_set};
use super::resolvers::{
    TsPathResolver, find_rust_crate_root, resolve_js_relative, resolve_python_relative,
    resolve_rust_import,
};
use super::rust::analyze_rust_file;

/// Build a globset from user patterns.
pub fn build_globset(patterns: &[String]) -> Option<GlobSet> {
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
    if !added { None } else { builder.build().ok() }
}

pub fn opt_globset(globs: &[String]) -> Option<GlobSet> {
    build_globset(globs).and_then(|g| if g.is_empty() { None } else { Some(g) })
}

pub fn strip_excluded(files: &[String], exclude: &Option<GlobSet>) -> Vec<String> {
    match exclude {
        None => files.to_vec(),
        Some(set) => files.iter().filter(|p| !set.is_match(p)).cloned().collect(),
    }
}

pub fn matches_focus(files: &[String], focus: &Option<GlobSet>) -> bool {
    match focus {
        None => true,
        Some(set) => files.iter().any(|p| set.is_match(p)),
    }
}

fn is_ident_like(raw: &str) -> bool {
    raw.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Resolve event names declared as constants across files. This mutates analyses in-place.
pub fn resolve_event_constants_across_files(analyses: &mut [FileAnalysis]) {
    let mut consts_by_path: HashMap<String, HashMap<String, String>> = HashMap::new();
    for a in analyses.iter() {
        if !a.event_consts.is_empty() {
            consts_by_path.insert(a.path.clone(), a.event_consts.clone());
        }
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut unique: HashMap<String, String> = HashMap::new();
    for map in consts_by_path.values() {
        for (name, val) in map {
            *counts.entry(name.clone()).or_insert(0) += 1;
            unique.entry(name.clone()).or_insert(val.clone());
        }
    }
    unique.retain(|k, _| counts.get(k) == Some(&1));

    for analysis in analyses.iter_mut() {
        for ev in analysis
            .event_emits
            .iter_mut()
            .chain(analysis.event_listens.iter_mut())
        {
            let raw = match ev.raw_name.clone() {
                Some(r) if is_ident_like(&r) => r,
                _ => continue,
            };

            let resolved = if let Some(val) = analysis.event_consts.get(&raw) {
                Some(val.clone())
            } else {
                let mut found: Option<String> = None;
                for imp in &analysis.imports {
                    if let Some(resolved_path) = &imp.resolved_path {
                        for sym in &imp.symbols {
                            let alias = sym.alias.as_ref().unwrap_or(&sym.name);
                            if alias == &raw
                                && let Some(map) = consts_by_path.get(resolved_path)
                                && let Some(val) = map.get(&sym.name)
                            {
                                found = Some(val.clone());
                            }
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
                found.or_else(|| unique.get(&raw).cloned())
            };

            if let Some(val) = resolved {
                ev.name = val;
                if ev.kind.starts_with("emit_ident") {
                    ev.kind = "emit_const".to_string();
                } else if ev.kind.starts_with("listen_ident") {
                    ev.kind = "listen_const".to_string();
                }
            }
        }
    }
}

pub fn analyze_file(
    path: &Path,
    root_canon: &Path,
    extensions: Option<&HashSet<String>>,
    ts_resolver: Option<&TsPathResolver>,
    py_roots: &[PathBuf],
    py_stdlib: &HashSet<String>,
    symbol: Option<&str>,
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
        "py" => analyze_py_file(
            &content, &canonical, root_canon, extensions, relative, py_roots, py_stdlib,
        ),
        _ => analyze_js_file(
            &content,
            &canonical,
            root_canon,
            extensions,
            ts_resolver,
            relative,
        ),
    };

    if let Some(sym) = symbol {
        for (i, line) in content.lines().enumerate() {
            if line.contains(sym) {
                analysis.matches.push(crate::types::SymbolMatch {
                    line: i + 1,
                    context: line.trim().to_string(),
                });
            }
        }
    }

    analysis.loc = loc;
    analysis.language = detect_language(&ext);
    let (kind, is_test, is_generated) = file_kind(&analysis.path);
    analysis.kind = kind;
    analysis.is_test = is_test;
    analysis.is_generated = is_generated;

    // Resolve Rust imports
    if ext == "rs" {
        let crate_root = find_rust_crate_root(&canonical);
        if let Some(ref crate_root) = crate_root {
            for imp in analysis.imports.iter_mut() {
                if imp.resolved_path.is_none() {
                    imp.resolved_path =
                        resolve_rust_import(&imp.source, &canonical, crate_root, root_canon);
                }
            }
        }
    }

    // Resolve other language imports (relative paths)
    for imp in analysis.imports.iter_mut() {
        if imp.resolved_path.is_none() && imp.source.starts_with('.') {
            let resolved = match ext.as_str() {
                "py" => resolve_python_relative(&imp.source, &canonical, root_canon, extensions),
                "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "css" => ts_resolver
                    .and_then(|r| r.resolve(&imp.source, extensions))
                    .or_else(|| {
                        resolve_js_relative(&canonical, root_canon, &imp.source, extensions)
                    }),
                _ => None,
            };
            imp.resolved_path = resolved;
        }
    }

    Ok(analysis)
}

/// Expand gather_files with gitignore handling. Returns the list of files and the visited set.
#[allow(dead_code)]
pub fn collect_files(
    root_path: &Path,
    options: &Options,
) -> io::Result<(Vec<PathBuf>, HashSet<PathBuf>)> {
    let git_checker = if options.use_gitignore {
        GitIgnoreChecker::new(root_path)
    } else {
        None
    };

    let mut files = Vec::new();
    let mut visited = HashSet::new();
    gather_files(
        root_path,
        options,
        0,
        git_checker.as_ref(),
        &mut visited,
        &mut files,
    )?;
    Ok((files, visited))
}

pub fn python_stdlib() -> HashSet<String> {
    python_stdlib_set().clone()
}
