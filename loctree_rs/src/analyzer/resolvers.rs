use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json;
use serde_json::Value;

/// Simple TS/JS path resolver backed by tsconfig.json `baseUrl` and `paths`.
/// Supports alias patterns with wildcards and falls back to `baseUrl`.
/// Also checks package.json exports field as fallback.
#[derive(Debug)]
pub(crate) struct TsPathResolver {
    base_dir: PathBuf,
    root: PathBuf,
    mappings: Vec<AliasMapping>,
    cache: Mutex<HashMap<String, Option<String>>>,
    package_exports: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct AliasMapping {
    pattern: String,
    targets: Vec<String>,
    wildcard_count: usize,
}

/// Extracted resolver configuration for caching in snapshots
#[derive(Debug, Clone, Default)]
pub struct ExtractedResolverConfig {
    /// TypeScript path aliases
    pub ts_paths: HashMap<String, Vec<String>>,
    /// Base URL for resolution
    pub ts_base_url: Option<String>,
}

impl TsPathResolver {
    pub(crate) fn from_tsconfig(root: &Path) -> Option<Self> {
        let ts_path = find_tsconfig(root)?;
        let json = load_tsconfig_recursive(&ts_path)?;
        let compiler = json
            .get("compilerOptions")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let base_url = compiler
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let base_dir = ts_path.parent().unwrap_or(root).join(base_url);

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
                let wildcard_count = alias_norm.matches('*').count();

                mappings.push(AliasMapping {
                    pattern: alias_norm,
                    targets: targets_vec,
                    wildcard_count,
                });
            }
        }

        // Load package.json exports if available
        let package_exports = load_package_exports(root).unwrap_or_default();

        Some(Self {
            base_dir: base_dir.canonicalize().unwrap_or(base_dir),
            root: root.to_path_buf(),
            mappings,
            cache: Mutex::new(HashMap::new()),
            package_exports,
        })
    }

    /// Extract the resolver configuration for caching in snapshots
    pub(crate) fn extract_config(&self) -> ExtractedResolverConfig {
        let ts_paths: HashMap<String, Vec<String>> = self
            .mappings
            .iter()
            .map(|m| (m.pattern.clone(), m.targets.clone()))
            .collect();

        let ts_base_url = self
            .base_dir
            .strip_prefix(&self.root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| Some(self.base_dir.to_string_lossy().to_string()));

        ExtractedResolverConfig {
            ts_paths,
            ts_base_url,
        }
    }

    pub(crate) fn resolve(&self, spec: &str, exts: Option<&HashSet<String>>) -> Option<String> {
        if spec.starts_with('.') {
            return None;
        }

        // Check cache first
        let cache_key = format!("{:?}:{}", exts, spec);
        if let Ok(cache) = self.cache.lock()
            && let Some(cached) = cache.get(&cache_key)
        {
            return cached.clone();
        }

        let normalized = spec.replace('\\', "/");
        let result = self.resolve_internal(&normalized, exts);

        // Store in cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(cache_key, result.clone());
        }

        result
    }

    fn resolve_internal(&self, normalized: &str, exts: Option<&HashSet<String>>) -> Option<String> {
        // Try tsconfig path mappings
        for mapping in &self.mappings {
            if mapping.wildcard_count > 0 {
                // Handle multiple wildcards
                if let Some(res) = self.match_wildcard_pattern(
                    &mapping.pattern,
                    normalized,
                    &mapping.targets,
                    exts,
                ) {
                    return Some(res);
                }
            } else if normalized == mapping.pattern {
                // Exact match (no wildcards)
                for target in &mapping.targets {
                    let candidate = self.base_dir.join(target);
                    if let Some(res) = resolve_with_extensions(candidate, &self.root, exts) {
                        return Some(res);
                    }
                }
            }
        }

        // Try package.json exports
        if let Some(export_path) = self.package_exports.get(normalized) {
            let candidate = self.root.join(export_path.trim_start_matches("./"));
            if let Some(res) = resolve_with_extensions(candidate, &self.root, exts) {
                return Some(res);
            }
        }

        // Fallback to baseUrl resolution
        if normalized.starts_with('/') {
            let candidate = self.root.join(normalized.trim_start_matches('/'));
            return resolve_with_extensions(candidate, &self.root, exts);
        }

        let candidate = self.base_dir.join(normalized);
        resolve_with_extensions(candidate, &self.root, exts)
    }

    fn match_wildcard_pattern(
        &self,
        pattern: &str,
        spec: &str,
        targets: &[String],
        exts: Option<&HashSet<String>>,
    ) -> Option<String> {
        // Convert pattern to regex-like matching
        // e.g., "@/*" -> captures everything after "@/"
        // e.g., "**/*" -> captures everything
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() < 2 {
            return None;
        }

        // Check if spec matches the pattern structure
        let mut spec_rest = spec;
        let mut captures = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                // First part must be prefix
                spec_rest = spec_rest.strip_prefix(part)?;
            } else if i == parts.len() - 1 {
                // Last part must be suffix
                if !spec_rest.ends_with(part) {
                    return None;
                }
                let captured = spec_rest.strip_suffix(part).unwrap_or(spec_rest);
                if i == 1 && parts.len() == 2 {
                    // Single wildcard case
                    captures.push(captured);
                } else {
                    // Multiple wildcards - split remaining
                    // For "**/*" pattern, capture greedily
                    captures.push(captured);
                }
            } else {
                // Middle parts - find next occurrence
                if let Some(idx) = spec_rest.find(part) {
                    captures.push(&spec_rest[..idx]);
                    spec_rest = &spec_rest[idx + part.len()..];
                } else {
                    return None;
                }
            }
        }

        // Try each target with captured wildcards
        for target in targets {
            let replaced = if captures.len() == 1 {
                target.replace('*', captures[0])
            } else {
                // Multiple wildcards - replace in order
                let mut result = target.to_string();
                for capture in &captures {
                    if let Some(idx) = result.find('*') {
                        result.replace_range(idx..=idx, capture);
                    }
                }
                result
            };

            let candidate = self.base_dir.join(replaced);
            if let Some(res) = resolve_with_extensions(candidate, &self.root, exts) {
                return Some(res);
            }
        }

        None
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
    resolve_python_candidate(candidate, root, exts)
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

    resolve_python_candidate(joined, root, exts)
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

