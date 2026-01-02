//! Python file analysis module.
//!
//! Provides comprehensive Python code analysis including:
//! - Import/export detection (static and dynamic)
//! - Type hint usage extraction
//! - Framework decorator detection (pytest, FastAPI, Flask, Django, etc.)
//! - Concurrency pattern detection for race conditions
//! - Dynamic code generation detection (exec/eval/compile)
//! - Package metadata (typed packages, namespace packages)
//!
//! Created by M&K (c)2025 The LibraxisAI Team
//! Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>

mod concurrency;
mod decorators;
mod dynamic;
mod exports;
mod helpers;
mod imports;
mod metadata;
mod stdlib;
mod usages;

// Re-export the public API
pub(crate) use stdlib::python_stdlib_set;

// Private imports from submodules
use concurrency::detect_py_race_indicators;
use decorators::{extract_decorator_type_usages, is_framework_decorator, parse_route_decorator};
use dynamic::detect_dynamic_exec_templates;
use exports::{parse_all_list, read_all_from_resolved};
use imports::resolve_python_import;
use metadata::{check_namespace_package, check_typed_package, is_python_test_file};
use usages::{
    extract_bare_class_references, extract_class_from_containers, extract_python_function_calls,
    extract_type_hint_usages,
};

// External imports
use super::regexes::{regex_py_dynamic_dunder, regex_py_dynamic_importlib};
use crate::types::{
    ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ImportSymbol, ReexportEntry, ReexportKind,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Main entry point for Python file analysis.
///
/// Analyzes a Python file and extracts:
/// - Import statements (static and dynamic)
/// - Export symbols (functions, classes, __all__ list)
/// - Re-exports from __init__.py files
/// - Type hint usages
/// - Framework decorators
/// - Concurrency patterns
/// - Entry points
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
    let mut pending_framework_decorator = false;
    let mut pending_fixture_decorator = false;
    let mut pending_routes: Vec<crate::types::RouteInfo> = Vec::new();
    let mut pending_fixture_name: Option<String> = None;
    let mut in_docstring = false;
    let is_package_init = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == "__init__.py");

    // Set Python-specific metadata
    analysis.is_test = is_python_test_file(path, content);
    analysis.is_typed_package = check_typed_package(path, root);
    analysis.is_namespace_package = check_namespace_package(path, root);

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed_leading = line.trim_start();

        if in_docstring {
            // End docstring on closing triple quotes
            if trimmed_leading.contains("\"\"\"") || trimmed_leading.contains("'''") {
                in_docstring = false;
            }
            continue;
        }

        // Skip docstring/comment blocks at the start of a line
        if trimmed_leading.starts_with("\"\"\"") || trimmed_leading.starts_with("'''") {
            // If closing appears on the same line, exit docstring immediately
            let mut occurrences = 0;
            for token in ["\"\"\"", "'''"] {
                occurrences += trimmed_leading.matches(token).count();
            }
            if occurrences < 2 {
                in_docstring = true;
            }
            continue;
        }

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
            // Track framework decorators that mark functions as "used"
            if is_framework_decorator(trimmed) {
                pending_framework_decorator = true;
            }
            if let Some(route) = parse_route_decorator(trimmed, line_num) {
                pending_routes.push(route);
            }
            // pytest fixtures: treat next def as used
            if trimmed.contains("pytest.fixture") {
                pending_fixture_decorator = true;
                pending_fixture_name = None;
            }
            // Extract type usages from decorator parameters (response_model=X, Depends(X))
            extract_decorator_type_usages(trimmed, &mut analysis.local_uses);
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
                    entry.is_lazy = indent > 0;
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
                entry.is_lazy = indent > 0;
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

                // Python package API re-export pattern:
                // __init__.py often re-exports names via `from .mod import Foo as Bar`.
                // Treat these as re-exports (not fresh definitions) to reduce duplicate/dead noise.
                if is_package_init && indent == 0 && names_clean != "*" {
                    let mut name_pairs: Vec<(String, String)> = Vec::new();
                    for sym in names_clean.split(',') {
                        let sym = sym.trim();
                        if sym.is_empty() {
                            continue;
                        }
                        let (original, exported) = if let Some((lhs, rhs)) = sym.split_once(" as ")
                        {
                            (lhs.trim(), rhs.trim())
                        } else {
                            (sym, sym)
                        };
                        if exported.is_empty() || exported.starts_with('_') {
                            continue;
                        }
                        name_pairs.push((original.to_string(), exported.to_string()));
                        analysis.exports.push(ExportSymbol::new(
                            exported.to_string(),
                            "reexport",
                            "named",
                            Some(line_num),
                        ));
                    }

                    if !name_pairs.is_empty() {
                        analysis.reexports.push(ReexportEntry {
                            source: module.to_string(),
                            kind: ReexportKind::Named(name_pairs),
                            resolved: resolved.clone(),
                        });
                    }
                }
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
                    // Star imports have no aliases - original and exported are the same
                    let name_pairs: Vec<(String, String)> =
                        names.into_iter().map(|n| (n.clone(), n)).collect();
                    entry.kind = ReexportKind::Named(name_pairs);
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
                            // Extract the last component for dotted names (e.g., wagtail.models.Page -> Page)
                            // But also keep the full dotted name in case it's a relative import
                            let simple_name = base.rsplit('.').next().unwrap_or(base);
                            if simple_name != base {
                                // If it's a dotted name, add both the full name and the simple name
                                analysis.local_uses.push(base.to_string());
                            }
                            if !simple_name.is_empty() {
                                analysis.local_uses.push(simple_name.to_string());
                            }
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

                // Mark function as used if decorated with callback/framework decorator
                if (pending_callback_decorator || pending_framework_decorator) && !name.is_empty() {
                    analysis.local_uses.push(name.to_string());
                }
                if pending_fixture_decorator && !name.is_empty() {
                    analysis.local_uses.push(name.to_string());
                    pending_fixture_name = Some(name.to_string());
                }
                if !name.is_empty() && !pending_routes.is_empty() {
                    for mut r in pending_routes.drain(..) {
                        if r.name.is_none() {
                            r.name = Some(name.to_string());
                        }
                        analysis.routes.push(r);
                    }
                } else {
                    pending_routes.clear();
                }
                pending_callback_decorator = false;
                pending_framework_decorator = false;
                pending_fixture_decorator = false;
                if let Some(fix) = pending_fixture_name.take() {
                    analysis.pytest_fixtures.push(fix);
                }
            } else if !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("class ")
            {
                // Reset decorator flags if we hit a non-decorator, non-def, non-class line
                pending_framework_decorator = false;
                pending_routes.clear();
                pending_fixture_name = None;
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

    // Detect type hint usages (dict[str, MyClass], defaultdict(MyClass), etc.)
    extract_type_hint_usages(content, &mut analysis.local_uses);

    // Detect class references in tuple/list/dict literals (issue #2)
    // This catches patterns like: (ClassName, 'value'), [Foo, Bar], {'key': Baz}
    extract_class_from_containers(content, &mut analysis.local_uses);

    // Detect bare class name usage in function arguments and returns (issue #3)
    // This catches: return ClassName, issubclass(x, ClassName), isinstance(obj, MyClass)
    extract_bare_class_references(content, &mut analysis.local_uses);

    // Detect exec/eval/compile dynamic code generation patterns
    // This catches template strings like "def get%s" that generate symbols dynamically
    analysis.dynamic_exec_templates = detect_dynamic_exec_templates(content);

    // Detect Python concurrency race indicators
    analysis.py_race_indicators = detect_py_race_indicators(content);

    analysis
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ImportResolutionKind;
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
    fn ignores_imports_inside_docstrings() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let content = "\"\"\"\nExample:\n    from app.middlewares.request_id import get_request_id\n\"\"\"\n\ndef real():\n    return 1\n";
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
        assert!(
            analysis.imports.is_empty(),
            "docstring-only import should be ignored"
        );
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
                let exported_names: Vec<_> = names.iter().map(|(_, e)| e.as_str()).collect();
                assert!(exported_names.contains(&"Foo"));
                assert!(exported_names.contains(&"Bar"));
            }
            other => panic!("expected named reexport, got {:?}", other),
        }
        let exported: HashSet<_> = analysis.exports.iter().map(|e| e.name.clone()).collect();
        assert!(exported.contains("Foo"));
        assert!(exported.contains("Bar"));
    }

    #[test]
    fn treats_init_named_from_import_as_reexport() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("pkg")).expect("mkdir pkg");
        std::fs::write(
            root.join("pkg/foo.py"),
            "class Foo: pass\nclass Baz: pass\n",
        )
        .expect("write foo.py");

        let content = "from .foo import Foo as Bar, Baz";
        let path = root.join("pkg/__init__.py");
        let analysis = analyze_py_file(
            content,
            &path,
            root,
            Some(&py_exts()),
            "pkg/__init__.py".to_string(),
            &[root.to_path_buf()],
            python_stdlib_set(),
        );

        let reexport = analysis
            .reexports
            .iter()
            .find(|r| r.source == ".foo")
            .expect("expected .foo reexport");

        match &reexport.kind {
            ReexportKind::Named(names) => {
                assert!(names.contains(&(String::from("Foo"), String::from("Bar"))));
                assert!(names.contains(&(String::from("Baz"), String::from("Baz"))));
            }
            other => panic!("expected named reexport, got {:?}", other),
        }

        let exported: HashSet<_> = analysis
            .exports
            .iter()
            .filter(|e| e.kind == "reexport")
            .map(|e| e.name.as_str())
            .collect();
        assert!(exported.contains("Bar"));
        assert!(exported.contains("Baz"));
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

        let export_names: Vec<_> = analysis.exports.iter().map(|e| e.name.as_str()).collect();
        assert!(export_names.contains(&"foo"));
        assert!(export_names.contains(&"bar"));
        assert!(!export_names.contains(&"_private"));
    }

    #[test]
    fn parses_all_list_with_comments() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
