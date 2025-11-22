use std::fs;
use std::io;
use std::path::Path;

use super::assets::CYTOSCAPE_JS;
use super::open_server::url_encode_component;
use super::ReportSection;

fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn linkify(base: Option<&str>, file: &str, line: usize) -> String {
    if let Some(base) = base {
        let href = format!("{}/open?f={}&l={}", base, url_encode_component(file), line);
        format!("<a href=\"{}\">{}:{}</a>", href, file, line)
    } else {
        format!("{}:{}", file, line)
    }
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
.graph-controls button{font-size:12px;padding:4px 8px;border:1px solid #ccc;background:#f8f8f8;border-radius:6px;cursor:pointer;}
.graph-controls button:hover{background:#eee;}
.command-table th,.command-table td{vertical-align:top;}
.command-table code{background:transparent;color:inherit;font-weight:600;}
.command-pill{display:inline-block;padding:3px 6px;border-radius:6px;background:#eef2ff;color:#2b2f3a;font-size:12px;margin:2px 4px 2px 0;}
.dark .command-pill{background:#1f2635;color:#e9ecf5;}
.command-col{width:50%;}
.dark body{background:#0f1115;color:#d7dde5;}
.dark body{background:#0f1115;color:#d7dde5;}
.dark table th{background:#1c2029;color:#d7dde5;}
.dark table td{background:#0f1115;color:#d7dde5;border-color:#2a2f3a;}
.dark code{background:#1c2029;color:#f0f4ff;}
.dark .graph{border-color:#2a2f3a;}
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
        if section.command_counts == (0, 0) {
            out.push_str("<p class=\"muted\">No Tauri commands detected in this root.</p>");
        } else if section.missing_handlers.is_empty() && section.unused_handlers.is_empty() {
            out.push_str("<p class=\"muted\">All frontend calls have matching handlers.</p>");
        } else {
            out.push_str("<table class=\"command-table\"><tr><th>Missing handlers (FE→BE)</th><th>Handlers unused by FE</th></tr><tr><td>");
            if section.missing_handlers.is_empty() {
                out.push_str("<span class=\"muted\">None</span>");
            } else {
                out.push_str("<ul class=\"command-list\">");
                for g in &section.missing_handlers {
                    let locs: Vec<String> = g
                        .locations
                        .iter()
                        .map(|(f, l)| linkify(section.open_base.as_deref(), f, *l))
                        .collect();
                    out.push_str(&format!(
                        "<li><code>{}</code> <span class=\"muted\">{}</span></li>",
                        escape_html(&g.name),
                        locs.join(", ")
                    ));
                }
                out.push_str("</ul>");
            }
            out.push_str("</td><td>");
            if section.unused_handlers.is_empty() {
                out.push_str("<span class=\"muted\">None</span>");
            } else {
                out.push_str("<ul class=\"command-list\">");
                for g in &section.unused_handlers {
                    let locs: Vec<String> = g
                        .locations
                        .iter()
                        .map(|(f, l)| linkify(section.open_base.as_deref(), f, *l))
                        .collect();
                    out.push_str(&format!(
                        "<li><code>{}</code> <span class=\"muted\">{}</span></li>",
                        escape_html(&g.name),
                        locs.join(", ")
                    ));
                }
                out.push_str("</ul>");
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

    // Graph bootstrap (Cytoscape self-hosted)
    out.push_str(
        r#"<script src="loctree-cytoscape.min.js"></script>
<script>
(function(){
  const graphs = window.__LOCTREE_GRAPHS || [];
  graphs.forEach(g => {
    const container = document.getElementById(g.id);
    if (!container) return;

    // Controls
    const toolbar = document.createElement('div');
    toolbar.className = 'graph-toolbar';
    toolbar.innerHTML = `
      <label>filter:
        <input type="text" size="18" placeholder="substring (np. features/ai-suite, .tsx)" data-role="filter-text" />
      </label>
      <label>min degree:
        <input type="number" min="0" value="0" style="width:60px" data-role="min-degree" />
      </label>
      <label><input type="checkbox" data-role="toggle-labels" checked /> labels</label>
      <span class="graph-controls">
        <button data-role="fit">fit</button>
        <button data-role="reset">reset</button>
        <label><input type="checkbox" data-role="dark" /> dark</label>
        <button data-role="fullscreen">fullscreen</button>
        <button data-role="png">png</button>
        <button data-role="json">json</button>
      </span>
      <div class="graph-legend">
        <span><span class="legend-dot" style="background:#4f81e1"></span> file</span>
        <span><span class="legend-dot" style="background:#888"></span> import</span>
        <span><span class="legend-dot" style="background:#e67e22"></span> re-export</span>
      </div>
    `;
    container.parentNode.insertBefore(toolbar, container);
    const hint = document.createElement('div');
    hint.className = 'graph-hint';
    hint.textContent = 'Filtr dopasowuje substring do ścieżki węzła (ścieżki są względne wobec roota). Ustaw min degree, aby ukryć liście.';
    container.parentNode.insertBefore(hint, container);

    const buildElements = () => {
      const rawNodes = Array.isArray(g.nodes) ? g.nodes : [];
      const rawEdges = Array.isArray(g.edges) ? g.edges : [];
      const nodes = rawNodes.map(n => {
        const size = Math.max(4, Math.min(30, Math.sqrt((n && n.loc) || 1)));
        return {
          data: { id: n.id || '', label: n.label || n.id || '', loc: n.loc || 0, size, full: n.id || '' },
          position: { x: n.x || 0, y: n.y || 0 }
        };
      });
      const edges = rawEdges.map((e, idx) => {
        const kind = (e && e[2]) || 'import';
        const color = kind === 'reexport' ? '#e67e22' : '#888';
        return {
          data: { id: 'e'+idx, source: e[0], target: e[1], label: kind, color }
        };
      });
      return { nodes, edges };
    };

    const original = buildElements();
    const emptyOverlay = document.createElement('div');
    emptyOverlay.className = 'graph-empty';
    emptyOverlay.style.display = 'none';
    container.appendChild(emptyOverlay);
    let cy = cytoscape({
      container,
      elements: original,
      style: [
        { selector: 'node', style: { 'label': 'data(label)', 'font-size': 10, 'text-wrap': 'wrap', 'text-max-width': 120, 'background-color': '#4f81e1', 'color': '#fff', 'width': 'data(size)', 'height': 'data(size)' } },
        { selector: 'edge', style: { 'curve-style': 'bezier', 'width': 1.1, 'line-color': 'data(color)', 'target-arrow-color': 'data(color)', 'target-arrow-shape': 'triangle', 'arrow-scale': 0.7, 'label': '', 'font-size': 9, 'text-background-color': '#fff', 'text-background-opacity': 0.8, 'text-background-padding': 2 } }
      ],
      layout: { name: 'preset', animate: false, fit: true }
      });

    const download = (filename, data, mime) => {
      let blob = null;
      if (data instanceof Blob) {
        blob = data;
      } else if (typeof data === 'string' && data.startsWith('data:')) {
        const parts = data.split(',');
        const b64 = parts.length > 1 ? parts[1] : '';
        const bin = atob(b64);
        const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        blob = new Blob([bytes], { type: mime || parts[0].split(';')[0].replace('data:','') || 'application/octet-stream' });
      } else {
        blob = new Blob([data], {type: mime || 'application/octet-stream'});
      }
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 500);
    };

    const applyFilters = () => {
      const text = (toolbar.querySelector('[data-role="filter-text"]')?.value || '').toLowerCase();
      const minDeg = parseInt(toolbar.querySelector('[data-role="min-degree"]')?.value || '0', 10) || 0;

      let nodes = original.nodes.map(n => ({
        data: { ...n.data },
        position: { ...n.position }
      }));
      if (text) {
        nodes = nodes.filter(n => (n.data.id || '').toLowerCase().includes(text));
      }
      const nodeSet = new Set(nodes.map(n => n.data.id));
      let edges = original.edges.map(e => ({ data: { ...e.data } }));
      edges = edges.filter(e => nodeSet.has(e.data.source) && nodeSet.has(e.data.target));

      if (minDeg > 0) {
        const deg = {};
        edges.forEach(e => {
          deg[e.data.source] = (deg[e.data.source] || 0) + 1;
          deg[e.data.target] = (deg[e.data.target] || 0) + 1;
        });
        nodes = nodes.filter(n => (deg[n.data.id] || 0) >= minDeg);
        const filteredSet = new Set(nodes.map(n => n.data.id));
        edges = edges.filter(e => filteredSet.has(e.data.source) && filteredSet.has(e.data.target));
      }

      if (nodes.length === 0) {
        emptyOverlay.style.display = 'block';
        cy.elements().remove();
        return;
      }
      emptyOverlay.style.display = 'none';

      cy.elements().remove();
      cy.add({ nodes, edges });

      const showLabels = toolbar.querySelector('[data-role="toggle-labels"]').checked;
      const autoHide = nodes.length > 800;
      const labelsOn = showLabels && !autoHide;
      cy.style()
        .selector('node').style('label', labelsOn ? 'data(label)' : '')
        .update();

      cy.layout({ name: 'preset', animate: false, fit: true }).run();
    };

    // Fit / reset / dark / fullscreen
    const fitBtn = toolbar.querySelector('[data-role="fit"]');
    const resetBtn = toolbar.querySelector('[data-role="reset"]');
    const darkChk = toolbar.querySelector('[data-role="dark"]');
    const fsBtn = toolbar.querySelector('[data-role="fullscreen"]');
    const pngBtn = toolbar.querySelector('[data-role="png"]');
    const jsonBtn = toolbar.querySelector('[data-role="json"]');

    if (fitBtn) fitBtn.addEventListener('click', () => cy.fit());
    if (resetBtn) resetBtn.addEventListener('click', () => {
      cy.elements().remove();
      cy.add(original);
      cy.layout({ name: 'preset', animate: false, fit: true }).run();
      applyFilters();
    });

    if (pngBtn) pngBtn.addEventListener('click', () => {
      const dark = darkChk && darkChk.checked;
      const dataUrl = cy.png({ bg: dark ? '#0f1115' : '#ffffff', full: true, scale: 2 });
      download(`${g.id}-graph.png`, dataUrl, 'image/png');
    });
    if (jsonBtn) jsonBtn.addEventListener('click', () => {
      const payload = {
        nodes: cy.nodes().map(n => n.data()),
        edges: cy.edges().map(e => ({ source: e.data('source'), target: e.data('target'), kind: e.data('kind') })),
        filter: toolbar.querySelector('[data-role="filter-text"]')?.value || '',
        minDegree: parseInt(toolbar.querySelector('[data-role="min-degree"]')?.value || '0', 10) || 0,
      };
      download(`${g.id}-graph.json`, JSON.stringify(payload, null, 2), 'application/json');
    });

    const applyDark = (on) => {
      document.documentElement.classList.toggle('dark', on);
      cy.style()
        .selector('node').style('color', on ? '#eef2ff' : '#fff').update()
        .selector('edge').style('text-background-color', on ? '#0f1115' : '#fff').update();
    };
    if (darkChk) {
      darkChk.addEventListener('change', () => applyDark(darkChk.checked));
    }

    const fsTarget = container;
    if (fsBtn && fsTarget && fsTarget.requestFullscreen) {
      fsBtn.addEventListener('click', () => {
        if (document.fullscreenElement) {
          document.exitFullscreen();
        } else {
          fsTarget.requestFullscreen().catch(()=>{});
        }
      });
      document.addEventListener('fullscreenchange', () => {
        fsBtn.textContent = document.fullscreenElement ? 'exit fullscreen' : 'fullscreen';
        if (!document.fullscreenElement) {
          cy.fit();
        }
      });
    }

    // Tooltip on hover/click
    const tooltip = document.createElement('div');
    tooltip.style.position = 'fixed';
    tooltip.style.pointerEvents = 'none';
    tooltip.style.background = '#111';
    tooltip.style.color = '#fff';
    tooltip.style.padding = '6px 8px';
    tooltip.style.borderRadius = '6px';
    tooltip.style.fontSize = '12px';
    tooltip.style.display = 'none';
    tooltip.style.zIndex = 9999;
    document.body.appendChild(tooltip);

    const showTip = (evt, node) => {
        const data = node.data();
        tooltip.innerHTML = `<strong>${data.full || data.id}</strong><br/>LOC: ${data.loc || 0}`;
        tooltip.style.left = (evt.renderedPosition.x + 12) + 'px';
        tooltip.style.top = (evt.renderedPosition.y + 12) + 'px';
        tooltip.style.display = 'block';
    };
    const hideTip = () => { tooltip.style.display = 'none'; };

    cy.off('mouseover');
    cy.off('mouseout');
    cy.off('tap');
    cy.off('tapdrag');
    cy.on('mouseover', 'node', (evt) => showTip(evt, evt.target));
    cy.on('mouseout', 'node', hideTip);
    cy.on('tap', 'node', (evt) => showTip(evt, evt.target));
    cy.on('tapdrag', 'node', hideTip);

    toolbar.querySelectorAll('input').forEach(inp => {
      inp.addEventListener('input', () => applyFilters());
      inp.addEventListener('change', () => applyFilters());
    });
  });
})();
</script>"#,
    );

    out.push_str("</body></html>");
    fs::write(path, out)
}
