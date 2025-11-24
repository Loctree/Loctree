use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

fn resolve_with_extensions(
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
