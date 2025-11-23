use std::fs;
use std::io;
use std::path::Path;

use super::assets::CYTOSCAPE_JS;
use super::open_server::url_encode_component;
use super::ReportSection;

const GRAPH_BOOTSTRAP: &str = include_str!("graph_bootstrap.js");

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn linkify(base: Option<&str>, file: &str, line: usize) -> String {
    if let Some(base) = base {
        let href = format!("{}/open?f={}&l={}", base, url_encode_component(file), line);
        format!("<a href=\"{}\">{}:{}</a>", href, file, line)
    } else {
        format!("{}:{}", file, line)
    }
}

fn render_ai_insights(out: &mut String, section: &ReportSection) {
    if section.insights.is_empty() {
        return;
    }
    out.push_str("<h3>AI Insights</h3><ul class=\"command-list\">");
    for insight in &section.insights {
        let color = match insight.severity.as_str() {
            "high" => "#e74c3c",
            "medium" => "#e67e22",
            _ => "#3498db",
        };
        out.push_str(&format!(
            "<li><strong style=\"color:{}\">{}</strong>: {}</li>",
            color,
            escape_html(&insight.title),
            escape_html(&insight.message)
        ));
    }
    out.push_str("</ul>");
}

fn render_duplicate_exports(out: &mut String, section: &ReportSection) {
    out.push_str("<h3>Top duplicate exports</h3>");
    if section.ranked_dups.is_empty() {
        out.push_str("<p class=\"muted\">None</p>");
    } else {
        out.push_str("<table><tr><th>Symbol</th><th>Files</th><th>Prod</th><th>Dev</th><th>Canonical</th><th>Refactor targets</th></tr>");
        for dup in section.ranked_dups.iter().take(section.analyze_limit) {
            out.push_str(&format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td></tr>",
                escape_html(&dup.name),
                dup.files.len(),
                dup.prod_count,
                dup.dev_count,
                escape_html(&dup.canonical),
                escape_html(&dup.refactors.join(", "))
            ));
        }
        out.push_str("</table>");
    }
}

fn render_cascades(out: &mut String, section: &ReportSection) {
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
}