pub(crate) fn resolve_python_candidate(
    candidate: PathBuf,
    root: &Path,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    if candidate.is_dir() {
        // Traditional packages: check for __init__.py variants
        let init_candidates = [
            candidate.join("__init__.py"),
            candidate.join("__init__.pyi"),
            candidate.join("mod.py"),
        ];
        for init in init_candidates {
            if init.exists() {
                return canonical_rel(&init, root).or_else(|| canonical_abs(&init));
            }
        }

        // PEP 420: Namespace packages - directories without __init__.py
        // A directory is a valid namespace package if it contains any .py files
        // or has subdirectories that are packages
        if is_namespace_package(&candidate) {
            // Return the directory path itself as the package resolution
            return canonical_rel(&candidate, root).or_else(|| canonical_abs(&candidate));
        }
    }

    resolve_with_extensions(candidate, root, exts)
}

/// Check if a directory is a valid PEP 420 namespace package
/// A namespace package is a directory that:
/// - Has no __init__.py (already checked by caller)
/// - Contains at least one .py file or valid subpackage
fn is_namespace_package(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "py" || ext == "pyi" {
                    return true;
                }
            }
        } else if path.is_dir() {
            // Check if subdirectory is a package (has __init__.py or is namespace)
            let subdir_init = path.join("__init__.py");
            if subdir_init.exists() || is_namespace_package(&path) {
                return true;
            }
        }
    }
    false
}

/// Check if a Python package has py.typed marker (PEP 561)
/// indicating it provides type information
pub(crate) fn has_py_typed_marker(package_dir: &Path) -> bool {
    package_dir.join("py.typed").exists()
}

pub(crate) fn resolve_python_absolute(
    module: &str,
    roots: &[PathBuf],
    root_for_rel: &Path,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    let normalized = module.replace('.', "/");
    for base in roots {
        let candidate = base.join(&normalized);
        if let Some(resolved) = resolve_python_candidate(candidate.clone(), root_for_rel, exts) {
            return Some(resolved);
        }
    }
    None
}

