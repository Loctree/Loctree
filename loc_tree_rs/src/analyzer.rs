use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::json;

use crate::args::ParsedArgs;
use crate::fs_utils::{gather_files, normalise_ignore_patterns, GitIgnoreChecker};
use crate::types::{
    ExportIndex, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, Options, OutputMode,
    ReexportEntry, ReexportKind,
};

type RankedDup = (
    String,
    Vec<String>,
    usize,
    usize,
    usize,
    String,
    Vec<String>,
);

struct ReportSection {
    root: String,
    files_analyzed: usize,
    ranked_dups: Vec<RankedDup>,
    cascades: Vec<(String, String)>,
    dynamic: Vec<(String, Vec<String>)>,
    analyze_limit: usize,
}

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_html_report(path: &Path, sections: &[ReportSection]) -> io::Result<()> {
    let mut out = String::new();
    out.push_str(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8" />
<title>loctree import/export report</title>
<style>
body{font-family:system-ui,-apple-system,Segoe UI,Helvetica,Arial,sans-serif;margin:24px;line-height:1.5;}
h1,h2,h3{margin-bottom:0.2em;}
table{border-collapse:collapse;width:100%;margin:0.5em 0;}
th,td{border:1px solid #ddd;padding:6px 8px;font-size:14px;}
th{background:#f5f5f5;text-align:left;}
code{background:#f6f8fa;padding:2px 4px;border-radius:4px;}
.muted{color:#666;}
</style>
</head><body>
<h1>loctree import/export analysis</h1>
"#,
    );

    for section in sections {
        out.push_str(&format!(
            "<h2>{}</h2><p class=\"muted\">Files analyzed: {}</p>",
            escape_html(&section.root),
            section.files_analyzed
        ));

        // Duplicate exports
        out.push_str("<h3>Top duplicate exports</h3>");
        if section.ranked_dups.is_empty() {
            out.push_str("<p class=\"muted\">None</p>");
        } else {
            out.push_str("<table><tr><th>Symbol</th><th>Files</th><th>Prod</th><th>Dev</th><th>Canonical</th><th>Refactor targets</th></tr>");
            for (name, files, _score, prod, dev, canonical, refactors) in
                section.ranked_dups.iter().take(section.analyze_limit)
            {
                out.push_str(&format!(
                    "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
                    escape_html(name),
                    files.len(),
                    prod,
                    dev,
                    escape_html(canonical),
                    escape_html(&refactors.join(", "))
                ));
            }
            out.push_str("</table>");
        }

        // Cascades
        out.push_str("<h3>Re-export cascades</h3>");
        if section.cascades.is_empty() {
            out.push_str("<p class=\"muted\">None</p>");
        } else {
            out.push_str("<ul>");
            for (from, to) in &section.cascades {
                out.push_str(&format!(
                    "<li><code>{}</code> → <code>{}</code></li>",
                    escape_html(from),
                    escape_html(to)
                ));
            }
            out.push_str("</ul>");
        }

        // Dynamic imports
        out.push_str("<h3>Dynamic imports</h3>");
        if section.dynamic.is_empty() {
            out.push_str("<p class=\"muted\">None</p>");
        } else {
            out.push_str("<table><tr><th>File</th><th>Sources</th></tr>");
            for (file, sources) in section.dynamic.iter().take(section.analyze_limit) {
                out.push_str(&format!(
                    "<tr><td><code>{}</code></td><td>{}</td></tr>",
                    escape_html(file),
                    escape_html(&sources.join(", "))
                ));
            }
            out.push_str("</table>");
        }
    }

    out.push_str("</body></html>");
    fs::write(path, out)
}

fn open_in_browser(path: &Path) {
    let target = path.to_string_lossy();
    let try_cmds = if cfg!(target_os = "macos") {
        vec![vec!["open", target.as_ref()]]
    } else if cfg!(target_os = "windows") {
        vec![vec!["cmd", "/C", "start", target.as_ref()]]
    } else {
        vec![vec!["xdg-open", target.as_ref()]]
    };

    for cmd in try_cmds {
        let mut parts = cmd.into_iter();
        if let Some(program) = parts.next() {
            if Command::new(program).args(parts).spawn().is_ok() {
                return;
            }
        }
    }
    eprintln!(
        "[loctree][warn] Could not open report automatically: {}",
        target
    );
}

fn regex_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*import\s+([^;]+?)\s+from\s+["']([^"']+)["']"#).unwrap())
}

fn regex_side_effect_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*import\s+["']([^"']+)["']"#).unwrap())
}

fn regex_reexport_star() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*export\s+\*\s+from\s+["']([^"']+)["']"#).unwrap())
}

fn regex_reexport_named() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*export\s+\{([^}]+)\}\s+from\s+["']([^"']+)["']"#).unwrap()
    })
}

