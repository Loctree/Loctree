use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json;

/// Simple TS/JS path resolver backed by tsconfig.json `baseUrl` and `paths`.
/// Supports alias patterns with a single `*` and falls back to `baseUrl`.
#[derive(Debug, Clone)]
pub(crate) struct TsPathResolver {
    base_dir: PathBuf,
    root: PathBuf,
    mappings: Vec<AliasMapping>,
}

#[derive(Debug, Clone)]
struct AliasMapping {
    prefix: String,
    suffix: String,
    targets: Vec<String>,
    has_wildcard: bool,
}

impl TsPathResolver {
    pub(crate) fn from_tsconfig(root: &Path) -> Option<Self> {
        let ts_path = root.join("tsconfig.json");
        if !ts_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&ts_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        let compiler = json
            .get("compilerOptions")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let base_url = compiler
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let base_dir = root.join(base_url);

        let mut mappings = Vec::new();
        if let Some(paths) = compiler.get("paths").and_then(|p| p.as_object()) {
            for (alias, targets) in paths {
                let targets_vec: Vec<String> = targets
                    .as_array()
                    .into_iter()
                    .flat_map(|arr| arr.iter())
                    .filter_map(|v| v.as_str())
                    .map(|s| s.replace('\\', "/"))
                    .collect();
                if targets_vec.is_empty() {
                    continue;
                }

                let alias_norm = alias.replace('\\', "/");
                let has_wildcard = alias_norm.contains('*');
                let (prefix, suffix) = if let Some((pre, suf)) = alias_norm.split_once('*') {
                    (pre.to_string(), suf.to_string())
                } else {
                    (alias_norm.clone(), String::new())
                };

                mappings.push(AliasMapping {
                    prefix,
                    suffix,
                    targets: targets_vec,
                    has_wildcard,
                });
            }
        }

        Some(Self {
            base_dir,
            root: root.to_path_buf(),
            mappings,
        })
    }

    pub(crate) fn resolve(&self, spec: &str, exts: Option<&HashSet<String>>) -> Option<String> {
        if spec.starts_with('.') {
            return None;
        }

        let normalized = spec.replace('\\', "/");

        for mapping in &self.mappings {
            if mapping.has_wildcard {
                if let Some(rest) = normalized.strip_prefix(&mapping.prefix) {
                    if rest.ends_with(&mapping.suffix) {
                        let mid = rest
                            .strip_suffix(&mapping.suffix)
                            .unwrap_or(rest)
                            .to_string();
                        for target in &mapping.targets {
                            let replaced = target.replace('*', &mid);
                            let candidate = self.base_dir.join(replaced);
                            if let Some(res) = resolve_with_extensions(candidate, &self.root, exts)
                            {
                                return Some(res);
                            }
                        }
                    }
                }
            } else if normalized == mapping.prefix {
                for target in &mapping.targets {
                    let candidate = self.base_dir.join(target);
                    if let Some(res) = resolve_with_extensions(candidate, &self.root, exts) {
                        return Some(res);
                    }
                }
            }
        }

        if normalized.starts_with('/') {
            let candidate = self.root.join(normalized.trim_start_matches('/'));
            return resolve_with_extensions(candidate, &self.root, exts);
        }

        let candidate = self.base_dir.join(&normalized);
        resolve_with_extensions(candidate, &self.root, exts)
    }
}

pub(crate) fn resolve_reexport_target(
    file_path: &Path,
    root: &Path,
    spec: &str,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    if !spec.starts_with('.') {
        return None;
    }
    let parent = file_path.parent()?;
    let candidate = parent.join(spec);
    if candidate.is_dir() {
        return None;
    }
    resolve_with_extensions(candidate, root, exts)
}

pub(crate) fn resolve_python_relative(
    module: &str,
    file_path: &Path,
    root: &Path,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    if !module.starts_with('.') {
        return None;
    }

    let mut leading = 0usize;
    for ch in module.chars() {
        if ch == '.' {
            leading += 1;
        } else {
            break;
        }
    }

    let mut base = file_path.parent()?;
    for _ in 1..leading {
        base = base.parent()?;
    }

    let remainder = module.trim_start_matches('.').replace('.', "/");
    let joined = if remainder.is_empty() {
        base.to_path_buf()
    } else {
        base.join(remainder)
    };

    if joined.is_dir() {
        return None;
    }

    resolve_with_extensions(joined, root, exts)
}

pub(crate) fn resolve_js_relative(
    file_path: &Path,
    root: &Path,
    spec: &str,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    if !spec.starts_with('.') {
        return None;
    }
    let parent = file_path.parent()?;
    let candidate = parent.join(spec);
    resolve_with_extensions(candidate, root, exts)
}

pub(crate) fn resolve_with_extensions(
    candidate: PathBuf,
    root: &Path,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    if candidate.extension().is_none() {
        if let Some(set) = exts {
            for ext in set {
                let with_ext = candidate.with_extension(ext);
                if with_ext.exists() {
                    return canonical_rel(&with_ext, root).or_else(|| canonical_abs(&with_ext));
                }
            }
        }
    }

    if candidate.exists() {
        canonical_rel(&candidate, root).or_else(|| canonical_abs(&candidate))
    } else {
        None
    }
}

fn canonical_rel(path: &Path, root: &Path) -> Option<String> {
    path.canonicalize().ok().and_then(|p| {
        p.strip_prefix(root)
            .ok()
            .map(|q| q.to_string_lossy().to_string())
    })
}

fn canonical_abs(path: &Path) -> Option<String> {
    path.canonicalize()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}
