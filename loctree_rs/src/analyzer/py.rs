use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::types::{
    ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ImportResolutionKind, ImportSymbol,
    PyRaceIndicator, ReexportEntry, ReexportKind,
};

use super::regexes::{regex_py_all, regex_py_dynamic_dunder, regex_py_dynamic_importlib};
use super::resolvers::{has_py_typed_marker, resolve_python_absolute, resolve_python_relative};

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

/// Detect Python concurrency patterns that may indicate race conditions
fn detect_py_race_indicators(content: &str) -> Vec<PyRaceIndicator> {
    let mut indicators = Vec::new();
    let mut has_threading_import = false;
    let mut has_lock_usage = false;
    let mut has_asyncio_import = false;
    let mut has_multiprocessing_import = false;
    let mut thread_creations: Vec<usize> = Vec::new();
    let mut asyncio_parallel: Vec<(usize, &str)> = Vec::new();
    let mut mp_pool_usage: Vec<usize> = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_1based = line_num + 1;
        let trimmed = line.trim();

        // Track imports
        if trimmed.contains("import threading") || trimmed.contains("from threading") {
            has_threading_import = true;
        }
        if trimmed.contains("import asyncio") || trimmed.contains("from asyncio") {
            has_asyncio_import = true;
        }
        if trimmed.contains("import multiprocessing") || trimmed.contains("from multiprocessing") {
            has_multiprocessing_import = true;
        }

        // Track Lock usage
        if trimmed.contains("Lock(") || trimmed.contains("RLock(") || trimmed.contains("Semaphore(")
        {
            has_lock_usage = true;
        }

        // Track Thread creation
        if trimmed.contains("Thread(")
            && (has_threading_import || trimmed.contains("threading.Thread"))
        {
            thread_creations.push(line_1based);
        }

        // Track asyncio parallel patterns
        if trimmed.contains("asyncio.gather(") || trimmed.contains("gather(") && has_asyncio_import
        {
            asyncio_parallel.push((line_1based, "gather"));
        }
        if trimmed.contains("asyncio.create_task(")
            || trimmed.contains("create_task(") && has_asyncio_import
        {
            asyncio_parallel.push((line_1based, "create_task"));
        }
        if trimmed.contains("asyncio.wait(") || trimmed.contains(".wait(") && has_asyncio_import {
            asyncio_parallel.push((line_1based, "wait"));
        }

        // Track concurrent.futures import
        if trimmed.contains("concurrent.futures") || trimmed.contains("from concurrent") {
            has_multiprocessing_import = true; // Treat as multiprocessing-like
        }

        // Track multiprocessing Pool
        if (trimmed.contains("Pool(")
            || trimmed.contains("ProcessPoolExecutor(")
            || trimmed.contains("ThreadPoolExecutor("))
            && (has_multiprocessing_import
                || trimmed.contains("multiprocessing.")
                || trimmed.contains("concurrent.futures"))
        {
            mp_pool_usage.push(line_1based);
        }
    }

    // Generate warnings based on patterns

    // Threading without Lock
    if !thread_creations.is_empty() && !has_lock_usage {
        for line in thread_creations {
            indicators.push(PyRaceIndicator {
                line,
                concurrency_type: "threading".to_string(),
                pattern: "Thread".to_string(),
                risk: "warning".to_string(),
                message: "Thread created without Lock/RLock/Semaphore - potential race condition"
                    .to_string(),
            });
        }
    }

    // Asyncio parallel execution (info level - needs manual review)
    for (line, pattern) in asyncio_parallel {
        indicators.push(PyRaceIndicator {
            line,
            concurrency_type: "asyncio".to_string(),
            pattern: pattern.to_string(),
            risk: "info".to_string(),
            message: format!(
                "Parallel async execution with {} - verify shared state access",
                pattern
            ),
        });
    }

    // Multiprocessing pool (info level)
    for line in mp_pool_usage {
        indicators.push(PyRaceIndicator {
            line,
            concurrency_type: "multiprocessing".to_string(),
            pattern: "Pool".to_string(),
            risk: "info".to_string(),
            message: "Process/Thread pool - ensure shared resources are process-safe".to_string(),
        });
    }

    indicators
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

