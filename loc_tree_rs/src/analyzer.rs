use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::thread;

use regex::Regex;

use crate::types::CommandRef;
use serde_json::json;

use crate::args::{preset_ignore_symbols, ParsedArgs};
use crate::fs_utils::{gather_files, normalise_ignore_patterns, GitIgnoreChecker};
use crate::types::{
    ExportIndex, ExportSymbol, FileAnalysis, ImportEntry, ImportKind, Options, OutputMode,
    ReexportEntry, ReexportKind,
};

static OPEN_SERVER_BASE: OnceLock<String> = OnceLock::new();

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
    missing_handlers: Vec<CommandGap>,
    unused_handlers: Vec<CommandGap>,
    open_base: Option<String>,
    graph: Option<GraphData>,
}

#[derive(Clone)]
struct CommandGap {
    name: String,
    locations: Vec<(String, usize)>,
}

#[derive(Clone)]
struct GraphData {
    nodes: Vec<String>,
    edges: Vec<(String, String, String)>, // from, to, kind
}

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn url_encode_component(input: &str) -> String {
    input
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

fn url_decode_component(input: &str) -> Option<String> {
    let mut out = String::new();
    let mut iter = input.as_bytes().iter().cloned();
    while let Some(b) = iter.next() {
        if b == b'%' {
            let hi = iter.next()?;
            let lo = iter.next()?;
            let hex = [hi, lo];
            let s = std::str::from_utf8(&hex).ok()?;
            let v = u8::from_str_radix(s, 16).ok()?;
            out.push(v as char);
        } else {
            out.push(b as char);
        }
    }
    Some(out)
}

fn linkify(base: Option<&str>, file: &str, line: usize) -> String {
    if let Some(base) = base {
        let href = format!("{}/open?f={}&l={}", base, url_encode_component(file), line);
        format!("<a href=\"{}\">{}:{}</a>", href, file, line)
    } else {
        format!("{}:{}", file, line)
    }
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
.graph{height:520px;border:1px solid #ddd;border-radius:8px;margin:12px 0;}
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

        // Command coverage
        out.push_str("<h3>Tauri command coverage</h3>");
        if section.missing_handlers.is_empty() && section.unused_handlers.is_empty() {
            out.push_str("<p class=\"muted\">All frontend calls have matching handlers.</p>");
        } else {
            out.push_str("<table><tr><th>Missing handlers (FE→BE)</th><th>Handlers unused by FE</th></tr><tr><td>");
            if section.missing_handlers.is_empty() {
                out.push_str("<span class=\"muted\">None</span>");
            } else {
                let lines: Vec<String> = section
                    .missing_handlers
                    .iter()
                    .map(|g| {
                        let locs: Vec<String> = g
                            .locations
                            .iter()
                            .map(|(f, l)| linkify(section.open_base.as_deref(), f, *l))
                            .collect();
                        format!("{} ({})", g.name, locs.join("; "))
                    })
                    .collect();
                out.push_str(&escape_html(&lines.join(" · ")));
            }
            out.push_str("</td><td>");
            if section.unused_handlers.is_empty() {
                out.push_str("<span class=\"muted\">None</span>");
            } else {
                let lines: Vec<String> = section
                    .unused_handlers
                    .iter()
                    .map(|g| {
                        let locs: Vec<String> = g
                            .locations
                            .iter()
                            .map(|(f, l)| linkify(section.open_base.as_deref(), f, *l))
                            .collect();
                        format!("{} ({})", g.name, locs.join("; "))
                    })
                    .collect();
                out.push_str(&escape_html(&lines.join(" · ")));
            }
            out.push_str("</td></tr></table>");
        }

        if let Some(graph) = &section.graph {
            out.push_str("<h3>Import graph</h3>");
            out.push_str(&format!(
                "<div class=\"graph\" id=\"graph-{}\"></div>",
                escape_html(
                    &section
                        .root
                        .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
                )
            ));
            let nodes_json = serde_json::to_string(&graph.nodes).unwrap_or("[]".into());
            let edges_json = serde_json::to_string(&graph.edges).unwrap_or("[]".into());
            out.push_str("<script>");
            out.push_str("window.__LOCTREE_GRAPHS = window.__LOCTREE_GRAPHS || [];");
            out.push_str("window.__LOCTREE_GRAPHS.push({");
            out.push_str(&format!(
                "id:\"graph-{}\",nodes:{},edges:{}",
                escape_html(
                    &section
                        .root
                        .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
                ),
                nodes_json,
                edges_json
            ));
            out.push_str("});</script>");
        }
    }

    // Graph bootstrap (Cytoscape via CDN)
    out.push_str(
        r#"<script src="https://unpkg.com/cytoscape@3.26.0/dist/cytoscape.min.js"></script>
<script>
(function(){
  const graphs = window.__LOCTREE_GRAPHS || [];
  graphs.forEach(g => {
    const container = document.getElementById(g.id);
    if (!container) return;
    const nodes = Array.from(new Set([].concat(g.nodes || []))).map(n => ({ data: { id: n, label: n }}));
    const edges = (g.edges || []).map((e, idx) => ({
      data: { id: 'e'+idx, source: e[0], target: e[1], label: e[2] }
    }));
    cytoscape({
      container,
      elements: { nodes, edges },
      style: [
        { selector: 'node', style: { 'label': 'data(label)', 'font-size': 10, 'text-wrap': 'wrap', 'text-max-width': 120, 'background-color': '#4f81e1', 'color': '#fff', 'width': 22, 'height': 22 } },
        { selector: 'edge', style: { 'curve-style': 'bezier', 'width': 1.5, 'line-color': '#888', 'target-arrow-color': '#888', 'target-arrow-shape': 'triangle', 'arrow-scale': 0.8, 'label': 'data(label)', 'font-size': 9, 'text-background-color': '#fff', 'text-background-opacity': 0.8, 'text-background-padding': 2 } }
      ],
      layout: { name: 'cose', idealEdgeLength: 120, nodeOverlap: 8, padding: 20 }
    });
  });
})();
</script>"#,
    );

    out.push_str("</body></html>");
    fs::write(path, out)
}

fn open_in_browser(path: &Path) {
    let Ok(canon) = path.canonicalize() else {
        eprintln!(
            "[loctree][warn] Could not resolve report path for auto-open: {}",
            path.display()
        );
        return;
    };

    let target = canon.to_string_lossy().to_string();
    if target.bytes().any(|b| b < 0x20) {
        eprintln!(
            "[loctree][warn] Skipping auto-open for suspicious path: {}",
            target
        );
        return;
    }

    #[cfg(target_os = "macos")]
    let try_cmds = vec![("open", vec![target.as_str()])];
    #[cfg(target_os = "windows")]
    let try_cmds = vec![(
        "powershell",
        vec![
            "-NoProfile",
            "-Command",
            "Start-Process",
            "-FilePath",
            target.as_str(),
        ],
    )];
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let try_cmds = vec![("xdg-open", vec![target.as_str()])];

    for (program, args) in try_cmds {
        if Command::new(program).args(args.clone()).spawn().is_ok() {
            return;
        }
    }
    eprintln!(
        "[loctree][warn] Could not open report automatically: {}",
        target
    );
}

fn open_file_in_editor(
    full_path: &Path,
    line: usize,
    editor_cmd: Option<&String>,
) -> io::Result<()> {
    if let Some(tpl) = editor_cmd {
        let replaced = tpl
            .replace("{file}", full_path.to_string_lossy().as_ref())
            .replace("{line}", &line.to_string());
        let parts: Vec<String> = replaced.split_whitespace().map(|s| s.to_string()).collect();
        if let Some((prog, args)) = parts.split_first() {
            let status = Command::new(prog).args(args).status()?;
            if status.success() {
                return Ok(());
            }
        }
    } else if Command::new("code")
        .arg("-g")
        .arg(format!("{}:{}", full_path.to_string_lossy(), line.max(1)))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    let fallback = Command::new("open")
        .arg(full_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    #[cfg(target_os = "windows")]
    let fallback = Command::new("cmd")
        .args(["/C", "start", full_path.to_string_lossy().as_ref()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let fallback = Command::new("xdg-open")
        .arg(full_path)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if fallback {
        Ok(())
    } else {
        Err(io::Error::other("could not open file via editor"))
    }
}

fn handle_open_request(
    stream: &mut TcpStream,
    roots: &[PathBuf],
    editor_cmd: Option<&String>,
    request_line: &str,
) {
    let mut parts = request_line.split_whitespace();
    let _method = parts.next();
    let target = parts.next().unwrap_or("/");
    if !target.starts_with("/open?") {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
        return;
    }
    let query = &target[6..];
    let mut file = None;
    let mut line = 1usize;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "f" => file = url_decode_component(v),
                "l" => {
                    line = v.parse::<usize>().unwrap_or(1).max(1);
                }
                _ => {}
            }
        }
    }
    let Some(rel_or_abs) = file else {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        return;
    };

    let mut candidate = None;
    let path_obj = PathBuf::from(&rel_or_abs);
    if path_obj.is_absolute() {
        if let Ok(canon) = path_obj.canonicalize() {
            if roots.iter().any(|r| canon.starts_with(r)) {
                candidate = Some(canon);
            }
        }
    } else {
        for root in roots {
            let joined = root.join(&path_obj);
            if let Ok(canon) = joined.canonicalize() {
                if canon.starts_with(root) {
                    candidate = Some(canon);
                    break;
                }
            }
        }
    }

    let Some(full) = candidate else {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n");
        return;
    };

    let status = open_file_in_editor(&full, line, editor_cmd);
    let response = if status.is_ok() {
        "HTTP/1.1 200 OK\r\n\r\nopened"
    } else {
        "HTTP/1.1 500 Internal Server Error\r\n\r\nopen failed"
    };
    let _ = stream.write_all(response.as_bytes());
}

fn start_open_server(
    roots: Vec<PathBuf>,
    editor_cmd: Option<String>,
) -> Option<(u16, thread::JoinHandle<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    let handle = thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut buf = String::new();
            let mut reader = io::BufReader::new(&stream);
            if reader.read_line(&mut buf).is_ok() {
                handle_open_request(&mut stream, &roots, editor_cmd.as_ref(), buf.trim());
            }
        }
    });
    Some((port, handle))
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