__all__ = [
    "foo",  # inline comment
    "bar",
    # "baz" is intentionally excluded
]
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

        let export_names: Vec<_> = analysis.exports.iter().map(|e| e.name.as_str()).collect();
        assert!(export_names.contains(&"foo"));
        assert!(export_names.contains(&"bar"));
        assert!(!export_names.iter().any(|n| n.contains('#')));
        assert!(!export_names.contains(&"baz"));
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

    #[test]
    fn detects_type_hint_usage() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
from collections import defaultdict
from typing import Dict, List

class UserRateLimit:
    pass

class Session:
    pass

rate_limits: dict[str, UserRateLimit] = {}
sessions: Dict[str, Session] = {}

user_limits = defaultdict(UserRateLimit)

def get_limit(user_id: str) -> UserRateLimit:
    return rate_limits[user_id]

def process(items: List[Session]) -> None:
    pass
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("session_security.py"),
            root,
            Some(&py_exts()),
            "session_security.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(
            analysis.local_uses.contains(&"UserRateLimit".to_string()),
            "UserRateLimit not found in local_uses: {:?}",
            analysis.local_uses
        );
        assert!(
            analysis.local_uses.contains(&"Session".to_string()),
            "Session not found in local_uses: {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn marks_pytest_fixture_as_used() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
import pytest

@pytest.fixture
def client():
    return object()
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("conftest.py"),
            root,
            Some(&py_exts()),
            "conftest.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(
            analysis.local_uses.contains(&"client".to_string()),
            "pytest fixture should be marked as used"
        );
        assert!(
            analysis.pytest_fixtures.contains(&"client".to_string()),
            "pytest fixture list should capture fixture name"
        );
    }

    #[test]
    fn captures_fastapi_route_metadata() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