/// Detect if a Python file is a test file based on path and content patterns
fn is_python_test_file(path: &Path, content: &str) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    // Path-based detection
    if path_str.contains("/tests/")
        || path_str.contains("/test/")
        || path_str.contains("/__tests__/")
        || path_str.ends_with("_test.py")
        || path_str.ends_with("_tests.py")
        || path_str.ends_with("test_.py")
        || path_str.contains("/test_")
        || path_str.contains("conftest.py")
        || path_str.contains("pytest_")
    {
        return true;
    }

    // Content-based detection: pytest imports or unittest usage
    if content.contains("import pytest")
        || content.contains("from pytest")
        || content.contains("import unittest")
        || content.contains("from unittest")
        || content.contains("@pytest.fixture")
        || content.contains("@pytest.mark")
        || content.contains("class Test")
        || content.contains("def test_")
    {
        return true;
    }

    false
}

/// Check if the file is part of a typed package (has py.typed marker upstream)
fn check_typed_package(path: &Path, root: &Path) -> bool {
    let mut current = path.parent();
    while let Some(dir) = current {
        if has_py_typed_marker(dir) {
            return true;
        }
        // Stop at root or if we've gone above root
        if dir == root || !dir.starts_with(root) {
            break;
        }
        current = dir.parent();
    }
    false
}