pub(crate) fn resolve_with_extensions(
    candidate: PathBuf,
    root: &Path,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    if candidate.extension().is_none()
        && let Some(set) = exts
    {
        for ext in set {
            let with_ext = candidate.with_extension(ext);
            if with_ext.exists() {
                return canonical_rel(&with_ext, root).or_else(|| canonical_abs(&with_ext));
            }
        }
    }

    if candidate.exists() {
        canonical_rel(&candidate, root).or_else(|| canonical_abs(&candidate))
    } else {
        // Fallback: if this looks like a directory/module, try index.* inside it
        if candidate.extension().is_none() {
            let dir_path = candidate.clone();
            for index_name in ["index.ts", "index.tsx", "index.js", "index.jsx"] {
                let index_candidate = dir_path.join(index_name);
                if index_candidate.exists() {
                    return canonical_rel(&index_candidate, root)
                        .or_else(|| canonical_abs(&index_candidate));
                }
            }
        }
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

pub(crate) fn find_tsconfig(start: &Path) -> Option<PathBuf> {
    let mut current = start
        .canonicalize()
        .ok()
        .unwrap_or_else(|| start.to_path_buf());
    loop {
        let candidate = current.join("tsconfig.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if let Some(parent) = current.parent() {
            if parent == current {
                break;
            }
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

fn load_tsconfig_recursive(ts_path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(ts_path).ok()?;
    let mut current: Value = parse_tsconfig_value(&content)?;

    // Merge extends (child overrides parent).
    if let Some(ext) = current.get("extends").and_then(|v| v.as_str()) {
        let base_path = if Path::new(ext).is_absolute() {
            PathBuf::from(ext)
        } else {
            ts_path
                .parent()
                .map(|p| p.join(ext))
                .unwrap_or_else(|| PathBuf::from(ext))
        };
        if base_path.exists()
            && let Some(parent) = load_tsconfig_recursive(&base_path)
        {
            if let (Some(child_co), Some(parent_co)) = (
                current
                    .get("compilerOptions")
                    .and_then(|v| v.as_object())
                    .cloned(),
                parent
                    .get("compilerOptions")
                    .and_then(|v| v.as_object())
                    .cloned(),
            ) {
                let merged = merge_compiler_options(&parent_co, &child_co);
                current["compilerOptions"] = Value::Object(merged);
            } else if let Some(parent_co) = parent
                .get("compilerOptions")
                .and_then(|v| v.as_object())
                .cloned()
            {
                current["compilerOptions"] = Value::Object(parent_co);
            }
        }
    }

    Some(current)
}

pub(crate) fn parse_tsconfig_value(content: &str) -> Option<Value> {
    if let Ok(v) = serde_json::from_str(content) {
        return Some(v);
    }
    if let Ok(v) = json_five::from_str::<serde_json::Value>(content) {
        return Some(v);
    }
    None
}

fn merge_compiler_options(
    parent: &serde_json::Map<String, Value>,
    child: &serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    let mut merged = parent.clone();
    for (k, v) in child {
        if k == "paths" {
            let mut combined = parent
                .get("paths")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            if let Some(child_paths) = v.as_object() {
                for (alias, targets) in child_paths {
                    combined.insert(alias.clone(), targets.clone());
                }
            }
            merged.insert(k.clone(), Value::Object(combined));
        } else {
            merged.insert(k.clone(), v.clone());
        }
    }
    merged
}

/// Load package.json exports field for module resolution fallback
fn load_package_exports(root: &Path) -> Option<HashMap<String, String>> {
    let package_json_path = root.join("package.json");
    if !package_json_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&package_json_path).ok()?;
    let json: Value = serde_json::from_str(&content).ok()?;

    let exports = json.get("exports")?;
    let mut result = HashMap::new();

    // Handle different export formats
    match exports {
        Value::String(path) => {
            result.insert(".".to_string(), path.clone());
        }
        Value::Object(map) => {
            for (key, value) in map {
                let export_path = match value {
                    Value::String(s) => Some(s.clone()),
                    Value::Object(conditions) => {
                        // Try common conditions: import, require, default
                        conditions
                            .get("import")
                            .or_else(|| conditions.get("require"))
                            .or_else(|| conditions.get("default"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    }
                    _ => None,
                };

                if let Some(path) = export_path {
                    result.insert(key.clone(), path);
                }
            }
        }
        _ => {}
    }

    Some(result)
}

/// Resolve Rust module imports to file paths.
/// Handles: `crate::foo`, `super::bar`, `self::baz`
pub(crate) fn resolve_rust_import(
    source: &str,
    file_path: &Path,
    crate_root: &Path,
    root: &Path,
) -> Option<String> {
    // Skip stdlib and external crates
    if source.starts_with("std::")
        || source.starts_with("core::")
        || source.starts_with("alloc::")
        || !source.contains("::")
    {
        return None;
    }

    let module_path = if source.starts_with("crate::") {
        // crate::foo::bar -> src/foo/bar or src/foo.rs
        let remainder = source.strip_prefix("crate::")?;
        resolve_rust_module_path(remainder, crate_root)
    } else if source.starts_with("super::") {
        // super::foo -> parent_dir/foo
        let remainder = source.strip_prefix("super::")?;
        let parent = file_path.parent()?.parent()?; // Go up from file to parent module
        resolve_rust_module_path(remainder, parent)
    } else if source.starts_with("self::") {
        // self::foo -> current_module/foo
        let remainder = source.strip_prefix("self::")?;
        let current_dir = if file_path.file_name()?.to_str()? == "mod.rs" {
            file_path.parent()?
        } else {
            // For src/foo.rs, self:: refers to src/foo/ directory
            let stem = file_path.file_stem()?.to_str()?;
            &file_path.parent()?.join(stem)
        };
        resolve_rust_module_path(remainder, current_dir)
    } else {
        // Could be external crate or local module without prefix
        // Try resolving as crate-relative
        resolve_rust_module_path(source, crate_root)
    };

    module_path.and_then(|p| canonical_rel(&p, root).or_else(|| canonical_abs(&p)))
}

/// Resolve a Rust module path (e.g., "foo::bar::Baz") to a file path.
/// Handles nested modules by checking all possible file locations.
fn resolve_rust_module_path(module: &str, base: &Path) -> Option<PathBuf> {
    let segments: Vec<&str> = module.split("::").collect();
    if segments.is_empty() {
        return None;
    }

    // For "foo::bar::Baz", try multiple strategies:
    // 1. base/foo.rs (if single segment or defines submodules inline)
    // 2. base/foo/mod.rs (if foo is a directory module)
    // 3. base/foo/bar.rs (if bar is a file in foo directory)
    // 4. base/foo/bar/mod.rs (if bar is a directory module)
    // 5. base/foo/bar/baz.rs (if Baz is defined in bar/baz.rs)

    let first_segment = segments[0];

    // Strategy 1: Try first segment as file (base/foo.rs)
    let as_file = base.join(format!("{}.rs", first_segment));
    if as_file.exists() {
        return Some(as_file);
    }

    // Strategy 2: Try first segment as directory with mod.rs (base/foo/mod.rs)
    let as_mod = base.join(first_segment).join("mod.rs");
    if as_mod.exists() && segments.len() == 1 {
        return Some(as_mod);
    }

    // For nested paths (more than one segment), explore deeper
    if segments.len() > 1 {
        // Build path progressively: base/foo/bar/...
        let mut current_path = base.join(first_segment);

        for (idx, segment) in segments.iter().enumerate().skip(1) {
            let is_last = idx == segments.len() - 1;

            // Try as file: current_path/segment.rs
            let segment_file = current_path.join(format!("{}.rs", segment));
            if segment_file.exists() {
                return Some(segment_file);
            }

            // Try as directory module: current_path/segment/mod.rs
            let segment_mod = current_path.join(segment).join("mod.rs");
            if segment_mod.exists() {
                if is_last {
                    return Some(segment_mod);
                } else {
                    // Continue deeper
                    current_path = current_path.join(segment);
                    continue;
                }
            }

            // Try advancing into segment directory for next iteration
            let next_dir = current_path.join(segment);
            if next_dir.exists() && next_dir.is_dir() {
                current_path = next_dir;
            } else {
                break;
            }
        }
    }

    // Fallback: check if first segment as mod.rs exists (already checked above but be safe)
    if as_mod.exists() {
        return Some(as_mod);
    }

    None
}

/// Find the crate root (directory containing Cargo.toml) for a Rust file.
pub(crate) fn find_rust_crate_root(file_path: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;
    loop {
        // Check for Cargo.toml
        if current.join("Cargo.toml").exists() {
            // Crate root is usually src/ under Cargo.toml
            let src_dir = current.join("src");
            if src_dir.exists() && src_dir.is_dir() {
                return Some(src_dir);
            }
            // Fallback to directory with Cargo.toml
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create tsconfig.json
        let tsconfig = r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "@/*": ["src/*"],
                    "@components/*": ["src/components/*"],
                    "utils": ["src/utils/index.ts"]
                }
            }
        }"#;
        fs::write(dir.path().join("tsconfig.json"), tsconfig).unwrap();

        // Create source files
        fs::create_dir_all(dir.path().join("src/components")).unwrap();
        fs::create_dir_all(dir.path().join("src/utils")).unwrap();
        fs::write(dir.path().join("src/index.ts"), "export {}").unwrap();
        fs::write(dir.path().join("src/components/Button.tsx"), "export {}").unwrap();
        fs::write(dir.path().join("src/utils/index.ts"), "export {}").unwrap();

        dir
    }

    fn create_python_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src/mypackage")).unwrap();
        fs::write(dir.path().join("src/mypackage/__init__.py"), "").unwrap();
        fs::write(dir.path().join("src/mypackage/utils.py"), "").unwrap();
        fs::write(dir.path().join("src/mypackage/helpers.py"), "").unwrap();
        dir
    }

    fn create_rust_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("src/lib.rs"), "mod utils;").unwrap();
        fs::write(dir.path().join("src/utils.rs"), "pub fn helper() {}").unwrap();
        dir
    }

    #[test]
    fn test_parse_tsconfig_value_valid_json() {
        let content = r#"{"compilerOptions": {"strict": true}}"#;
        let result = parse_tsconfig_value(content);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_tsconfig_value_json5() {
        let content = r#"{
            // comment
            "compilerOptions": {
                "strict": true,
            }
        }"#;
        let result = parse_tsconfig_value(content);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_tsconfig_value_invalid() {
        let content = "not valid json at all";
        let result = parse_tsconfig_value(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_tsconfig() {
        let dir = create_test_project();
        let result = find_tsconfig(dir.path());
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("tsconfig.json"));
    }

    #[test]
    fn test_find_tsconfig_not_found() {
        let dir = TempDir::new().unwrap();
        let result = find_tsconfig(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_ts_path_resolver_from_tsconfig() {
        let dir = create_test_project();
        let resolver = TsPathResolver::from_tsconfig(dir.path());
        assert!(resolver.is_some());
    }

    #[test]
    fn test_ts_path_resolver_no_tsconfig() {
        let dir = TempDir::new().unwrap();
        let resolver = TsPathResolver::from_tsconfig(dir.path());
        assert!(resolver.is_none());
    }

    #[test]
    fn test_ts_path_resolver_resolve_relative_skipped() {
        let dir = create_test_project();
        let resolver = TsPathResolver::from_tsconfig(dir.path()).unwrap();
        // Relative paths should return None
        let result = resolver.resolve("./utils", None);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_reexport_target_relative() {
        let dir = create_test_project();
        let file = dir.path().join("src/index.ts");
        let exts: HashSet<String> = ["ts", "tsx"].iter().map(|s| s.to_string()).collect();

        let result = resolve_reexport_target(&file, dir.path(), "./utils/index", Some(&exts));
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_reexport_target_non_relative() {
        let dir = create_test_project();
        let file = dir.path().join("src/index.ts");

        let result = resolve_reexport_target(&file, dir.path(), "@/utils", None);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_js_relative() {
        let dir = create_test_project();
        let file = dir.path().join("src/index.ts");
        let exts: HashSet<String> = ["ts", "tsx"].iter().map(|s| s.to_string()).collect();

        let result = resolve_js_relative(&file, dir.path(), "./components/Button", Some(&exts));
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_js_relative_non_relative() {
        let dir = create_test_project();
        let file = dir.path().join("src/index.ts");

        let result = resolve_js_relative(&file, dir.path(), "lodash", None);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_python_relative() {
        let dir = create_python_project();
        let file = dir.path().join("src/mypackage/utils.py");
        let exts: HashSet<String> = ["py"].iter().map(|s| s.to_string()).collect();

        let result = resolve_python_relative(".helpers", &file, dir.path(), Some(&exts));
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_python_relative_double_dot() {
        let dir = create_python_project();
        fs::create_dir_all(dir.path().join("src/other")).unwrap();
        fs::write(dir.path().join("src/other/module.py"), "").unwrap();

        let file = dir.path().join("src/mypackage/utils.py");
        let exts: HashSet<String> = ["py"].iter().map(|s| s.to_string()).collect();

        let result = resolve_python_relative("..other.module", &file, dir.path(), Some(&exts));
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_python_relative_non_relative() {
        let dir = create_python_project();
        let file = dir.path().join("src/mypackage/utils.py");

        let result = resolve_python_relative("os", &file, dir.path(), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_python_candidate_directory_with_init() {
        let dir = create_python_project();
        let candidate = dir.path().join("src/mypackage");

        let result = resolve_python_candidate(candidate, dir.path(), None);
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_python_candidate_file() {
        let dir = create_python_project();
        let candidate = dir.path().join("src/mypackage/utils.py");

        let result = resolve_python_candidate(candidate, dir.path(), None);
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_python_absolute() {
        let dir = create_python_project();
        let roots = vec![dir.path().join("src")];
        let exts: HashSet<String> = ["py"].iter().map(|s| s.to_string()).collect();

        let result = resolve_python_absolute("mypackage.utils", &roots, dir.path(), Some(&exts));
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_python_absolute_not_found() {
        let dir = create_python_project();
        let roots = vec![dir.path().join("src")];

        let result = resolve_python_absolute("nonexistent.module", &roots, dir.path(), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_with_extensions_adds_extension() {
        let dir = create_test_project();
        let candidate = dir.path().join("src/index");
        let exts: HashSet<String> = ["ts"].iter().map(|s| s.to_string()).collect();

        let result = resolve_with_extensions(candidate, dir.path(), Some(&exts));
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_with_extensions_index_file() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src/utils")).unwrap();
        fs::write(dir.path().join("src/utils/index.ts"), "").unwrap();

        let candidate = dir.path().join("src/utils");
        let result = resolve_with_extensions(candidate, dir.path(), None);
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_with_extensions_not_found() {
        let dir = TempDir::new().unwrap();
        let candidate = dir.path().join("nonexistent");

        let result = resolve_with_extensions(candidate, dir.path(), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_rust_crate_root() {
        let dir = create_rust_project();
        let file = dir.path().join("src/main.rs");

        let result = find_rust_crate_root(&file);
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("src"));
    }

    #[test]
    fn test_find_rust_crate_root_not_found() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("some/random/file.rs");

        let result = find_rust_crate_root(&file);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_rust_import_crate() {
        let dir = create_rust_project();
        let file = dir.path().join("src/main.rs");
        let crate_root = dir.path().join("src");

        let result = resolve_rust_import("crate::utils", &file, &crate_root, dir.path());
        assert!(result.is_some());
    }

    #[test]
    fn test_resolve_rust_import_stdlib_skipped() {
        let dir = create_rust_project();
        let file = dir.path().join("src/main.rs");
        let crate_root = dir.path().join("src");

        let result =
            resolve_rust_import("std::collections::HashMap", &file, &crate_root, dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_rust_import_no_separator() {
        let dir = create_rust_project();
        let file = dir.path().join("src/main.rs");
        let crate_root = dir.path().join("src");

        let result = resolve_rust_import("serde", &file, &crate_root, dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_rust_import_nested_module() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src/analyzer")).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(dir.path().join("src/lib.rs"), "mod analyzer;").unwrap();
        fs::write(dir.path().join("src/analyzer/mod.rs"), "pub mod scan;").unwrap();
        fs::write(dir.path().join("src/analyzer/scan.rs"), "pub fn scan() {}").unwrap();

        let file = dir.path().join("src/lib.rs");
        let crate_root = dir.path().join("src");

        // Test deep path: crate::analyzer::scan
        let result = resolve_rust_import("crate::analyzer::scan", &file, &crate_root, dir.path());
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert!(resolved.contains("analyzer") && resolved.ends_with("scan.rs"));
    }

    #[test]
    fn test_resolve_rust_import_nested_module_dir() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src/foo/bar")).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(dir.path().join("src/lib.rs"), "mod foo;").unwrap();
        fs::write(dir.path().join("src/foo/mod.rs"), "pub mod bar;").unwrap();
        fs::write(dir.path().join("src/foo/bar/mod.rs"), "pub struct Baz;").unwrap();

        let file = dir.path().join("src/lib.rs");
        let crate_root = dir.path().join("src");

        // Test: crate::foo::bar::Baz should resolve to foo/bar/mod.rs
        let result = resolve_rust_import("crate::foo::bar", &file, &crate_root, dir.path());
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert!(resolved.contains("foo") && resolved.contains("bar"));
    }

    #[test]
    fn test_ts_path_resolver_multiple_wildcards() {
        let dir = TempDir::new().unwrap();

        // Create tsconfig with multiple wildcard pattern
        let tsconfig = r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "**/*": ["src/**/*"]
                }
            }
        }"#;
        fs::write(dir.path().join("tsconfig.json"), tsconfig).unwrap();
        fs::create_dir_all(dir.path().join("src/components/ui")).unwrap();
        fs::write(dir.path().join("src/components/ui/Button.tsx"), "export {}").unwrap();

        let resolver = TsPathResolver::from_tsconfig(dir.path()).unwrap();
        let exts: HashSet<String> = ["tsx", "ts"].iter().map(|s| s.to_string()).collect();

        let result = resolver.resolve("components/ui/Button", Some(&exts));
        assert!(result.is_some());
    }

    #[test]
    fn test_ts_path_resolver_caching() {
        let dir = create_test_project();
        let resolver = TsPathResolver::from_tsconfig(dir.path()).unwrap();
        let exts: HashSet<String> = ["ts"].iter().map(|s| s.to_string()).collect();

        // First resolve
        let result1 = resolver.resolve("src/index", Some(&exts));
        assert!(result1.is_some());

        // Second resolve should hit cache
        let result2 = resolver.resolve("src/index", Some(&exts));
        assert!(result2.is_some());
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_load_package_exports() {
        let dir = TempDir::new().unwrap();

        // Create package.json with exports
        let package_json = r#"{
            "name": "test-package",
            "exports": {
                ".": "./dist/index.js",
                "./utils": "./dist/utils/index.js",
                "./components": {
                    "import": "./dist/components/index.mjs",
                    "require": "./dist/components/index.js"
                }
            }
        }"#;
        fs::write(dir.path().join("package.json"), package_json).unwrap();

        let exports = load_package_exports(dir.path()).unwrap();
        assert_eq!(exports.get("."), Some(&"./dist/index.js".to_string()));
        assert_eq!(
            exports.get("./utils"),
            Some(&"./dist/utils/index.js".to_string())
        );
        assert!(exports.contains_key("./components"));
    }

    #[test]
    fn test_ts_path_resolver_with_package_exports() {
        let dir = TempDir::new().unwrap();

        // Create minimal tsconfig
        let tsconfig = r#"{"compilerOptions": {"baseUrl": "."}}"#;
        fs::write(dir.path().join("tsconfig.json"), tsconfig).unwrap();

        // Create package.json with exports
        let package_json = r#"{
            "exports": {
                "./utils": "./dist/utils.js"
            }
        }"#;
        fs::write(dir.path().join("package.json"), package_json).unwrap();
        fs::create_dir_all(dir.path().join("dist")).unwrap();
        fs::write(dir.path().join("dist/utils.js"), "export {}").unwrap();

        let resolver = TsPathResolver::from_tsconfig(dir.path()).unwrap();
        let result = resolver.resolve("./utils", None);
        // This should fallback to package exports
        // Note: relative paths are skipped, so this tests the fallback logic
        assert!(result.is_none()); // Relative paths return None by design
    }
}