fn regex_dynamic_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"import\s*\(\s*["']([^"']+)["']\s*\)"#).unwrap())
}

fn regex_export_named_decl() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^\s*export\s+(?:async\s+)?(?:function|const|let|var|class|interface|type|enum)\s+([A-Za-z0-9_.$]+)"#,
        )
        .unwrap()
    })
}

fn regex_export_default() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*export\s+default(?:\s+(?:async\s+)?(?:function|class)\s+([A-Za-z0-9_.$]+))?"#).unwrap())
}

fn regex_export_brace() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*export\s+\{([^}]+)\}\s*;?"#).unwrap())
}

fn resolve_reexport_target(
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
    if candidate.extension().is_none() {
        if let Some(set) = exts {
            for ext in set {
                let with_ext = candidate.with_extension(ext);
                if with_ext.exists() {
                    return with_ext.canonicalize().ok().map(|p| {
                        p.strip_prefix(root)
                            .map(|q| q.to_string_lossy().to_string())
                            .unwrap_or_else(|_| p.to_string_lossy().to_string())
                    });
                }
            }
        }
    }
    if candidate.exists() {
        candidate.canonicalize().ok().map(|p| {
            p.strip_prefix(root)
                .map(|q| q.to_string_lossy().to_string())
                .unwrap_or_else(|_| p.to_string_lossy().to_string())
        })
    } else {
        None
    }
}

fn analyze_file(
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
) -> io::Result<FileAnalysis> {
    let content = std::fs::read_to_string(path)?;
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let mut imports = Vec::new();
    for caps in regex_import().captures_iter(&content) {
        let source = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
        imports.push(ImportEntry {
            source,
            kind: ImportKind::Static,
        });
    }
    for caps in regex_side_effect_import().captures_iter(&content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        imports.push(ImportEntry {
            source,
            kind: ImportKind::SideEffect,
        });
    }

    let mut reexports = Vec::new();
    for caps in regex_reexport_star().captures_iter(&content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let resolved = resolve_reexport_target(path, root, &source, extensions);
        reexports.push(ReexportEntry {
            source,
            kind: ReexportKind::Star,
            resolved,
        });
    }
    for caps in regex_reexport_named().captures_iter(&content) {
        let raw_names = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let source = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
        let names = brace_list_to_names(raw_names);
        let resolved = resolve_reexport_target(path, root, &source, extensions);
        reexports.push(ReexportEntry {
            source,
            kind: ReexportKind::Named(names.clone()),
            resolved,
        });
    }

    let mut dynamic_imports = Vec::new();
    for caps in regex_dynamic_import().captures_iter(&content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        dynamic_imports.push(source);
    }

    let mut exports = Vec::new();
    for caps in regex_export_named_decl().captures_iter(&content) {
        let name = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        if !name.is_empty() {
            exports.push(ExportSymbol {
                name,
                kind: "decl".to_string(),
            });
        }
    }
    for caps in regex_export_default().captures_iter(&content) {
        let name = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "default".to_string());
        exports.push(ExportSymbol {
            name,
            kind: "default".to_string(),
        });
    }
    for caps in regex_export_brace().captures_iter(&content) {
        let raw = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        for name in brace_list_to_names(raw) {
            exports.push(ExportSymbol {
                name,
                kind: "named".to_string(),
            });
        }
    }
    for re in &reexports {
        if let ReexportKind::Named(names) = &re.kind {
            for name in names {
                exports.push(ExportSymbol {
                    name: name.clone(),
                    kind: "reexport".to_string(),
                });
            }
        }
    }

    Ok(FileAnalysis {
        path: relative,
        imports,
        reexports,
        dynamic_imports,
        exports,
    })
}

fn is_dev_file(path: &str) -> bool {
    path.contains("__tests__")
        || path.contains("stories")
        || path.contains(".stories.")
        || path.contains("story.")
}