/// Check if the file is part of a namespace package (no __init__.py upstream before root)
fn check_namespace_package(path: &Path, root: &Path) -> bool {
    let mut current = path.parent();
    while let Some(dir) = current {
        // If there's an __init__.py, it's a traditional package
        if dir.join("__init__.py").exists() || dir.join("__init__.pyi").exists() {
            return false;
        }
        // If we reach root without finding __init__.py, check if it's a valid namespace
        if dir == root {
            break;
        }
        current = dir.parent();
    }
    // True if we have .py files but no __init__.py found in hierarchy
    path.parent().is_some_and(|p| {
        p.read_dir().ok().is_some_and(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "py" || ext == "pyi")
            })
        })
    })
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
    let mut pending_callback_decorator = false;

    // Set Python-specific metadata
    analysis.is_test = is_python_test_file(path, content);
    analysis.is_typed_package = check_typed_package(path, root);
    analysis.is_namespace_package = check_namespace_package(path, root);

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
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
        if trimmed.starts_with('@') {
            // Track decorators that register callbacks (e.g., @rumps.clicked)
            if trimmed.contains("clicked") || trimmed.contains("rumps.") {
                pending_callback_decorator = true;
            }
            continue;
        }

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
                entry.source_raw = format!("from {} import {}", module, names_clean);

                if names_clean != "*" {
                    for sym in names_clean.split(',') {
                        let sym = sym.trim();
                        if sym.is_empty() {
                            continue;
                        }
                        let (name, alias) = if let Some((lhs, rhs)) = sym.split_once(" as ") {
                            (lhs.trim(), Some(rhs.trim().to_string()))
                        } else {
                            (sym, None)
                        };
                        entry.symbols.push(ImportSymbol {
                            name: name.to_string(),
                            alias,
                            is_default: false,
                        });
                    }
                }
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
        } else {
            // Detect callback assignment patterns (callback=self.refresh or callback=refresh)
            if let Some(pos) = trimmed.find("callback")
                && let Some(eq_pos) = trimmed[pos..].find('=')
            {
                let after_eq = trimmed[pos + eq_pos + 1..].trim();
                let target = after_eq
                    .trim_start_matches("self.")
                    .trim_start_matches("cls.")
                    .trim_start_matches('&')
                    .trim_start_matches('*');
                let ident = target
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("")
                    .trim();
                if !ident.is_empty() {
                    analysis.local_uses.push(ident.to_string());
                }
            }

            // Track class bases and top-level exports
            if let Some(rest) = trimmed.strip_prefix("class ") {
                let (name_part, _) = rest.split_once(':').unwrap_or((rest, ""));
                let (name, bases_part) = if let Some((n, bases)) = name_part.split_once('(') {
                    (n.trim(), Some(bases.trim_end_matches(')').trim()))
                } else {
                    (name_part.trim(), None)
                };

                if indent == 0 && !name.starts_with('_') && !name.is_empty() {
                    analysis.exports.push(ExportSymbol::new(
                        name.to_string(),
                        "class",
                        "named",
                        Some(line_num),
                    ));
                }

                if let Some(bases) = bases_part {
                    for base in bases.split(',') {
                        let base = base
                            .trim_start_matches("self.")
                            .trim_start_matches("cls.")
                            .trim();
                        if !base.is_empty() {
                            analysis.local_uses.push(base.to_string());
                        }
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("def ") {
                let name = rest
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches(':');
                if indent == 0 && !name.starts_with('_') && !name.is_empty() {
                    analysis.exports.push(ExportSymbol::new(
                        name.to_string(),
                        "def",
                        "named",
                        Some(line_num),
                    ));
                }

                if pending_callback_decorator && !name.is_empty() {
                    analysis.local_uses.push(name.to_string());
                }
                pending_callback_decorator = false;
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
        // Also mark 'main' as locally used if it's called in the __main__ block
        if content.contains("main()") && !analysis.local_uses.contains(&"main".to_string()) {
            analysis.local_uses.push("main".to_string());
        }
    }

    // Detect bare function calls in Python (similar to Rust detection)
    // This catches local function calls like `helper_func(...)` within the same file
    extract_python_function_calls(content, &mut analysis.local_uses);

    // Detect Python concurrency race indicators
    analysis.py_race_indicators = detect_py_race_indicators(content);

    analysis
}

/// Extract function calls from Python code to detect local usage.
/// This catches patterns like `func_name(...)` which indicate the function is used.
fn extract_python_function_calls(content: &str, local_uses: &mut Vec<String>) {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Python keywords that look like function calls but aren't
    const KEYWORDS: &[&str] = &[
        "if", "else", "elif", "while", "for", "try", "except", "finally", "with", "as", "def",
        "class", "return", "yield", "raise", "import", "from", "pass", "break", "continue",
        "lambda", "and", "or", "not", "in", "is", "True", "False", "None", "assert", "del", "exec",
        "print", "global", "nonlocal", "async", "await",
    ];

    while i < len {
        // Look for identifier followed by `(`
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &content[start..i];

            // Skip whitespace
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }

            // Check if followed by `(`
            if i < len
                && bytes[i] == b'('
                && !KEYWORDS.contains(&ident)
                && !local_uses.contains(&ident.to_string())
            {
                local_uses.push(ident.to_string());
            }
        } else {
            i += 1;
        }
    }
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
    fn tracks_from_import_symbols_and_aliases() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("utils")).expect("mkdir utils");
        std::fs::write(
            root.join("utils/helpers.py"),
            "class Foo: pass\nclass Baz: pass",
        )
        .expect("write helpers");
        let content = "from utils.helpers import Foo as Bar, Baz";
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
        let imp = analysis.imports.first().expect("import entry");
        assert_eq!(imp.symbols.len(), 2);
        assert_eq!(imp.symbols[0].name, "Foo");
        assert_eq!(imp.symbols[0].alias.as_deref(), Some("Bar"));
        assert_eq!(imp.symbols[1].name, "Baz");
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

    #[test]
    fn detects_threading_without_lock() {
        let content = r#"
import threading

def worker():
    pass

t = threading.Thread(target=worker)
t.start()
"#;
        let indicators = detect_py_race_indicators(content);
        assert_eq!(indicators.len(), 1);
        assert_eq!(indicators[0].concurrency_type, "threading");
        assert_eq!(indicators[0].risk, "warning");
    }

    #[test]
    fn no_warning_with_lock() {
        let content = r#"
import threading

lock = threading.Lock()

def worker():
    with lock:
        pass

t = threading.Thread(target=worker)
t.start()
"#;
        let indicators = detect_py_race_indicators(content);
        // Should not have threading warning because Lock is used
        let threading_warnings = indicators
            .iter()
            .filter(|i| i.concurrency_type == "threading")
            .count();
        assert_eq!(threading_warnings, 0);
    }

    #[test]
    fn detects_asyncio_gather() {
        let content = r#"
import asyncio

async def main():
    await asyncio.gather(task1(), task2())
"#;
        let indicators = detect_py_race_indicators(content);
        let asyncio_indicators: Vec<_> = indicators
            .iter()
            .filter(|i| i.concurrency_type == "asyncio")
            .collect();
        assert!(!asyncio_indicators.is_empty());
        assert_eq!(asyncio_indicators[0].pattern, "gather");
    }

    #[test]
    fn detects_asyncio_create_task() {
        let content = r#"
import asyncio

async def main():
    task = asyncio.create_task(worker())
"#;
        let indicators = detect_py_race_indicators(content);
        let asyncio_indicators: Vec<_> = indicators
            .iter()
            .filter(|i| i.concurrency_type == "asyncio")
            .collect();
        assert!(!asyncio_indicators.is_empty());
        assert!(
            asyncio_indicators
                .iter()
                .any(|i| i.pattern == "create_task")
        );
    }

    #[test]
    fn detects_multiprocessing_pool() {
        let content = r#"
import multiprocessing

def main():
    with multiprocessing.Pool(4) as pool:
        results = pool.map(worker, data)
"#;
        let indicators = detect_py_race_indicators(content);
        let mp_indicators: Vec<_> = indicators
            .iter()
            .filter(|i| i.concurrency_type == "multiprocessing")
            .collect();
        assert!(!mp_indicators.is_empty());
    }

    #[test]
    fn detects_concurrent_futures_pool() {
        let content = r#"
from concurrent.futures import ThreadPoolExecutor

with ThreadPoolExecutor(max_workers=4) as executor:
    results = executor.map(worker, data)
"#;
        let indicators = detect_py_race_indicators(content);
        let pool_indicators: Vec<_> = indicators.iter().filter(|i| i.pattern == "Pool").collect();
        assert!(!pool_indicators.is_empty());
    }

    #[test]
    fn no_indicators_for_clean_code() {
        let content = r#"
def add(a, b):
    return a + b

result = add(1, 2)
print(result)
"#;
        let indicators = detect_py_race_indicators(content);
        assert!(indicators.is_empty());
    }

    #[test]
    fn detects_asyncio_wait() {
        let content = r#"
import asyncio

async def main():
    done, pending = await asyncio.wait(tasks)
"#;
        let indicators = detect_py_race_indicators(content);
        let asyncio_indicators: Vec<_> = indicators
            .iter()
            .filter(|i| i.concurrency_type == "asyncio")
            .collect();
        assert!(!asyncio_indicators.is_empty());
        assert!(asyncio_indicators.iter().any(|i| i.pattern == "wait"));
    }

    #[test]
    fn parses_all_list_exports() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
__all__ = ["foo", "bar"]

def foo():
    pass

def bar():
    pass

def _private():
    pass
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("module.py"),
            root,
            Some(&py_exts()),
            "module.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        // Should have exports from __all__ and public defs
        let export_names: Vec<_> = analysis.exports.iter().map(|e| e.name.as_str()).collect();
        assert!(export_names.contains(&"foo"));
        assert!(export_names.contains(&"bar"));
        // Private _private should not be in exports
        assert!(!export_names.contains(&"_private"));
    }

    #[test]
    fn parses_class_exports() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