from fastapi import APIRouter
router = APIRouter()

@router.get("/patients")
def list_patients():
    return []
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("api.py"),
            root,
            Some(&py_exts()),
            "api.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert_eq!(analysis.routes.len(), 1);
        let route = &analysis.routes[0];
        assert_eq!(route.framework, "fastapi");
        assert_eq!(route.method, "GET");
        assert_eq!(route.path.as_deref(), Some("/patients"));
        assert_eq!(route.name.as_deref(), Some("list_patients"));
    }

    #[test]
    fn captures_flask_route_methods_list() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
from flask import Blueprint
bp = Blueprint("bp", __name__)

@bp.route("/ping", methods=["GET", "POST"])
def ping():
    return "ok"
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("flask_app.py"),
            root,
            Some(&py_exts()),
            "flask_app.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert_eq!(analysis.routes.len(), 1);
        let route = &analysis.routes[0];
        assert_eq!(route.framework, "flask");
        assert_eq!(route.method, "GET,POST");
        assert_eq!(route.path.as_deref(), Some("/ping"));
        assert_eq!(route.name.as_deref(), Some("ping"));
    }

    #[test]
    fn golang_gdb_pattern_full_integration() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
class StringTypePrinter:
    pattern = re.compile(r'^struct string$')

class SliceTypePrinter:
    pattern = re.compile(r'^struct \[\]')