fn regex_safe_invoke() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"safeInvoke\(\s*["']([^"']+)["']"#).unwrap())
}

fn regex_invoke_snake() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"invokeSnake\(\s*["']([^"']+)["']"#).unwrap())
}

fn regex_tauri_command_fn() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)#\s*\[\s*tauri::command[^\]]*\]\s*pub\s+async\s+fn\s+([A-Za-z0-9_]+)"#)
            .unwrap()
    })
}

fn regex_css_import() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // @import "x.css";  @import url("x.css"); @import url(x.css);
        Regex::new(r#"(?m)@import\s+(?:url\()?["']?([^"'()\s]+)["']?\)?"#).unwrap()
    })
}

fn regex_rust_use() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*(?:pub\s*(?:\([^)]*\))?\s+)?use\s+([^;]+);"#).unwrap())
}

fn regex_rust_pub_use() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*pub\s*(?:\([^)]*\))?\s+use\s+([^;]+);"#).unwrap())
}

fn regex_rust_pub_item(kind: &str) -> Regex {
    // Matches visibility modifiers like pub(crate) and optional async for fn
    let pattern = format!(
        r#"(?m)^\s*pub\s*(?:\([^)]*\)\s*)?(?:async\s+)?{}\s+([A-Za-z0-9_]+)"#,
        kind
    );
    Regex::new(&pattern).unwrap()
}