class MyClass:
    pass

class _PrivateClass:
    pass
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("classes.py"),
            root,
            Some(&py_exts()),
            "classes.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        let class_exports: Vec<_> = analysis
            .exports
            .iter()
            .filter(|e| e.kind == "class")
            .collect();
        assert!(class_exports.iter().any(|e| e.name == "MyClass"));
        // Private class should not be exported
        assert!(!class_exports.iter().any(|e| e.name == "_PrivateClass"));
    }

    #[test]
    fn detects_main_entry_point() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
def main():
    print("Hello")

if __name__ == "__main__":
    main()
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("__main__.py"),
            root,
            Some(&py_exts()),
            "__main__.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(analysis.entry_points.contains(&"__main__".to_string()));
    }

    #[test]
    fn detects_script_entry_point() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
def main():
    print("Hello")

if __name__ == "__main__":
    main()
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("script.py"),
            root,
            Some(&py_exts()),
            "script.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(analysis.entry_points.contains(&"script".to_string()));
    }

    #[test]
    fn detects_test_file_by_path() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("tests")).expect("mkdir");
        std::fs::write(root.join("tests/test_utils.py"), "def test_foo(): pass")
            .expect("write test file");

        let content = "def test_foo(): pass";
        let analysis = analyze_py_file(
            content,
            &root.join("tests/test_utils.py"),
            root,
            Some(&py_exts()),
            "tests/test_utils.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(analysis.is_test);
    }

    #[test]
    fn detects_test_file_by_content() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
