use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::types::{
    ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ImportResolutionKind, ReexportEntry,
    ReexportKind,
};

use super::regexes::{
    regex_py_all, regex_py_class, regex_py_def, regex_py_dynamic_dunder, regex_py_dynamic_importlib,
};
use super::resolvers::{resolve_python_absolute, resolve_python_relative};

pub(crate) fn python_stdlib_set() -> &'static HashSet<String> {
    static STDLIB: OnceLock<HashSet<String>> = OnceLock::new();
    STDLIB.get_or_init(|| {
        [
            "abc",
            "argparse",
            "array",
            "asyncio",
            "base64",
            "binascii",
            "bisect",
            "cmath",
            "collections",
            "concurrent",
            "contextlib",
            "copy",
            "crypt",
            "csv",
            "ctypes",
            "dataclasses",
            "datetime",
            "decimal",
            "difflib",
            "email",
            "errno",
            "functools",
            "gc",
            "getpass",
            "glob",
            "hashlib",
            "heapq",
            "html",
            "http",
            "importlib",
            "inspect",
            "io",
            "ipaddress",
            "itertools",
            "json",
            "logging",
            "lzma",
            "math",
            "multiprocessing",
            "numbers",
            "operator",
            "os",
            "pathlib",
            "pickle",
            "platform",
            "plistlib",
            "queue",
            "random",
            "re",
            "sched",
            "secrets",
            "select",
            "shlex",
            "shutil",
            "signal",
            "socket",
            "sqlite3",
            "ssl",
            "statistics",
            "string",
            "struct",
            "subprocess",
            "sys",
            "tempfile",
            "textwrap",
            "threading",
            "time",
            "timeit",
            "tkinter",
            "traceback",
            "types",
            "typing",
            "typing_extensions",
            "unicodedata",
            "urllib",
            "uuid",
            "xml",
            "xmlrpc",
            "zipfile",
            "zlib",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    })
}

fn parse_all_list(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    for caps in regex_py_all().captures_iter(content) {
        let body = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        for item in body.split(',') {
            let trimmed = item.trim();
            let name = trimmed
                .trim_matches(|c| c == '\'' || c == '"')
                .trim()
                .to_string();
            if !name.is_empty() {
                names.push(name);
            }
        }
    }
    names
}

fn read_all_from_resolved(resolved: &Option<String>, root: &Path) -> Option<Vec<String>> {
    let path_str = resolved.as_ref()?;
    let candidate = {
        let p = PathBuf::from(path_str);
        if p.is_absolute() { p } else { root.join(p) }
    };
    let content = std::fs::read_to_string(&candidate).ok()?;
    let names = parse_all_list(&content);
    if names.is_empty() { None } else { Some(names) }
}

fn resolve_python_import(
    module: &str,
    file_path: &Path,
    root: &Path,
    py_roots: &[PathBuf],
    extensions: Option<&HashSet<String>>,
    stdlib: &HashSet<String>,
) -> (Option<String>, ImportResolutionKind) {
    if module.starts_with('.') {
        let resolved = resolve_python_relative(module, file_path, root, extensions);
        let kind = if resolved.is_some() {
            ImportResolutionKind::Local
        } else {
            ImportResolutionKind::Unknown
        };
        return (resolved, kind);
    }

    if let Some(resolved) = resolve_python_absolute(module, py_roots, root, extensions) {
        return (Some(resolved), ImportResolutionKind::Local);
    }

    let head = module.split('.').next().unwrap_or(module).to_lowercase();
    if stdlib.contains(&head) {
        return (None, ImportResolutionKind::Stdlib);
    }

    (None, ImportResolutionKind::Unknown)
}