fn regex_rust_pub_const_like(kind: &str) -> Regex {
    let pattern = format!(
        r#"(?m)^\s*pub\s*(?:\([^)]*\)\s*)?{}\s+([A-Za-z0-9_]+)"#,
        kind
    );
    Regex::new(&pattern).unwrap()
}

fn rust_pub_decl_regexes() -> &'static [Regex] {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        vec![
            regex_rust_pub_item("fn"),
            regex_rust_pub_item("struct"),
            regex_rust_pub_item("enum"),
            regex_rust_pub_item("trait"),
            regex_rust_pub_item("type"),
            regex_rust_pub_item("union"),
            regex_rust_pub_item("mod"),
        ]
    })
    .as_slice()
}

fn rust_pub_const_regexes() -> &'static [Regex] {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        vec![
            regex_rust_pub_const_like("const"),
            regex_rust_pub_const_like("static"),
        ]
    })
    .as_slice()
}

fn regex_py_dynamic_importlib() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"importlib\.import_module\(\s*["']([^"']+)["']"#).unwrap())
}

fn regex_py_dynamic_dunder() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"__import__\(\s*["']([^"']+)["']"#).unwrap())
}

fn regex_py_all() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?s)__all__\s*=\s*\[([^\]]*)\]"#).unwrap())
}