import pytest

@pytest.fixture
def sample_fixture():
    return 42

def test_something(sample_fixture):
    assert sample_fixture == 42
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("my_tests.py"),
            root,
            Some(&py_exts()),
            "my_tests.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(analysis.is_test);
    }

    #[test]
    fn detects_typed_package() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("mypackage")).expect("mkdir");
        std::fs::write(root.join("mypackage/__init__.py"), "").expect("write __init__");
        std::fs::write(root.join("mypackage/py.typed"), "").expect("write py.typed");
        std::fs::write(root.join("mypackage/utils.py"), "def foo(): pass").expect("write utils");

        let content = "def foo(): pass";
        let analysis = analyze_py_file(
            content,
            &root.join("mypackage/utils.py"),
            root,
            Some(&py_exts()),
            "mypackage/utils.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(analysis.is_typed_package);
    }

    #[test]
    fn detects_non_typed_package() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("mypackage")).expect("mkdir");
        std::fs::write(root.join("mypackage/__init__.py"), "").expect("write __init__");
        // No py.typed marker
        std::fs::write(root.join("mypackage/utils.py"), "def foo(): pass").expect("write utils");

        let content = "def foo(): pass";
        let analysis = analyze_py_file(
            content,
            &root.join("mypackage/utils.py"),
            root,
            Some(&py_exts()),
            "mypackage/utils.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(!analysis.is_typed_package);
    }

    #[test]
    fn detects_namespace_package() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        // Create namespace package (no __init__.py)
        std::fs::create_dir_all(root.join("namespace_pkg")).expect("mkdir");
        std::fs::write(root.join("namespace_pkg/module.py"), "VALUE = 1").expect("write module");

        let content = "VALUE = 1";
        let analysis = analyze_py_file(
            content,
            &root.join("namespace_pkg/module.py"),
            root,
            Some(&py_exts()),
            "namespace_pkg/module.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(analysis.is_namespace_package);
    }

    #[test]
    fn traditional_package_not_namespace() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("pkg")).expect("mkdir");
        std::fs::write(root.join("pkg/__init__.py"), "").expect("write __init__");
        std::fs::write(root.join("pkg/module.py"), "VALUE = 1").expect("write module");

        let content = "VALUE = 1";
        let analysis = analyze_py_file(
            content,
            &root.join("pkg/module.py"),
            root,
            Some(&py_exts()),
            "pkg/module.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(!analysis.is_namespace_package);
    }

    #[test]
    fn top_level_exports_have_lines_and_methods_not_exported() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let content = "\
class Base:\n    pass\n\nclass Child(Base):\n    def method(self):\n        pass\n\ndef top():\n    return True\n\nmenu = MenuItem(callback=top)\n";
        let analysis = analyze_py_file(
            content,
            &root.join("app.py"),
            root,
            Some(&py_exts()),
            "app.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        let names: Vec<_> = analysis.exports.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Base"));
        assert!(names.contains(&"Child"));
        assert!(names.contains(&"top"));
        assert!(!names.contains(&"method"));

        let top_line = analysis
            .exports
            .iter()
            .find(|e| e.name == "top")
            .and_then(|e| e.line)
            .unwrap();
        assert_eq!(top_line, 8);

        assert!(analysis.local_uses.contains(&"top".to_string()));
        assert!(analysis.local_uses.contains(&"Base".to_string()));
    }
}
