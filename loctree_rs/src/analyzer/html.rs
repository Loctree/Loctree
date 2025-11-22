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
        out.push_str(&format!(
            "<h2>{}</h2><p class=\"muted\">Files analyzed: {}</p>",
            escape_html(&section.root),
            section.files_analyzed
        ));

        // AI Insights
        if !section.insights.is_empty() {
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
                        "—".to_string()
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

            out.push_str("<table class=\"command-table\"><tr><th>Missing handlers (FE→BE)</th><th>Handlers unused by FE</th></tr><tr><td>");
            render_grouped(&section.missing_handlers, &mut out);
            out.push_str("</td><td>");
            render_grouped(&section.unused_handlers, &mut out);
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

    // Graph bootstrap (Cytoscape self-hosted)
    out.push_str(
        r#"<script src="loctree-cytoscape.min.js"></script>
<script>
(function(){
  const graphs = window.__LOCTREE_GRAPHS || [];
  const escapeHtml = (value = '') => String(value).replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c] || c));
  const formatNum = (n) => (typeof n === 'number' && n.toLocaleString) ? n.toLocaleString() : (n || 0);
  graphs.forEach((g, idx) => {
    const container = document.getElementById(g.id);
    if (!container) return;

    const components = Array.isArray(g.components) ? g.components : [];
    const componentMap = new Map();
    components.forEach(c => componentMap.set(c.id, c));
    const detachedSet = new Set(components.filter(c => c.detached).map(c => c.id));
    const mainComponentId = g.mainComponent || 0;
    const openBase = g.openBase || null;

    const useDrawer = graphs.length === 1;
    let drawerBody = null;
    if (useDrawer) {
      const drawer = document.createElement('div');
      drawer.className = 'graph-drawer';
      const header = document.createElement('div');
      header.className = 'graph-drawer-header';
      header.innerHTML = '<button data-role="drawer-toggle">hide graph</button><span>Import graph</span>';
      drawerBody = document.createElement('div');
      drawerBody.className = 'graph-drawer-body';
      container.parentNode.insertBefore(drawer, container);
      drawerBody.appendChild(container);
      drawer.appendChild(header);
      drawer.appendChild(drawerBody);
      document.body.appendChild(drawer);
      container.style.height = '460px';
      const toggle = header.querySelector('[data-role="drawer-toggle"]');
      let collapsed = false;
      const applyCollapse = () => {
        drawer.classList.toggle('collapsed', collapsed);
        drawerBody.style.display = collapsed ? 'none' : 'block';
        toggle.textContent = collapsed ? 'show graph' : 'hide graph';
      };
      header.addEventListener('click', () => {
        collapsed = !collapsed;
        applyCollapse();
      });
      applyCollapse();
    }

    // Component controls
    const targetParent = drawerBody || container.parentNode;
    const componentBar = document.createElement('div');
    componentBar.className = 'graph-toolbar component-toolbar';
    componentBar.innerHTML = `
      <label>Component filter:
        <select data-role="component-filter">
          <option value="all">All components</option>
          <option value="isolates">Isolates / size≤2</option>
          <option value="size">Size ≤ slider</option>
        </select>
      </label>
      <label>threshold:
        <input type="range" min="1" max="64" value="8" data-role="component-threshold" />
        <span data-role="component-threshold-label">8</span>
      </label>
      <span class="graph-controls">
        <button data-role="component-highlight">Highlight selected</button>
        <button data-role="component-dim">Dim others</button>
        <button data-role="component-copy">Copy file list</button>
        <button data-role="component-export-json">Export JSON</button>
        <button data-role="component-export-csv">Export CSV</button>
        <button data-role="component-show-isolates">Show isolates</button>
      </span>
    `;
    targetParent.insertBefore(componentBar, container);

    // Controls
    const toolbar = document.createElement('div');
    toolbar.className = 'graph-toolbar';
    toolbar.innerHTML = `
      <label>filter:
        <input type="text" size="18" placeholder="substring (e.g. features/ai-suite, .tsx)" data-role="filter-text" />
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
        <span><span class="legend-dot" style="background:#d1830f"></span> detached</span>
      </div>
    `;
    targetParent.insertBefore(toolbar, container);

    const componentPanel = document.createElement('div');
    componentPanel.className = 'component-panel';
    componentPanel.innerHTML = `
      <div class="component-panel-header">
        <div><strong>Disconnected components</strong> <span class="muted" data-role="component-summary"></span></div>
        <div class="panel-actions">
          <label>show size ≤ <input type="number" min="1" value="8" data-role="component-size-limit" style="width:70px" /></label>
          <button data-role="component-reset">Reset view</button>
        </div>
      </div>
      <table>
        <thead><tr><th>id</th><th>size</th><th>sample</th><th>isolated</th><th>edges</th><th>LOC</th><th>actions</th></tr></thead>
        <tbody data-role="component-table"></tbody>
      </table>
    `;
    targetParent.insertBefore(componentPanel, container);

    const hint = document.createElement('div');
    hint.className = 'graph-hint';
    hint.textContent = 'Component filter selects an island (or small islands) and highlights nodes; slider sets threshold for small components. Text filter still matches node paths.';
    targetParent.insertBefore(hint, container);

    const componentSelect = componentBar.querySelector('[data-role="component-filter"]');
    const sizeSlider = componentBar.querySelector('[data-role="component-threshold"]');
    const sizeLabel = componentBar.querySelector('[data-role="component-threshold-label"]');
    const tableBody = componentPanel.querySelector('[data-role="component-table"]');
    const summaryEl = componentPanel.querySelector('[data-role="component-summary"]');
    const sizeLimitInput = componentPanel.querySelector('[data-role="component-size-limit"]');
    const componentReset = componentPanel.querySelector('[data-role="component-reset"]');

    const addComponentOptions = () => {
      const sorted = [...components].sort((a, b) => a.size - b.size || a.id - b.id);
      sorted.forEach(comp => {
        const opt = document.createElement('option');
        opt.value = `comp-${comp.id}`;
        const labelSample = comp.sample || (Array.isArray(comp.nodes) && comp.nodes[0]) || '';
        opt.textContent = `C${comp.id} • ${comp.size} nodes • ${labelSample}`;
        opt.dataset.size = comp.size;
        componentSelect.appendChild(opt);
      });
    };
    addComponentOptions();

    const state = {
      viewComponents: new Set(),
      highlightComponents: new Set(),
      sizeThreshold: parseInt(sizeSlider?.value || '8', 10) || 8,
      dimOthers: true,
    };

    const syncSize = (val) => {
      const safe = Math.max(1, Math.min(128, val || state.sizeThreshold));
      state.sizeThreshold = safe;
      if (sizeLabel) sizeLabel.textContent = safe;
      if (sizeSlider && sizeSlider.value !== String(safe)) sizeSlider.value = safe;
      if (sizeLimitInput && sizeLimitInput.value !== String(safe)) sizeLimitInput.value = safe;
      const sizeOption = componentSelect.querySelector('option[value="size"]');
      if (sizeOption) sizeOption.textContent = `Size ≤ ${safe}`;
    };
    syncSize(state.sizeThreshold);

    const buildElements = () => {
      const rawNodes = Array.isArray(g.nodes) ? g.nodes : [];
      const rawEdges = Array.isArray(g.edges) ? g.edges : [];
      const nodeToComponent = new Map();
      const nodes = rawNodes.map(n => {
        const size = Math.max(4, Math.min(30, Math.sqrt((n && n.loc) || 1)));
        const comp = n.component || 0;
        const compSize = (componentMap.get(comp) || {}).size || 0;
        const detached = detachedSet.has(comp) || !!n.detached;
        const isolate = (n.degree || 0) == 0 || compSize <= 2;
        const id = n.id || '';
        nodeToComponent.set(id, comp);
        return {
          data: { id, label: n.label || id || '', loc: n.loc || 0, size, full: id || '', component: comp, degree: n.degree || 0, detached, componentSize: compSize, isolate: isolate ? 1 : 0, color: detached ? '#d1830f' : '#4f81e1' },
          position: { x: n.x || 0, y: n.y || 0 }
        };
      });
      const edges = rawEdges.map((e, idx) => {
        const kind = (e && e[2]) || 'import';
        const sourceComp = nodeToComponent.get(e[0]) || nodeToComponent.get(e[1]) || 0;
        const detached = detachedSet.has(sourceComp);
        const color = detached ? '#d1830f' : (kind === 'reexport' ? '#e67e22' : '#888');
        return {
          data: { id: 'e'+idx, source: e[0], target: e[1], label: kind, kind, color, component: sourceComp, detached: detached ? 1 : 0 }
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
        { selector: 'node', style: { 'label': 'data(label)', 'font-size': 10, 'text-wrap': 'wrap', 'text-max-width': 120, 'background-color': 'data(color)', 'color': '#fff', 'width': 'data(size)', 'height': 'data(size)', 'overlay-padding': 8, 'overlay-opacity': 0 } },
        { selector: 'node.detached', style: { 'background-color': '#d1830f' } },
        { selector: 'node.isolate', style: { 'border-width': 2, 'border-color': '#d74d26' } },
        { selector: 'node.highlight', style: { 'border-width': 3, 'border-color': '#111', 'shadow-blur': 12, 'shadow-color': '#111', 'shadow-opacity': 0.45, 'shadow-offset-x': 0, 'shadow-offset-y': 0, 'z-index': 999 } },
        { selector: 'node.dimmed', style: { 'opacity': 0.15 } },
        { selector: 'edge', style: { 'curve-style': 'bezier', 'width': 1.1, 'line-color': 'data(color)', 'target-arrow-color': 'data(color)', 'target-arrow-shape': 'triangle', 'arrow-scale': 0.7, 'label': '', 'font-size': 9, 'text-background-color': '#fff', 'text-background-opacity': 0.8, 'text-background-padding': 2 } },
        { selector: 'edge.detached', style: { 'line-color': '#d1830f', 'target-arrow-color': '#d1830f' } },
        { selector: 'edge.highlight', style: { 'width': 2, 'opacity': 0.9 } },
        { selector: 'edge.dimmed', style: { 'opacity': 0.08 } }
      ],
      layout: { name: 'preset', animate: false, fit: true }
      });

    const download = (filename, content, type) => {
      const blob = new Blob([content], { type });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 500);
    };

    const gatherSelectedComponents = () => {
      if (state.highlightComponents.size) return new Set(state.highlightComponents);
      if (state.viewComponents.size) return new Set(state.viewComponents);
      return new Set();
    };

    const applyHighlight = (forceDim) => {
      const highlightSet = gatherSelectedComponents();
      const dim = forceDim === undefined ? state.dimOthers : forceDim;

      cy.nodes().removeClass('dimmed highlight isolate detached');
      cy.edges().removeClass('dimmed highlight detached');

      cy.nodes().filter(n => n.data('detached')).addClass('detached');
      cy.edges().filter(e => e.data('detached')).addClass('detached');
      cy.nodes().filter(n => (n.data('isolate') || 0) === 1 || (n.data('componentSize') || 0) <= 2).addClass('isolate');

      if (!highlightSet.size) return;
      const nodes = cy.nodes().filter(n => highlightSet.has(n.data('component')));
      const edges = cy.edges().filter(e => highlightSet.has(e.data('component')));
      nodes.addClass('highlight');
      edges.addClass('highlight');
      if (dim) {
        cy.nodes().not(nodes).addClass('dimmed');
        cy.edges().not(edges).addClass('dimmed');
      }
    };

    const applyFilters = () => {
      const text = (toolbar.querySelector('[data-role="filter-text"]')?.value || '').toLowerCase();
      const minDeg = parseInt(toolbar.querySelector('[data-role="min-degree"]')?.value || '0', 10) || 0;
      const allowedComponents = state.viewComponents;

      let nodes = original.nodes.map(n => ({
        data: { ...n.data },
        position: { ...n.position }
      }));
      if (text) {
        nodes = nodes.filter(n => (n.data.id || '').toLowerCase().includes(text));
      }
      if (allowedComponents.size) {
        nodes = nodes.filter(n => allowedComponents.has(n.data.component));
      }
      let edges = original.edges.map(e => ({ data: { ...e.data } }));
      const nodeSet = new Set(nodes.map(n => n.data.id));
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
      applyHighlight();
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
      state.viewComponents = new Set();
      state.highlightComponents = new Set();
      applyFilters();
    });

    if (pngBtn) pngBtn.addEventListener('click', () => {
      const dark = darkChk && darkChk.checked;
      const dataUrl = cy.png({ bg: dark ? '#0f1115' : '#ffffff', full: true, scale: 2 });
      // cy.png already returns a data URL; use direct download to preserve format
      const a = document.createElement('a');
      a.href = dataUrl;
      a.download = `${g.id}-graph.png`;
      document.body.appendChild(a);
      a.click();
      a.remove();
    });
    if (jsonBtn) jsonBtn.addEventListener('click', () => {
      const payload = {
        nodes: cy.nodes().map(n => n.data()),
        edges: cy.edges().map(e => ({ source: e.data('source'), target: e.data('target'), kind: e.data('kind') })),
        filter: toolbar.querySelector('[data-role="filter-text"]')?.value || '',
        minDegree: parseInt(toolbar.querySelector('[data-role="min-degree"]')?.value || '0', 10) || 0,
        components,
        highlightedComponents: Array.from(state.highlightComponents),
        viewedComponents: Array.from(state.viewComponents),
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
    tooltip.style.pointerEvents = 'auto';
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
        const path = data.full || data.id;
        const comp = componentMap.get(data.component);
        const compLabel = comp ? `C${comp.id} (${comp.size} nodes${comp.detached ? ', detached' : ''})` : '—';
        tooltip.innerHTML = `
            <div style="margin-bottom:4px"><strong>${escapeHtml(path)}</strong></div>
            <div>LOC: ${data.loc || 0} | degree: ${data.degree || 0}</div>
            <div>component: ${compLabel}</div>
            <button data-role="copy-path" style="margin-top:4px;font-size:10px;cursor:pointer">copy path</button>
        `;
        const copyBtn = tooltip.querySelector('[data-role="copy-path"]');
        if (copyBtn) copyBtn.addEventListener('click', () => navigator.clipboard.writeText(path));
        const rect = container.getBoundingClientRect();
        const scrollX = window.scrollX || document.documentElement.scrollLeft || 0;
        const scrollY = window.scrollY || document.documentElement.scrollTop || 0;
        let left = rect.left + evt.renderedPosition.x + 12 + scrollX;
        let top = rect.top + evt.renderedPosition.y + 12 + scrollY;
        const maxLeft = scrollX + window.innerWidth - 220;
        if (left > maxLeft) left = maxLeft;
        tooltip.style.left = left + 'px';
        tooltip.style.top = top + 'px';
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

    const updateComponentFilter = () => {
      const val = componentSelect.value;
      const set = new Set();
      if (val === 'isolates') {
        components.filter(c => c.size <= 2 || c.isolated_count > 0).forEach(c => set.add(c.id));
      } else if (val === 'size') {
        components.filter(c => c.size <= state.sizeThreshold).forEach(c => set.add(c.id));
      } else if (val.startsWith('comp-')) {
        const id = parseInt(val.replace('comp-', ''), 10);
        if (Number.isFinite(id)) set.add(id);
      }
      state.viewComponents = set;
      state.highlightComponents = new Set(set);
      applyFilters();
    };

    const renderComponentTable = () => {
      const limit = parseInt(sizeLimitInput.value || state.sizeThreshold, 10) || state.sizeThreshold;
      syncSize(limit);
      const rows = [...components].sort((a, b) => a.size - b.size || a.id - b.id);
      const filtered = rows.filter(c => c.size <= limit);
      tableBody.innerHTML = '';
      filtered.forEach(comp => {
        const sample = comp.sample || (comp.nodes && comp.nodes[0]) || '';
        const sampleHref = openBase ? `${openBase}/open?f=${encodeURIComponent(sample)}&l=1` : null;
        const sampleCell = sampleHref ? `<a href="${sampleHref}">${escapeHtml(sample)}</a>` : `<code>${escapeHtml(sample)}</code>`;
        const warn = comp.detached ? ' ⚠️' : '';
        const tr = document.createElement('tr');
        const edgeCount = comp.edges !== undefined ? comp.edges : comp.edge_count;
        tr.innerHTML = `<td>C${comp.id}${warn}</td><td>${comp.size}</td><td>${sampleCell}</td><td>${comp.isolated_count}</td><td>${edgeCount || 0}</td><td>${formatNum(comp.loc_sum)}</td><td><button data-role="component-focus" data-comp="${comp.id}">Highlight</button></td>`;
        tableBody.appendChild(tr);
      });
      summaryEl.textContent = `${filtered.length} / ${components.length} components ≤ ${limit} nodes • detached: ${detachedSet.size} • isolates: ${components.filter(c => c.size <= 2 || c.isolated_count > 0).length}`;
      tableBody.querySelectorAll('[data-role="component-focus"]').forEach(btn => {
        btn.addEventListener('click', (evt) => {
          const compId = parseInt(evt.currentTarget.getAttribute('data-comp'), 10);
          if (!Number.isFinite(compId)) return;
          componentSelect.value = `comp-${compId}`;
          state.viewComponents = new Set([compId]);
          state.highlightComponents = new Set([compId]);
          applyFilters();
          const nodes = cy.nodes().filter(n => n.data('component') === compId);
          if (nodes.length) cy.fit(nodes, 30);
        });
      });
    };

    const showIsolatesBtn = componentBar.querySelector('[data-role="component-show-isolates"]');
    const highlightBtn = componentBar.querySelector('[data-role="component-highlight"]');
    const dimBtn = componentBar.querySelector('[data-role="component-dim"]');
    const copyBtn = componentBar.querySelector('[data-role="component-copy"]');
    const exportJsonBtn = componentBar.querySelector('[data-role="component-export-json"]');
    const exportCsvBtn = componentBar.querySelector('[data-role="component-export-csv"]');

    const gatherNodesForExport = () => {
      const target = gatherSelectedComponents();
      const nodes = target.size
        ? cy.nodes().filter(n => target.has(n.data('component')))
        : cy.nodes();
      return nodes.map(n => n.data());
    };

    if (showIsolatesBtn) showIsolatesBtn.addEventListener('click', () => {
      componentSelect.value = 'isolates';
      updateComponentFilter();
    });
    if (componentSelect) componentSelect.addEventListener('change', updateComponentFilter);
    if (sizeSlider) sizeSlider.addEventListener('input', (e) => {
      syncSize(parseInt(e.target.value, 10));
      if (componentSelect.value === 'size') updateComponentFilter();
      renderComponentTable();
    });
    if (sizeLimitInput) sizeLimitInput.addEventListener('input', (e) => {
      syncSize(parseInt(e.target.value, 10));
      if (componentSelect.value === 'size') updateComponentFilter();
      renderComponentTable();
    });
    if (componentReset) componentReset.addEventListener('click', () => {
      componentSelect.value = 'all';
      state.viewComponents = new Set();
      state.highlightComponents = new Set();
      applyFilters();
    });
    if (highlightBtn) highlightBtn.addEventListener('click', () => {
      state.dimOthers = false;
      applyHighlight(false);
      const comps = gatherSelectedComponents();
      if (comps.size) {
        const nodes = cy.nodes().filter(n => comps.has(n.data('component')));
        if (nodes.length) cy.fit(nodes, 30);
      }
    });
    if (dimBtn) dimBtn.addEventListener('click', () => {
      state.dimOthers = true;
      applyHighlight(true);
    });
    if (copyBtn) copyBtn.addEventListener('click', () => {
      const nodes = gatherNodesForExport();
      const lines = nodes.map(n => `${n.id || ''}, loc=${n.loc || 0}, degree=${n.degree || 0}, comp=C${n.component || '?'}`);
      navigator.clipboard.writeText(lines.join('\n'));
    });
    if (exportJsonBtn) exportJsonBtn.addEventListener('click', () => {
      const nodes = gatherNodesForExport();
      download(`${g.id}-component.json`, JSON.stringify(nodes, null, 2), 'application/json');
    });
    if (exportCsvBtn) exportCsvBtn.addEventListener('click', () => {
      const nodes = gatherNodesForExport();
      const header = 'path,loc,degree,component';
      const rows = nodes.map(n => `${n.id || ''},${n.loc || 0},${n.degree || 0},C${n.component || ''}`);
      download(`${g.id}-component.csv`, [header, ...rows].join('\n'), 'text/csv');
    });

    toolbar.querySelectorAll('input').forEach(inp => {
      inp.addEventListener('input', () => applyFilters());
      inp.addEventListener('change', () => applyFilters());
    });

    renderComponentTable();
    applyFilters();
  });
})();
</script>"#,
    );

    out.push_str("</body></html>");
    fs::write(path, out)
}

#[cfg(test)]
mod tests {
    use super::render_html_report;
    use crate::analyzer::report::{AiInsight, RankedDup, ReportSection};
    use std::fs;

    #[test]
    fn renders_basic_report() {
        let tmp_dir = std::env::temp_dir().join("loctree_html_test");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();
        let out_path = tmp_dir.join("report.html");

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
            insights: vec![AiInsight {
                title: "Hint".into(),
                severity: "medium".into(),
                message: "Message".into(),
            }],
        };

        render_html_report(&out_path, &[section]).unwrap();
        let html = fs::read_to_string(&out_path).unwrap();
        assert!(html.contains("loctree import/export analysis"));
        assert!(html.contains("Hint"));
        assert!(html.contains("Foo"));
    }
}