fn regex_py_def() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)"#).unwrap())
}

fn regex_py_class() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)"#).unwrap())
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

fn resolve_python_relative(
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

    if joined.extension().is_none() {
        if let Some(set) = exts {
            for ext in set {
                let candidate = joined.with_extension(ext);
                if candidate.exists() {
                    return candidate
                        .canonicalize()
                        .ok()
                        .and_then(|p| {
                            p.strip_prefix(root)
                                .ok()
                                .map(|q| q.to_string_lossy().to_string())
                        })
                        .or_else(|| {
                            candidate
                                .canonicalize()
                                .ok()
                                .map(|p| p.to_string_lossy().to_string())
                        });
                }
            }
        }
    }

    if joined.exists() {
        joined
            .canonicalize()
            .ok()
            .and_then(|p| {
                p.strip_prefix(root)
                    .ok()
                    .map(|q| q.to_string_lossy().to_string())
            })
            .or_else(|| {
                joined
                    .canonicalize()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
    } else {
        None
    }
}

fn resolve_js_relative(
    file_path: &Path,
    root: &Path,
    spec: &str,
    exts: Option<&HashSet<String>>,
) -> Option<String> {
    if !spec.starts_with('.') {
        return None;
    }
    let parent = file_path.parent()?;
    let mut candidate = parent.join(spec);
    if candidate.extension().is_none() {
        if let Some(set) = exts {
            for ext in set {
                let with_ext = candidate.with_extension(ext);
                if with_ext.exists() {
                    candidate = with_ext;
                    break;
                }
            }
        }
    }
    if candidate.exists() {
        candidate
            .canonicalize()
            .ok()
            .and_then(|p| {
                p.strip_prefix(root)
                    .ok()
                    .map(|q| q.to_string_lossy().to_string())
            })
            .or_else(|| {
                candidate
                    .canonicalize()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
    } else {
        None
    }
}

fn analyze_js_file(
    content: &str,
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
    relative: String,
) -> FileAnalysis {
    let mut imports = Vec::new();
    let mut command_calls = Vec::new();
    for caps in regex_import().captures_iter(content) {
        let source = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
        imports.push(ImportEntry {
            source,
            kind: ImportKind::Static,
        });
    }
    for caps in regex_side_effect_import().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        imports.push(ImportEntry {
            source,
            kind: ImportKind::SideEffect,
        });
    }

    for caps in regex_safe_invoke().captures_iter(content) {
        if let Some(cmd) = caps.get(1) {
            let line = offset_to_line(content, cmd.start());
            command_calls.push(CommandRef {
                name: cmd.as_str().to_string(),
                line,
            });
        }
    }
    for caps in regex_invoke_snake().captures_iter(content) {
        if let Some(cmd) = caps.get(1) {
            let line = offset_to_line(content, cmd.start());
            command_calls.push(CommandRef {
                name: cmd.as_str().to_string(),
                line,
            });
        }
    }

    let mut reexports = Vec::new();
    for caps in regex_reexport_star().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let resolved = resolve_reexport_target(path, root, &source, extensions);
        reexports.push(ReexportEntry {
            source,
            kind: ReexportKind::Star,
            resolved,
        });
    }
    for caps in regex_reexport_named().captures_iter(content) {
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
    for caps in regex_dynamic_import().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        dynamic_imports.push(source);
    }

    let mut exports = Vec::new();
    for caps in regex_export_named_decl().captures_iter(content) {
        let name = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        if !name.is_empty() {
            exports.push(ExportSymbol {
                name,
                kind: "decl".to_string(),
            });
        }
    }
    for caps in regex_export_default().captures_iter(content) {
        let name = caps
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "default".to_string());
        exports.push(ExportSymbol {
            name,
            kind: "default".to_string(),
        });
    }
    for caps in regex_export_brace().captures_iter(content) {
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

    FileAnalysis {
        path: relative,
        imports,
        reexports,
        dynamic_imports,
        exports,
        command_calls,
        command_handlers: Vec::new(),
    }
}

fn analyze_css_file(content: &str, relative: String) -> FileAnalysis {
    let mut imports = Vec::new();
    for caps in regex_css_import().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        imports.push(ImportEntry {
            source,
            kind: ImportKind::Static,
        });
    }

    FileAnalysis {
        path: relative,
        imports,
        reexports: Vec::new(),
        dynamic_imports: Vec::new(),
        exports: Vec::new(),
        command_calls: Vec::new(),
        command_handlers: Vec::new(),
    }
}

fn analyze_py_file(
    content: &str,
    path: &Path,
    root: &Path,
    extensions: Option<&HashSet<String>>,
    relative: String,
) -> FileAnalysis {
    let mut imports = Vec::new();
    let mut reexports = Vec::new();
    let mut dynamic_imports = Vec::new();
    let mut exports = Vec::new();

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
                    imports.push(ImportEntry {
                        source: name.to_string(),
                        kind: ImportKind::Static,
                    });
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("from ") {
            if let Some((module, names_raw)) = rest.split_once(" import ") {
                let module = module.trim().trim_end_matches('.');
                let names_clean = names_raw.trim().trim_matches('(').trim_matches(')');
                let names_clean = names_clean.split('#').next().unwrap_or("").trim();
                if !module.is_empty() {
                    imports.push(ImportEntry {
                        source: module.to_string(),
                        kind: ImportKind::Static,
                    });
                }
                if names_clean == "*" {
                    let resolved = resolve_python_relative(module, path, root, extensions);
                    reexports.push(ReexportEntry {
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
            dynamic_imports.push(m.as_str().to_string());
        }
    }
    for caps in regex_py_dynamic_dunder().captures_iter(content) {
        if let Some(m) = caps.get(1) {
            dynamic_imports.push(m.as_str().to_string());
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
                exports.push(ExportSymbol {
                    name,
                    kind: "__all__".to_string(),
                });
            }
        }
    }

    for caps in regex_py_def().captures_iter(content) {
        if let Some(name) = caps.get(1) {
            let n = name.as_str();
            if !n.starts_with('_') {
                exports.push(ExportSymbol {
                    name: n.to_string(),
                    kind: "def".to_string(),
                });
            }
        }
    }
    for caps in regex_py_class().captures_iter(content) {
        if let Some(name) = caps.get(1) {
            let n = name.as_str();
            if !n.starts_with('_') {
                exports.push(ExportSymbol {
                    name: n.to_string(),
                    kind: "class".to_string(),
                });
            }
        }
    }

    FileAnalysis {
        path: relative,
        imports,
        reexports,
        dynamic_imports,
        exports,
        command_calls: Vec::new(),
        command_handlers: Vec::new(),
    }
}

fn parse_rust_brace_names(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed == "self" {
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

fn analyze_rust_file(content: &str, relative: String) -> FileAnalysis {
    let mut imports = Vec::new();
    for caps in regex_rust_use().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        if !source.is_empty() {
            imports.push(ImportEntry {
                source: source.to_string(),
                kind: ImportKind::Static,
            });
        }
    }

    let mut reexports = Vec::new();
    let mut exports = Vec::new();

    for caps in regex_rust_pub_use().captures_iter(content) {
        let raw = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        if raw.is_empty() {
            continue;
        }

        if raw.contains('{') && raw.contains('}') {
            let mut parts = raw.splitn(2, '{');
            let prefix = parts.next().unwrap_or("").trim().trim_end_matches("::");
            let braces = parts.next().unwrap_or("").trim_end_matches('}').trim();
            let names = parse_rust_brace_names(braces);
            reexports.push(ReexportEntry {
                source: raw.to_string(),
                kind: ReexportKind::Named(names.clone()),
                resolved: None,
            });
            for name in names {
                exports.push(ExportSymbol {
                    name,
                    kind: "reexport".to_string(),
                });
            }
            let _ = prefix; // prefix retained for future resolution
        } else if raw.ends_with("::*") {
            reexports.push(ReexportEntry {
                source: raw.to_string(),
                kind: ReexportKind::Star,
                resolved: None,
            });
        } else {
            // pub use foo::bar as Baz;
            let (path_part, export_name) = if let Some((path, alias)) = raw.split_once(" as ") {
                (path.trim(), alias.trim())
            } else {
                let mut segments = raw.rsplitn(2, "::");
                let name = segments.next().unwrap_or(raw).trim();
                let _ = segments.next();
                (raw, name)
            };

            reexports.push(ReexportEntry {
                source: path_part.to_string(),
                kind: ReexportKind::Named(vec![export_name.to_string()]),
                resolved: None,
            });
            exports.push(ExportSymbol {
                name: export_name.to_string(),
                kind: "reexport".to_string(),
            });
        }
    }

    // public items
    for regex in rust_pub_decl_regexes() {
        for caps in regex.captures_iter(content) {
            if let Some(name) = caps.get(1) {
                exports.push(ExportSymbol {
                    name: name.as_str().to_string(),
                    kind: "decl".to_string(),
                });
            }
        }
    }

    for regex in rust_pub_const_regexes() {
        for caps in regex.captures_iter(content) {
            if let Some(name) = caps.get(1) {
                exports.push(ExportSymbol {
                    name: name.as_str().to_string(),
                    kind: "decl".to_string(),
                });
            }
        }
    }

    let mut command_handlers = Vec::new();
    for caps in regex_tauri_command_fn().captures_iter(content) {
        if let Some(name) = caps.get(1) {
            let line = offset_to_line(content, name.start());
            command_handlers.push(CommandRef {
                name: name.as_str().to_string(),
                line,
            });
        }
    }

    FileAnalysis {
        path: relative,
        imports,
        reexports,
        dynamic_imports: Vec::new(),
        exports,
        command_calls: Vec::new(),
        command_handlers,
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
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let analysis = match ext.as_str() {
        "rs" => analyze_rust_file(&content, relative),
        "css" => analyze_css_file(&content, relative),
        "py" => analyze_py_file(&content, path, root, extensions, relative),
        _ => analyze_js_file(&content, path, root, extensions, relative),
    };

    Ok(analysis)
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
    let mut server_handle = None;

    let mut ignore_exact: HashSet<String> = HashSet::new();
    let mut ignore_prefixes: Vec<String> = Vec::new();

    if let Some(preset_name) = parsed.ignore_symbols_preset.as_deref() {
        if let Some(set) = preset_ignore_symbols(preset_name) {
            for s in set {
                if s.ends_with('*') {
                    ignore_prefixes.push(s.trim_end_matches('*').to_string());
                } else {
                    ignore_exact.insert(s);
                }
            }
        } else {
            eprintln!(
                "[loctree][warn] unknown --ignore-symbols-preset '{}', ignoring",
                preset_name
            );
        }
    }

    if let Some(user_syms) = parsed.ignore_symbols.clone() {
        for s in user_syms {
            let lc = s.to_lowercase();
            if lc.ends_with('*') {
                ignore_prefixes.push(lc.trim_end_matches('*').to_string());
            } else {
                ignore_exact.insert(lc);
            }
        }
    }

    if parsed.serve {
        if let Some((port, handle)) =
            start_open_server(root_list.to_vec(), parsed.editor_cmd.clone())
        {
            let base = format!("http://127.0.0.1:{port}");
            let _ = OPEN_SERVER_BASE.set(base.clone());
            server_handle = Some(handle);
            eprintln!("[loctree] local open server at {}", base);
        } else {
            eprintln!("[loctree][warn] could not start open server; continue without --serve");
        }
    }

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
            serve: parsed.serve,
            editor_cmd: parsed.editor_cmd.clone(),
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
        let mut fe_commands: HashMap<String, Vec<(String, usize)>> = HashMap::new();
        let mut be_commands: HashMap<String, Vec<(String, usize)>> = HashMap::new();
        let mut graph_edges: Vec<(String, String, String)> = Vec::new();

        for file in files {
            let analysis = analyze_file(&file, root_path, options.extensions.as_ref())?;
            for exp in &analysis.exports {
                let name_lc = exp.name.to_lowercase();
                let ignored = ignore_exact.contains(&name_lc)
                    || ignore_prefixes.iter().any(|p| name_lc.starts_with(p));
                if ignored {
                    continue;
                }
                export_index
                    .entry(exp.name.clone())
                    .or_default()
                    .push(analysis.path.clone());
            }
            for re in &analysis.reexports {
                reexport_edges.push((analysis.path.clone(), re.resolved.clone()));
                if parsed.graph && options.report_path.is_some() {
                    if let Some(target) = &re.resolved {
                        graph_edges.push((
                            analysis.path.clone(),
                            target.clone(),
                            "reexport".to_string(),
                        ));
                    }
                }
            }
            if !analysis.dynamic_imports.is_empty() {
                dynamic_summary.push((analysis.path.clone(), analysis.dynamic_imports.clone()));
            }
            if parsed.graph && options.report_path.is_some() {
                let ext = file
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_default();
                for imp in &analysis.imports {
                    let resolved = match ext.as_str() {
                        "py" => {
                            if imp.source.starts_with('.') {
                                resolve_python_relative(
                                    &imp.source,
                                    &file,
                                    root_path,
                                    options.extensions.as_ref(),
                                )
                            } else {
                                None
                            }
                        }
                        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
                            if imp.source.starts_with('.') {
                                resolve_js_relative(
                                    &file,
                                    root_path,
                                    &imp.source,
                                    options.extensions.as_ref(),
                                )
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    if let Some(target) = resolved {
                        graph_edges.push((
                            analysis.path.clone(),
                            target,
                            match imp.kind {
                                ImportKind::Static | ImportKind::SideEffect => "import".to_string(),
                            },
                        ));
                    }
                }
            }
            for call in &analysis.command_calls {
                fe_commands
                    .entry(call.name.clone())
                    .or_default()
                    .push((analysis.path.clone(), call.line));
            }
            for handler in &analysis.command_handlers {
                be_commands
                    .entry(handler.name.clone())
                    .or_default()
                    .push((analysis.path.clone(), handler.line));
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

        let missing_handlers: Vec<CommandGap> = fe_commands
            .iter()
            .filter(|(name, _)| !be_commands.contains_key(*name))
            .map(|(name, locs)| CommandGap {
                name: name.clone(),
                locations: locs.clone(),
            })
            .collect();
        let unused_handlers: Vec<CommandGap> = be_commands
            .iter()
            .filter(|(name, _)| !fe_commands.contains_key(*name))
            .map(|(name, locs)| CommandGap {
                name: name.clone(),
                locations: locs.clone(),
            })
            .collect();

        let mut section_open = None;
        if options.report_path.is_some() && options.serve {
            if let Some(base) = OPEN_SERVER_BASE.get() {
                section_open = Some(base.clone());
            }
        }

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
                missing_handlers: {
                    let mut v = missing_handlers.clone();
                    v.sort_by(|a, b| a.name.cmp(&b.name));
                    v
                },
                unused_handlers: {
                    let mut v = unused_handlers.clone();
                    v.sort_by(|a, b| a.name.cmp(&b.name));
                    v
                },
                open_base: section_open,
                graph: if parsed.graph && options.report_path.is_some() && !graph_edges.is_empty() {
                    let mut nodes: HashSet<String> = HashSet::new();
                    for (a, b, _) in &graph_edges {
                        nodes.insert(a.clone());
                        nodes.insert(b.clone());
                    }
                    Some(GraphData {
                        nodes: nodes.into_iter().collect(),
                        edges: graph_edges.clone(),
                    })
                } else {
                    None
                },
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
                        "commandCalls": a.command_calls.iter().map(|c| json!({"name": c.name, "line": c.line})).collect::<Vec<_>>(),
                        "commandHandlers": a.command_handlers.iter().map(|c| json!({"name": c.name, "line": c.line})).collect::<Vec<_>>(),
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
                "commands": {
                    "frontend": fe_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "backend": be_commands.iter().map(|(k,v)| json!({"name": k, "locations": v})).collect::<Vec<_>>(),
                    "missingHandlers": missing_handlers.iter().map(|g| json!({"name": g.name, "locations": g.locations})).collect::<Vec<_>>(),
                    "unusedHandlers": unused_handlers.iter().map(|g| json!({"name": g.name, "locations": g.locations})).collect::<Vec<_>>(),
                },
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

        if !missing_handlers.is_empty() || !unused_handlers.is_empty() {
            println!("\nTauri command coverage:");
            if !missing_handlers.is_empty() {
                println!(
                    "  Missing handlers (frontend calls without backend): {}",
                    missing_handlers
                        .iter()
                        .map(|g| g.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if !unused_handlers.is_empty() {
                println!(
                    "  Unused handlers (backend not called by FE): {}",
                    unused_handlers
                        .iter()
                        .map(|g| g.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
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

    drop(server_handle);
    Ok(())
}

pub fn default_analyzer_exts() -> HashSet<String> {
    ["ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "css", "py"]
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

fn offset_to_line(content: &str, offset: usize) -> usize {
    content[..offset].bytes().filter(|b| *b == b'\n').count() + 1
}