fn render_dynamic_imports(out: &mut String, section: &ReportSection) {
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

fn render_command_coverage(out: &mut String, section: &ReportSection) {
    out.push_str("<h3>Tauri command coverage</h3>");
    if section.command_counts == (0, 0) {
        out.push_str("<p class=\"muted\">No Tauri commands detected in this root.</p>");
        return;
    }
    if section.missing_handlers.is_empty() && section.unused_handlers.is_empty() {
        out.push_str("<p class=\"muted\">All frontend calls have matching handlers.</p>");
        return;
    }

    let render_grouped = |gaps: &Vec<super::report::CommandGap>, out: &mut String| {
        if gaps.is_empty() {
            out.push_str("<span class=\"muted\">None</span>");
            return;
        }
        let mut groups: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for g in gaps {
            let module = g
                .locations
                .first()
                .map(|(p, _)| {
                    let parts: Vec<&str> = p.split('/').collect();
                    if parts.len() >= 2 {
                        format!("{}/{}", parts[0], parts[1])
                    } else {
                        parts.first().unwrap_or(&"").to_string()
                    }
                })
                .unwrap_or_else(|| "".to_string());
            let locs: Vec<String> = g
                .locations
                .iter()
                .map(|(f, l)| linkify(section.open_base.as_deref(), f, *l))
                .collect();

            let alias_info = if let Some(impl_name) = &g.implementation_name {
                if impl_name != &g.name {
                    format!(
                        " <span class=\"muted\">(impl: {})</span>",
                        escape_html(impl_name)
                    )
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let pill = format!(
                "<span class=\"command-pill\"><code>{}</code>{}</span> <span class=\"muted\">{}</span>",
                escape_html(&g.name),
                alias_info,
                locs.join(", ")
            );
            groups.entry(module).or_default().push(pill);
        }
        for (module, items) in groups {
            let module_label = if module.is_empty() {
                "-".to_string()
            } else {
                escape_html(&module)
            };
            out.push_str(&format!(
                "<div class=\"module-group\"><div class=\"module-header\">{}</div><div>{}</div></div>",
                module_label,
                items.join("<br/>")
            ));
        }
    };

    out.push_str(
        "<table class=\"command-table\"><tr><th>Missing handlers (FE→BE)</th><th>Handlers unused by FE</th></tr><tr><td>",
    );
    render_grouped(&section.missing_handlers, out);
    out.push_str("</td><td>");
    render_grouped(&section.unused_handlers, out);
    out.push_str("</td></tr></table>");
}

fn render_graph_stub(out: &mut String, section: &ReportSection) {
    if let Some(warn) = &section.graph_warning {
        out.push_str(&format!(
            "<div class=\"graph-empty\">{}</div>",
            escape_html(warn)
        ));
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
        let components_json = serde_json::to_string(&graph.components).unwrap_or("[]".into());
        let open_json = serde_json::to_string(&section.open_base).unwrap_or("null".into());
        out.push_str("<script>");
        out.push_str("window.__LOCTREE_GRAPHS = window.__LOCTREE_GRAPHS || [];");
        out.push_str("window.__LOCTREE_GRAPHS.push({");
        out.push_str(&format!(
            "id:\"graph-{}\",nodes:{},edges:{},components:{},mainComponent:{},openBase:{}",
            escape_html(
                &section
                    .root
                    .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
            ),
            nodes_json,
            edges_json,
            components_json,
            graph.main_component_id,
            open_json
        ));
        out.push_str("});</script>");
    }
}

fn render_section(out: &mut String, section: &ReportSection) {
    out.push_str(&format!(
        "<h2>{}</h2><p class=\"muted\">Files analyzed: {}</p>",
        escape_html(&section.root),
        section.files_analyzed
    ));

    render_ai_insights(out, section);
    render_duplicate_exports(out, section);
    render_cascades(out, section);
    render_dynamic_imports(out, section);
    render_command_coverage(out, section);
    render_graph_stub(out, section);
}

fn render_graph_bootstrap(out: &mut String) {
    out.push_str(GRAPH_BOOTSTRAP);
}

pub(crate) fn render_html_report(path: &Path, sections: &[ReportSection]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
        let js_path = dir.join("loctree-cytoscape.min.js");
        if !js_path.exists() {
            fs::write(&js_path, CYTOSCAPE_JS)?;
        }
    }

    let mut out = String::new();
    out.push_str(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8" />
<meta http-equiv="Content-Security-Policy" content="default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'none'; font-src 'self' data:;">
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
.command-table td{vertical-align:top;}
.command-list{margin:0;padding-left:1.1rem;columns:2;column-gap:1.4rem;list-style:disc;}
.command-list li{break-inside:avoid;word-break:break-word;margin-bottom:4px;}
.graph-toolbar{display:flex;flex-wrap:wrap;gap:8px;align-items:center;margin:6px 0 4px;}
.graph-toolbar label,.graph-legend{font-size:13px;color:#444;display:flex;align-items:center;gap:8px;}
.graph-legend{gap:12px;}
.legend-dot{width:12px;height:12px;border-radius:50%;display:inline-block;}
.graph-hint{font-size:12px;color:#555;margin:2px 0 6px;}
.graph-empty{font-size:13px;color:#777;text-align:center;padding:24px;}
.component-panel{border:1px solid #d5dce6;border-radius:10px;padding:8px 10px;margin:10px 0;background:#f8fafc;}
.component-panel-header{display:flex;align-items:center;justify-content:space-between;gap:10px;flex-wrap:wrap;}
.component-panel table{margin:6px 0 0 0;}
.component-panel .muted{font-size:12px;}
.component-chip{display:inline-block;padding:3px 6px;border-radius:6px;background:#eef2ff;color:#2b2f3a;font-size:12px;}
.component-panel .panel-actions{display:flex;flex-wrap:wrap;align-items:center;gap:8px;}
.component-toolbar{margin-bottom:6px;}
.component-toolbar select,.component-toolbar input[type=\"range\"],.component-toolbar input[type=\"number\"]{font-size:12px;}
.graph-controls button{font-size:12px;padding:4px 8px;border:1px solid #ccc;background:#f8f8f8;border-radius:6px;cursor:pointer;}
.graph-controls button:hover{background:#eee;}
.command-table th,.command-table td{vertical-align:top;}
.command-table code{background:transparent;color:inherit;font-weight:600;}
.command-pill{display:inline-block;padding:3px 6px;border-radius:6px;background:#eef2ff;color:#2b2f3a;font-size:12px;margin:2px 4px 2px 0;}
.dark .command-pill{background:#1f2635;color:#e9ecf5;}
.command-col{width:50%;}
.module-header{font-weight:700;margin-top:4px;}
.module-group{margin-bottom:10px;}
.graph-drawer{position:fixed;left:16px;right:16px;bottom:12px;z-index:1100;background:#f5f7fb;border:1px solid #cfd4de;border-radius:12px;box-shadow:0 8px 32px rgba(0,0,0,.25);padding:8px 10px;}
.graph-drawer.collapsed{opacity:0.9;}
.graph-drawer-header{display:flex;align-items:center;gap:10px;cursor:pointer;font-weight:600;}
.graph-drawer-header button{font-size:12px;padding:4px 8px;border:1px solid #ccc;background:#fff;border-radius:6px;cursor:pointer;}
.graph-drawer-body{margin-top:6px;}
.graph-drawer .graph{margin:0;border-color:#cfd4de;}
.dark body{background:#0f1115;color:#d7dde5;}
.dark table th{background:#1c2029;color:#d7dde5;}
.dark table td{background:#0f1115;color:#d7dde5;border-color:#2a2f3a;}
.dark code{background:#1c2029;color:#f0f4ff;}
.dark .graph{border-color:#2a2f3a;}
.dark .graph-drawer{background:#0b0d11;border-color:#2a2f3a;box-shadow:0 8px 32px rgba(0,0,0,.45);}
.dark .graph-drawer-header button{background:#111522;color:#e9ecf5;border-color:#2a2f3a;}
.dark .component-panel{background:#0f131c;border-color:#2a2f3a;}
.dark .component-chip{background:#1f2635;color:#e9ecf5;}
</style>
</head><body>
<h1>loctree import/export analysis</h1>
"#,
    );

    for section in sections {
        render_section(&mut out, section);
    }

    render_graph_bootstrap(&mut out);

    out.push_str("</body></html>");
    fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::render_html_report;
    use crate::analyzer::report::{AiInsight, RankedDup, ReportSection};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn renders_basic_report() {
        let tmp_dir = tempdir().expect("tmp dir");
        let out_path = tmp_dir.path().join("report.html");

        let dup = RankedDup {
            name: "Foo".into(),
            files: vec!["a.ts".into(), "b.ts".into()],
            score: 2,
            prod_count: 2,
            dev_count: 0,
            canonical: "a.ts".into(),
            refactors: vec!["b.ts".into()],
        };

        let section = ReportSection {
            root: "test-root".into(),
            files_analyzed: 2,
            ranked_dups: vec![dup],
            cascades: vec![("a.ts".into(), "b.ts".into())],
            dynamic: vec![("dyn.ts".into(), vec!["./lazy".into()])],
            analyze_limit: 5,
            missing_handlers: Vec::new(),
            unused_handlers: Vec::new(),
            command_counts: (0, 0),
            open_base: None,
            graph: None,
            graph_warning: None,
            insights: vec![AiInsight {
                title: "Hint".into(),
                severity: "medium".into(),
                message: "Message".into(),
            }],
        };

        render_html_report(&out_path, &[section]).expect("render html");
        let html = fs::read_to_string(&out_path).expect("read html");
        assert!(html.contains("loctree import/export analysis"));
        assert!(html.contains("Hint"));
        assert!(html.contains("Foo"));
    }

    #[test]
    fn escapes_html_entities() {
        let tmp_dir = tempdir().expect("tmp dir");
        let out_path = tmp_dir.path().join("report.html");
        let malicious = r#"<script>alert('x')</script>"#;
        let section = ReportSection {
            root: malicious.into(),
            files_analyzed: 0,
            ranked_dups: Vec::new(),
            cascades: Vec::new(),
            dynamic: Vec::new(),
            analyze_limit: 1,
            missing_handlers: Vec::new(),
            unused_handlers: Vec::new(),
            command_counts: (0, 0),
            open_base: None,
            graph: None,
            graph_warning: Some(
                "Graph skipped (10000 nodes, 500 edges exceed limits of 8000 nodes / 12000 edges)"
                    .into(),
            ),
            insights: Vec::new(),
        };

        render_html_report(&out_path, &[section]).expect("render html");
        let html = fs::read_to_string(&out_path).expect("read html");
        assert!(!html.contains(malicious));
        assert!(html.contains("&lt;script&gt;alert(&#x27;x&#x27;)&lt;/script&gt;"));
        assert!(html.contains("Graph skipped"));
    }
}