class MapTypePrinter:
    pattern = re.compile(r'^map\[')

class ChanTypePrinter:
    pattern = re.compile(r'^chan ')

class GoLenFunc(gdb.Function):
    how = ((StringTypePrinter, 'len'),
           (SliceTypePrinter, 'len'),
           (MapTypePrinter, 'used'),
           (ChanTypePrinter, 'qcount'))

    def invoke(self, obj):
        typename = str(obj.type)
        for klass, fld in self.how:
            if klass.pattern.match(typename):
                return obj[fld]
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("gdb_golang.py"),
            root,
            Some(&py_exts()),
            "gdb_golang.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(
            analysis
                .local_uses
                .contains(&"StringTypePrinter".to_string()),
            "StringTypePrinter not found in local_uses: {:?}",
            analysis.local_uses
        );
        assert!(
            analysis
                .local_uses
                .contains(&"SliceTypePrinter".to_string()),
            "SliceTypePrinter not found in local_uses"
        );
        assert!(
            analysis.local_uses.contains(&"MapTypePrinter".to_string()),
            "MapTypePrinter not found in local_uses"
        );
        assert!(
            analysis.local_uses.contains(&"ChanTypePrinter".to_string()),
            "ChanTypePrinter not found in local_uses"
        );

        let export_names: Vec<_> = analysis.exports.iter().map(|e| e.name.as_str()).collect();
        assert!(export_names.contains(&"StringTypePrinter"));
        assert!(export_names.contains(&"SliceTypePrinter"));
        assert!(export_names.contains(&"MapTypePrinter"));
        assert!(export_names.contains(&"ChanTypePrinter"));
        assert!(export_names.contains(&"GoLenFunc"));
    }

    #[test]
    fn detects_mixin_class_usage() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let content = r#"
class ButtonsColumnMixin:
    """Mixin for button column functionality"""
    pass

class WagtailAdminDraftStateFormMixin:
    pass

class IndexViewOptionalFeaturesMixin:
    pass

class NullAdminURLFinder:
    """Class used in same-file reference"""
    pass

class MyView(IndexViewOptionalFeaturesMixin, ButtonsColumnMixin):
    pass

def get_finder():
    return NullAdminURLFinder

def check_column(column_class):
    if issubclass(column_class, ButtonsColumnMixin):
        return True
"#;

        let analysis = analyze_py_file(
            content,
            &root.join("views.py"),
            root,
            Some(&py_exts()),
            "views.py".to_string(),
            &[root.to_path_buf()],
            &HashSet::new(),
        );

        assert!(
            analysis
                .local_uses
                .contains(&"ButtonsColumnMixin".to_string()),
            "ButtonsColumnMixin should be marked as used (inheritance): {:?}",
            analysis.local_uses
        );
        assert!(
            analysis
                .local_uses
                .contains(&"IndexViewOptionalFeaturesMixin".to_string()),
            "IndexViewOptionalFeaturesMixin should be marked as used (inheritance): {:?}",
            analysis.local_uses
        );
        assert!(
            analysis
                .local_uses
                .contains(&"NullAdminURLFinder".to_string()),
            "NullAdminURLFinder should be marked as used (function return): {:?}",
            analysis.local_uses
        );
    }

    #[test]
    fn handles_utf8_emoji_in_python_code() {
        let code = r#"
"""
This docstring has emoji and ellipsis and bullet points
"""

class MyClass:
    """Another docstring with emoji"""

    def method(self):
        return MyHelper  # Class reference after emoji content

class MyHelper:
    pass
"#;

        let temp = tempdir().unwrap();
        let py_file = temp.path().join("test_emoji.py");
        std::fs::write(&py_file, code).unwrap();

        let relative = py_file
            .strip_prefix(temp.path())
            .unwrap()
            .to_string_lossy()
            .to_string();
        let analysis = analyze_py_file(
            code,
            &py_file,
            temp.path(),
            Some(&py_exts()),
            relative,
            &[],
            &HashSet::new(),
        );

        assert!(analysis.exports.iter().any(|e| e.name == "MyClass"));
        assert!(analysis.exports.iter().any(|e| e.name == "MyHelper"));
    }
}