pub(crate) fn analyze_py_file(
    content: &str,
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
    relative: String,
    py_roots: &[PathBuf],
    stdlib: &HashSet<String>,
) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative);
    let mut type_check_stack: Vec<usize> = Vec::new();

    for line in content.lines() {
        let without_comment = line.split('#').next().unwrap_or("").trim_end();
        let indent = without_comment
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();
        if !without_comment.trim().is_empty() {
            while let Some(level) = type_check_stack.last() {
                if indent < *level {
                    type_check_stack.pop();
                } else {
                    break;
                }
            }
        }

        let trimmed = without_comment.trim_start();
        if let Some(body) = trimmed
            .strip_prefix("if ")
            .and_then(|rest| rest.strip_suffix(':'))
        {
            if body.contains("TYPE_CHECKING") {
                type_check_stack.push(indent + 1);
            }
            continue;
        }

        let in_type_checking = !type_check_stack.is_empty();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            for part in rest.split(',') {
                let mut name = part.trim();
                if let Some((lhs, _)) = name.split_once(" as ") {
                    name = lhs.trim();
                }
                if !name.is_empty() {
                    let mut entry = ImportEntry::new(name.to_string(), ImportKind::Static);
                    let (resolved, resolution) =
                        resolve_python_import(name, path, root, py_roots, extensions, stdlib);
                    entry.resolution = resolution;
                    entry.resolved_path = resolved;
                    entry.is_type_checking = in_type_checking;
                    analysis.imports.push(entry);
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("from ")
            && let Some((module, names_raw)) = rest.split_once(" import ")
        {
            let module = module.trim().trim_end_matches('.');
            let names_clean = names_raw.trim().trim_matches('(').trim_matches(')');
            let names_clean = names_clean.split('#').next().unwrap_or("").trim();
            if !module.is_empty() {
                let mut entry = ImportEntry::new(module.to_string(), ImportKind::Static);
                let (resolved, resolution) =
                    resolve_python_import(module, path, root, py_roots, extensions, stdlib);
                entry.resolution = resolution;
                entry.resolved_path = resolved.clone();
                entry.is_type_checking = in_type_checking;
                analysis.imports.push(entry);
            }
            if names_clean == "*" {
                let (resolved, _) =
                    resolve_python_import(module, path, root, py_roots, extensions, stdlib);
                let mut entry = ReexportEntry {
                    source: module.to_string(),
                    kind: ReexportKind::Star,
                    resolved: resolved.clone(),
                };
                if let Some(names) = read_all_from_resolved(&resolved, root) {
                    for name in &names {
                        analysis.exports.push(ExportSymbol::new(
                            name.clone(),
                            "reexport",
                            "named",
                            None,
                        ));
                    }
                    entry.kind = ReexportKind::Named(names);
                }
                analysis.reexports.push(entry);
            }
        }
    }

    for caps in regex_py_dynamic_importlib().captures_iter(content) {
        if let Some(m) = caps.get(1) {
            analysis.dynamic_imports.push(m.as_str().trim().to_string());
        }
    }
    for caps in regex_py_dynamic_dunder().captures_iter(content) {
        if let Some(m) = caps.get(1) {
            analysis.dynamic_imports.push(m.as_str().trim().to_string());
        }
    }

    for name in parse_all_list(content) {
        analysis
            .exports
            .push(ExportSymbol::new(name, "__all__", "named", None));
    }

    for caps in regex_py_def().captures_iter(content) {
        if let Some(name) = caps.get(1) {
            let n = name.as_str();
            if !n.starts_with('_') {
                analysis
                    .exports
                    .push(ExportSymbol::new(n.to_string(), "def", "named", None));
            }
        }
    }
    for caps in regex_py_class().captures_iter(content) {
        if let Some(name) = caps.get(1) {
            let n = name.as_str();
            if !n.starts_with('_') {
                analysis
                    .exports
                    .push(ExportSymbol::new(n.to_string(), "class", "named", None));
            }
        }
    }

    // Detect Python entry points
    // 1. __main__.py files are package entry points
    if analysis.path.ends_with("__main__.py") {
        analysis.entry_points.push("__main__".to_string());
    }
    // 2. if __name__ == "__main__": is a script entry point
    if content.contains("if __name__")
        && (content.contains("__main__") || content.contains("'__main__'"))
        && !analysis.entry_points.contains(&"script".to_string())
    {
        analysis.entry_points.push("script".to_string());
    }

    analysis
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn py_exts() -> HashSet<String> {
        ["py"].iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn marks_type_checking_imports_and_stdlib() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("foo.py"), "VALUE = 1").expect("write foo.py");
        let content = r#"
from typing import TYPE_CHECKING
if TYPE_CHECKING:
    import foo

import sys
"#;
        let path = root.join("main.py");
        let analysis = analyze_py_file(
            content,
            &path,
            root,
            Some(&py_exts()),
            "main.py".to_string(),
            &[root.to_path_buf()],
            python_stdlib_set(),
        );
        assert!(analysis.imports.len() >= 2);
        let foo = analysis
            .imports
            .iter()
            .find(|i| i.source == "foo")
            .expect("foo import");
        assert!(foo.is_type_checking);
        assert_eq!(foo.resolution, ImportResolutionKind::Local);
        assert!(foo.resolved_path.as_deref().unwrap().contains("foo.py"));

        let sys = analysis
            .imports
            .iter()
            .find(|i| i.source == "sys")
            .expect("sys import");
        assert!(!sys.is_type_checking);
        assert_eq!(sys.resolution, ImportResolutionKind::Stdlib);
        assert!(sys.resolved_path.is_none());
    }

    #[test]
    fn expands_all_for_star_import() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("pkg")).expect("mkdir pkg");
        std::fs::write(root.join("pkg/__init__.py"), "__all__ = ['Foo', 'Bar']")
            .expect("write __init__");
        let content = "from pkg import *";
        let path = root.join("main.py");
        let analysis = analyze_py_file(
            content,
            &path,
            root,
            Some(&py_exts()),
            "main.py".to_string(),
            &[root.to_path_buf()],
            python_stdlib_set(),
        );
        let reexports = analysis
            .reexports
            .iter()
            .find(|r| r.source == "pkg")
            .expect("pkg reexport");
        match &reexports.kind {
            ReexportKind::Named(names) => {
                assert_eq!(names.len(), 2);
                assert!(names.contains(&"Foo".to_string()));
                assert!(names.contains(&"Bar".to_string()));
            }
            other => panic!("expected named reexport, got {:?}", other),
        }
        let exported: HashSet<_> = analysis.exports.iter().map(|e| e.name.clone()).collect();
        assert!(exported.contains("Foo"));
        assert!(exported.contains("Bar"));
    }

    #[test]
    fn dynamic_imports_and_local_over_stdlib() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("json.py"), "LOCAL = True").expect("write json.py");
        let content = r#"
import json
mod = importlib.import_module(f"pkg.{name}")
dyn = __import__("x.y")
"#;
        let path = root.join("main.py");
        let analysis = analyze_py_file(
            content,
            &path,
            root,
            Some(&py_exts()),
            "main.py".to_string(),
            &[root.to_path_buf()],
            python_stdlib_set(),
        );
        let json_imp = analysis
            .imports
            .iter()
            .find(|i| i.source == "json")
            .expect("json import");
        assert_eq!(json_imp.resolution, ImportResolutionKind::Local);
        assert!(
            json_imp
                .resolved_path
                .as_deref()
                .unwrap_or("")
                .ends_with("json.py")
        );

        assert_eq!(analysis.dynamic_imports.len(), 2);
        assert!(analysis.dynamic_imports.iter().any(|s| s.contains("pkg.")));
        assert!(analysis.dynamic_imports.iter().any(|s| s.contains("x.y")));
    }
}