pub fn run_import_analyzer(root_list: &[PathBuf], parsed: &ParsedArgs) -> io::Result<()> {
    let mut json_results = Vec::new();
    let mut report_sections: Vec<ReportSection> = Vec::new();

    for (idx, root_path) in root_list.iter().enumerate() {
        let ignore_paths = normalise_ignore_patterns(&parsed.ignore_patterns, root_path);
        let mut extensions = parsed.extensions.clone();
        if extensions.is_none() {
            extensions = Some(default_analyzer_exts());
        }

        let options = Options {
            extensions: extensions.clone(),
            ignore_paths,
            use_gitignore: parsed.use_gitignore,
            max_depth: parsed.max_depth,
            color: parsed.color,
            output: parsed.output,
            summary: parsed.summary,
            summary_limit: parsed.summary_limit,
            show_hidden: parsed.show_hidden,
            loc_threshold: parsed.loc_threshold,
            analyze_limit: parsed.analyze_limit,
            report_path: parsed.report_path.clone(),
        };

        let git_checker = if options.use_gitignore {
            GitIgnoreChecker::new(root_path)
        } else {
            None
        };

        let mut files = Vec::new();
        gather_files(root_path, &options, 0, git_checker.as_ref(), &mut files)?;

        let mut analyses = Vec::new();
        let mut export_index: ExportIndex = HashMap::new();
        let mut reexport_edges: Vec<(String, Option<String>)> = Vec::new();
        let mut dynamic_summary: Vec<(String, Vec<String>)> = Vec::new();

        for file in files {
            let analysis = analyze_file(&file, root_path, options.extensions.as_ref())?;
            for exp in &analysis.exports {
                export_index
                    .entry(exp.name.clone())
                    .or_default()
                    .push(analysis.path.clone());
            }
            for re in &analysis.reexports {
                reexport_edges.push((analysis.path.clone(), re.resolved.clone()));
            }
            if !analysis.dynamic_imports.is_empty() {
                dynamic_summary.push((analysis.path.clone(), analysis.dynamic_imports.clone()));
            }
            analyses.push(analysis);
        }
        let duplicate_exports: Vec<_> = export_index
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .collect();

        let reexport_files: HashSet<String> = analyses
            .iter()
            .filter(|a| !a.reexports.is_empty())
            .map(|a| a.path.clone())
            .collect();

        let mut cascades = Vec::new();
        for (from, resolved) in &reexport_edges {
            if let Some(target) = resolved {
                if reexport_files.contains(target) {
                    cascades.push((from.clone(), target.clone()));
                }
            }
        }

        let mut ranked_dups = Vec::new();
        for (name, files) in &duplicate_exports {
            let dev_count = files.iter().filter(|f| is_dev_file(f)).count();
            let prod_count = files.len().saturating_sub(dev_count);
            let score = prod_count * 2 + dev_count;
            let canonical = files
                .iter()
                .find(|f| !is_dev_file(f))
                .cloned()
                .unwrap_or_else(|| files[0].clone());
            let mut refactors: Vec<String> =
                files.iter().filter(|f| *f != &canonical).cloned().collect();
            refactors.sort();
            ranked_dups.push((
                name.clone(),
                files.clone(),
                score,
                prod_count,
                dev_count,
                canonical,
                refactors,
            ));
        }
        ranked_dups.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.len().cmp(&a.1.len())));

        if options.report_path.is_some() {
            let mut sorted_dyn = dynamic_summary.clone();
            sorted_dyn.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
            report_sections.push(ReportSection {
                root: root_path.display().to_string(),
                files_analyzed: analyses.len(),
                ranked_dups: ranked_dups.clone(),
                cascades: cascades.clone(),
                dynamic: sorted_dyn,
                analyze_limit: options.analyze_limit,
            });
        }

        if matches!(options.output, OutputMode::Json | OutputMode::Jsonl) {
            let files_json: Vec<_> = analyses
                .iter()
                .map(|a| {
                    json!({
                        "path": a.path,
                        "imports": a.imports.iter().map(|i| json!({"source": i.source, "kind": match i.kind { ImportKind::Static => "static", ImportKind::SideEffect => "side-effect" }})).collect::<Vec<_>>(),
                        "reexports": a.reexports.iter().map(|r| {
                            match &r.kind {
                                ReexportKind::Star => json!({"source": r.source, "kind": "star", "resolved": r.resolved}),
                                ReexportKind::Named(names) => json!({"source": r.source, "kind": "named", "names": names, "resolved": r.resolved})
                            }
                        }).collect::<Vec<_>>(),
                        "dynamicImports": a.dynamic_imports,
                        "exports": a.exports.iter().map(|e| json!({"name": e.name, "kind": e.kind})).collect::<Vec<_>>(),
                    })
                })
                .collect();

            let payload = json!({
                "root": root_path,
                "filesAnalyzed": analyses.len(),
                "duplicateExports": duplicate_exports
                    .iter()
                    .map(|(name, files)| json!({"name": name, "files": files}))
                    .collect::<Vec<_>>(),
                "duplicateExportsRanked": ranked_dups
                    .iter()
                    .map(|(name, files, score, prod, dev, canonical, refactors)| json!({
                        "name": name,
                        "files": files,
                        "score": score,
                        "nonDevCount": prod,
                        "devCount": dev,
                        "canonical": canonical,
                        "refactorTargets": refactors,
                    }))
                    .collect::<Vec<_>>(),
                "reexportCascades": cascades
                    .iter()
                    .map(|(from, to)| json!({"from": from, "to": to}))
                    .collect::<Vec<_>>(),
                "dynamicImports": dynamic_summary
                    .iter()
                    .map(|(file, sources)| {
                        let unique: HashSet<_> = sources.iter().collect();
                        json!({
                            "file": file,
                            "sources": sources,
                            "manySources": sources.len() > 5,
                            "selfImport": unique.len() < sources.len(),
                        })
                    })
                    .collect::<Vec<_>>(),
                "files": files_json,
            });

            if matches!(options.output, OutputMode::Jsonl) {
                println!("{}", serde_json::to_string(&payload).unwrap());
            } else {
                json_results.push(payload);
            }
            continue;
        }

        if idx > 0 {
            println!();
        }

        println!("Import/export analysis for {}/", root_path.display());
        println!("  Files analyzed: {}", analyses.len());
        println!("  Duplicate exports: {}", duplicate_exports.len());
        println!("  Files with re-exports: {}", reexport_files.len());
        println!("  Dynamic imports: {}", dynamic_summary.len());

        if !duplicate_exports.is_empty() {
            println!(
                "\nTop duplicate exports (showing up to {}):",
                options.analyze_limit
            );
            for (name, files, score, prod, dev, canonical, refactors) in
                ranked_dups.iter().take(options.analyze_limit)
            {
                println!(
                    "  - {} (score {}, {} files: {} prod, {} dev) canonical: {} | refs: {}",
                    name,
                    score,
                    files.len(),
                    prod,
                    dev,
                    canonical,
                    refactors.join(", ")
                );
            }
        }

        if !cascades.is_empty() {
            println!("\nRe-export cascades:");
            for (from, to) in &cascades {
                println!("  - {} -> {}", from, to);
            }
        }

        if !dynamic_summary.is_empty() {
            println!(
                "\nDynamic imports (showing up to {}):",
                options.analyze_limit
            );
            let mut sorted_dyn = dynamic_summary.clone();
            sorted_dyn.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
            for (file, sources) in sorted_dyn.iter().take(options.analyze_limit) {
                println!(
                    "  - {}: {}{}",
                    file,
                    sources.join(", "),
                    if sources.len() > 5 {
                        "  [many sources]"
                    } else {
                        ""
                    }
                );
            }
        }

        println!("\nTip: rerun with --json for machine-readable output.");
    }

    if matches!(parsed.output, OutputMode::Json) {
        if json_results.len() == 1 {
            println!(
                "{}",
                serde_json::to_string_pretty(&json_results[0]).unwrap()
            );
        } else {
            println!("{}", serde_json::to_string_pretty(&json_results).unwrap());
        }
    }

    if let Some(report_path) = parsed.report_path.as_ref() {
        render_html_report(report_path, &report_sections)?;
        eprintln!("[loctree] HTML report written to {}", report_path.display());
        open_in_browser(report_path);
    }

    if let Some(limit) = parsed
        .mode
        .eq(&crate::types::Mode::AnalyzeImports)
        .then_some(parsed.analyze_limit)
    {
        // just to use limit in tree? no-op
        let _ = limit;
    }

    Ok(())
}

pub fn default_analyzer_exts() -> HashSet<String> {
    ["ts", "tsx", "js", "jsx", "mjs", "cjs"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn brace_list_to_names(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Some((_, alias)) = trimmed.split_once(" as ") {
                Some(alias.trim().to_string())
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}
