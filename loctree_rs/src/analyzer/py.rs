use std::collections::HashSet;
use std::path::Path;

use crate::types::{
    ExportSymbol, FileAnalysis, ImportEntry, ImportKind, ReexportEntry, ReexportKind,
};

use super::regexes::{
    regex_py_all, regex_py_class, regex_py_def, regex_py_dynamic_dunder, regex_py_dynamic_importlib,
};
use super::resolvers::resolve_python_relative;

pub(crate) fn analyze_py_file(
    content: &str,
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
    relative: String,
) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative);

    for line in content.lines() {
        let without_comment = line.split('#').next().unwrap_or("").trim_end();
        let trimmed = without_comment.trim_start();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            for part in rest.split(',') {
                let mut name = part.trim();
                if let Some((lhs, _)) = name.split_once(" as ") {
                    name = lhs.trim();
                }
                if !name.is_empty() {
                    let mut entry = ImportEntry::new(name.to_string(), ImportKind::Static);
                    entry.resolved_path = resolve_python_relative(name, path, root, extensions);
                    analysis.imports.push(entry);
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("from ") {
            if let Some((module, names_raw)) = rest.split_once(" import ") {
                let module = module.trim().trim_end_matches('.');
                let names_clean = names_raw.trim().trim_matches('(').trim_matches(')');
                let names_clean = names_clean.split('#').next().unwrap_or("").trim();
                if !module.is_empty() {
                    let mut entry = ImportEntry::new(module.to_string(), ImportKind::Static);
                    entry.resolved_path = resolve_python_relative(module, path, root, extensions);
                    analysis.imports.push(entry);
                }
                if names_clean == "*" {
                    let resolved = resolve_python_relative(module, path, root, extensions);
                    analysis.reexports.push(ReexportEntry {
                        source: module.to_string(),
                        kind: ReexportKind::Star,
                        resolved,
                    });
                }
            }
        }
    }

    for caps in regex_py_dynamic_importlib().captures_iter(content) {
        if let Some(m) = caps.get(1) {
            analysis.dynamic_imports.push(m.as_str().to_string());
        }
    }
    for caps in regex_py_dynamic_dunder().captures_iter(content) {
        if let Some(m) = caps.get(1) {
            analysis.dynamic_imports.push(m.as_str().to_string());
        }
    }

    for caps in regex_py_all().captures_iter(content) {
        let body = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        for item in body.split(',') {
            let trimmed = item.trim();
            let name = trimmed
                .trim_matches(|c| c == '\'' || c == '"')
                .trim()
                .to_string();
            if !name.is_empty() {
                analysis
                    .exports
                    .push(ExportSymbol::new(name, "__all__", "named", None));
            }
        }
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

    analysis
}
