(function () {
  const graphs = window.__LOCTREE_GRAPHS || [];
  const escapeHtml = (value = "") =>
    String(value).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
  const formatNum = (n) => (typeof n === "number" && n.toLocaleString ? n.toLocaleString() : n || 0);
  const cyInstances = new Set();
  const darkToggles = new Set();
  const filterElements = (elements, opts) => {
    const text = (opts.text || "").toLowerCase();
    const minDeg = parseInt(opts.minDeg || "0", 10) || 0;
    const allowedComponents = opts.allowedComponents || new Set();
    let nodes = elements.nodes.map((n) => ({ data: { ...n.data }, position: { ...n.position } }));
    if (text) nodes = nodes.filter((n) => (n.data.id || "").toLowerCase().includes(text));
    if (allowedComponents.size) nodes = nodes.filter((n) => allowedComponents.has(n.data.component));

    let edges = elements.edges.map((e) => ({ data: { ...e.data } }));
    const nodeSet = new Set(nodes.map((n) => n.data.id));
    edges = edges.filter((e) => nodeSet.has(e.data.source) && nodeSet.has(e.data.target));

    if (minDeg > 0) {
      const deg = {};
      edges.forEach((e) => {
        deg[e.data.source] = (deg[e.data.source] || 0) + 1;
        deg[e.data.target] = (deg[e.data.target] || 0) + 1;
      });
      nodes = nodes.filter((n) => (deg[n.data.id] || 0) >= minDeg);
      const filteredSet = new Set(nodes.map((n) => n.data.id));
      edges = edges.filter((e) => filteredSet.has(e.data.source) && filteredSet.has(e.data.target));
    }
    return { nodes, edges };
  };

  const applyDarkTheme = (on, graphs) => {
    document.documentElement.classList.toggle("dark", on);
    graphs
      .filter(Boolean)
      .forEach((inst) => {
        if (inst && typeof inst.style === "function") {
          const style = inst.style();
          style.selector("node").style("color", on ? "#eef2ff" : "#fff").update();
          style.selector("edge").style("text-background-color", on ? "#0f1115" : "#fff").update();
        }
      });
  };
  const setDarkMode = (on) => applyDarkTheme(on, Array.from(cyInstances));
  const applyDarkShared = (on) => {
    darkToggles.forEach((chk) => {
      if (chk) chk.checked = on;
    });
    setDarkMode(on);
  };

  graphs.forEach((g) => {
    const container = document.getElementById(g.id);
    if (!container || container.dataset.enhanced === "1") return;
    container.dataset.enhanced = "1";

    const components = Array.isArray(g.components) ? g.components : [];
    const componentMap = new Map();
    components.forEach((c) => componentMap.set(c.id, c));
    const detachedSet = new Set(components.filter((c) => c.detached).map((c) => c.id));
    const openBase = g.openBase || null;

    const originalParent = container.parentNode;
    container.style.height = "520px";

    // Component controls stay in the main Graph tab
    const targetParent = originalParent || container.parentNode;
    const componentBar = document.createElement("div");
    componentBar.className = "graph-toolbar component-toolbar";
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
    if (targetParent) targetParent.insertBefore(componentBar, container);

    // Graph controls
    const toolbar = document.createElement("div");
    toolbar.className = "graph-toolbar";
    toolbar.innerHTML = `
      <label>filter:
        <input type="text" size="18" placeholder="substring (e.g. features/ai-suite, .tsx)" data-role="filter-text" />
      </label>
      <label>min degree:
        <input type="number" min="0" value="0" style="width:60px" data-role="min-degree" />
      </label>
      <label>layout:
        <select data-role="layout-select">
          <option value="cose" selected>cose (force)</option>
          <option value="dagre">dagre (hierarchy)</option>
          <option value="cose-bilkent">cose-bilkent</option>
          <option value="concentric">concentric</option>
          <option value="breadthfirst">breadthfirst</option>
          <option value="preset">preset (original)</option>
        </select>
      </label>
      <label><input type="checkbox" data-role="toggle-labels" checked /> labels</label>
      <span class="graph-controls">
        <button data-role="fit">fit</button>
        <button data-role="relayout">relayout</button>
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
    if (targetParent) targetParent.insertBefore(toolbar, componentBar.nextSibling);

    const componentPanel = document.createElement("div");
    componentPanel.className = "component-panel";
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
    if (targetParent) targetParent.insertBefore(componentPanel, toolbar.nextSibling);

    const hint = document.createElement("div");
    hint.className = "graph-hint";
    hint.textContent =
      "Component filter selects an island (or small islands) and highlights nodes; slider sets threshold for small components. Text filter still matches node paths.";
    if (targetParent) targetParent.insertBefore(hint, componentPanel.nextSibling);

    const componentSelect = componentBar.querySelector('[data-role="component-filter"]');
    const sizeSlider = componentBar.querySelector('[data-role="component-threshold"]');
    const sizeLabel = componentBar.querySelector('[data-role="component-threshold-label"]');
    const tableBody = componentPanel.querySelector('[data-role="component-table"]');
    const summaryEl = componentPanel.querySelector('[data-role="component-summary"]');
    const sizeLimitInput = componentPanel.querySelector('[data-role="component-size-limit"]');
    const componentReset = componentPanel.querySelector('[data-role="component-reset"]');

    const addComponentOptions = () => {
      const sorted = [...components].sort((a, b) => a.size - b.size || a.id - b.id);
      sorted.forEach((comp) => {
        const opt = document.createElement("option");
        opt.value = `comp-${comp.id}`;
        const labelSample = comp.sample || (Array.isArray(comp.nodes) && comp.nodes[0]) || "";
        opt.textContent = `C${comp.id} • ${comp.size} nodes • ${labelSample}`;
        opt.dataset.size = comp.size;
        componentSelect.appendChild(opt);
      });
    };
    addComponentOptions();

    const state = {
      viewComponents: new Set(),
      highlightComponents: new Set(),
      sizeThreshold: parseInt(sizeSlider?.value || "8", 10) || 8,
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

    // Layout configuration helper - supports multiple algorithms
    const getLayoutConfig = (name, nodeCount) => {
      const animate = nodeCount < 300;
      const configs = {
        cose: {
          name: "cose",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          nodeRepulsion: function(node) { return 8000; },
          idealEdgeLength: function(edge) { return 100; },
          edgeElasticity: function(edge) { return 100; },
          nestingFactor: 1.2,
          gravity: 1,
          numIter: 1000,
          initialTemp: 1000,
          coolingFactor: 0.99,
          minTemp: 1.0,
          randomize: false,
        },
        "cose-bilkent": {
          name: "cose-bilkent",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          nodeRepulsion: 4500,
          idealEdgeLength: 80,
          edgeElasticity: 0.45,
          nestingFactor: 0.1,
          gravity: 0.25,
          numIter: 2500,
          tile: true,
          tilingPaddingVertical: 10,
          tilingPaddingHorizontal: 10,
          gravityRangeCompound: 1.5,
          gravityCompound: 1.0,
          gravityRange: 3.8,
          randomize: true,
        },
        dagre: {
          name: "dagre",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          rankDir: "TB",  // top-to-bottom (hierarchy: caller → callee)
          nodeSep: 50,
          rankSep: 80,
          edgeSep: 10,
          ranker: "network-simplex",  // tight-tree, longest-path, network-simplex
        },
        concentric: {
          name: "concentric",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          minNodeSpacing: 50,
          concentric: function(node) { return node.data("degree") || 0; },
          levelWidth: function(nodes) { return Math.max(1, Math.ceil(nodes.length / 8)); },
          clockwise: true,
          startAngle: 3 / 2 * Math.PI,
        },
        breadthfirst: {
          name: "breadthfirst",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          directed: true,
          spacingFactor: 1.5,
          circle: false,
          grid: false,
          avoidOverlap: true,
        },
        preset: {
          name: "preset",
          animate: false,
          fit: true,
        },
      };
      return configs[name] || configs.preset;
    };

    const buildElements = () => {
      const rawNodes = Array.isArray(g.nodes) ? g.nodes : [];
      const rawEdges = Array.isArray(g.edges) ? g.edges : [];
      const nodeToComponent = new Map();
      const nodes = rawNodes.map((n) => {
        const size = Math.max(4, Math.min(30, Math.sqrt((n && n.loc) || 1)));
        const comp = n.component || 0;
        const compSize = (componentMap.get(comp) || {}).size || 0;
        const detached = detachedSet.has(comp) || !!n.detached;
        const isolate = (n.degree || 0) === 0 || compSize <= 2;
        const id = n.id || "";
        nodeToComponent.set(id, comp);
        return {
          data: {
            id,
            label: n.label || id || "",
            loc: n.loc || 0,
            size,
            full: id || "",
            component: comp,
            degree: n.degree || 0,
            detached,
            componentSize: compSize,
            isolate: isolate ? 1 : 0,
            color: detached ? "#d1830f" : "#4f81e1",
          },
          position: { x: n.x || 0, y: n.y || 0 },
        };
      });
      const edges = rawEdges.map((e, idx) => {
        const kind = (e && e[2]) || "import";
        const sourceComp = nodeToComponent.get(e[0]) || nodeToComponent.get(e[1]) || 0;
        const detached = detachedSet.has(sourceComp);
        const color = detached ? "#d1830f" : kind === "reexport" ? "#e67e22" : "#888";
        return {
          data: {
            id: "e" + idx,
            source: e[0],
            target: e[1],
            label: kind,
            kind,
            color,
            component: sourceComp,
            detached: detached ? 1 : 0,
          },
        };
      });
      return { nodes, edges };
    };

    const original = buildElements();
    const emptyOverlay = document.createElement("div");
    emptyOverlay.className = "graph-empty";
    emptyOverlay.style.display = "none";
    container.appendChild(emptyOverlay);
    let cy = cytoscape({
      container,
      elements: original,
      style: [
        { selector: "node", style: { label: "data(label)", "font-size": 10, "text-wrap": "wrap", "text-max-width": 120, "background-color": "data(color)", color: "#fff", width: "data(size)", height: "data(size)", "overlay-padding": 8, "overlay-opacity": 0 } },
        { selector: "node.detached", style: { "background-color": "#d1830f" } },
        { selector: "node.isolate", style: { "border-width": 2, "border-color": "#d74d26" } },
        { selector: "node.highlight", style: { "border-width": 3, "border-color": "#111", "shadow-blur": 12, "shadow-color": "#111", "shadow-opacity": 0.45, "shadow-offset-x": 0, "shadow-offset-y": 0, "z-index": 999 } },
        { selector: "node.dimmed", style: { opacity: 0.15 } },
        { selector: "edge", style: { "curve-style": "bezier", width: 1.1, "line-color": "data(color)", "target-arrow-color": "data(color)", "target-arrow-shape": "triangle", "arrow-scale": 0.7, label: "", "font-size": 9, "text-background-color": "#fff", "text-background-opacity": 0.8, "text-background-padding": 2 } },
        { selector: "edge.detached", style: { "line-color": "#d1830f", "target-arrow-color": "#d1830f" } },
        { selector: "edge.highlight", style: { width: 2, opacity: 0.9 } },
        { selector: "edge.dimmed", style: { opacity: 0.08 } },
      ],
      layout: { name: "preset", animate: false, fit: true },
    });
    cyInstances.add(cy);

    const download = (filename, content, type) => {
      const blob = new Blob([content], { type });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
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

      cy.nodes().removeClass("dimmed highlight isolate detached");
      cy.edges().removeClass("dimmed highlight detached");

      cy.nodes().filter((n) => n.data("detached")).addClass("detached");
      cy.edges().filter((e) => e.data("detached")).addClass("detached");
      cy.nodes()
        .filter((n) => (n.data("isolate") || 0) === 1 || (n.data("componentSize") || 0) <= 2)
        .addClass("isolate");

      if (!highlightSet.size) return;
      const nodes = cy.nodes().filter((n) => highlightSet.has(n.data("component")));
      const edges = cy.edges().filter((e) => highlightSet.has(e.data("component")));
      nodes.addClass("highlight");
      edges.addClass("highlight");
      if (dim) {
        cy.nodes().not(nodes).addClass("dimmed");
        cy.edges().not(edges).addClass("dimmed");
      }
    };

    const layoutSelect = toolbar.querySelector('[data-role="layout-select"]');
    const getSelectedLayout = () => layoutSelect?.value || "cose";

    const applyFilters = (runLayout = true) => {
      const text = (toolbar.querySelector('[data-role="filter-text"]')?.value || "").toLowerCase();
      const minDeg = parseInt(toolbar.querySelector('[data-role="min-degree"]')?.value || "0", 10) || 0;
      const allowedComponents = state.viewComponents;
      const filtered = filterElements(original, { text, minDeg, allowedComponents });
      let nodes = filtered.nodes;
      let edges = filtered.edges;

      if (nodes.length === 0) {
        emptyOverlay.style.display = "block";
        cy.elements().remove();
        return;
      }
      emptyOverlay.style.display = "none";

      cy.elements().remove();
      cy.add({ nodes, edges });

      const showLabels = toolbar.querySelector('[data-role="toggle-labels"]').checked;
      const autoHide = nodes.length > 800;
      const labelsOn = showLabels && !autoHide;
      cy.style().selector("node").style("label", labelsOn ? "data(label)" : "").update();

      if (runLayout) {
        const layoutName = getSelectedLayout();
        const layoutConfig = getLayoutConfig(layoutName, nodes.length);
        cy.layout(layoutConfig).run();
      }
      applyHighlight();
    };

    const runRelayout = () => {
      const layoutName = getSelectedLayout();
      const nodeCount = cy.nodes().length;
      const layoutConfig = getLayoutConfig(layoutName, nodeCount);
      cy.layout(layoutConfig).run();
    };

    // Fit / reset / relayout / dark / fullscreen
    const fitBtn = toolbar.querySelector('[data-role="fit"]');
    const relayoutBtn = toolbar.querySelector('[data-role="relayout"]');
    const resetBtn = toolbar.querySelector('[data-role="reset"]');
    const darkChk = toolbar.querySelector('[data-role="dark"]');
    const fsBtn = toolbar.querySelector('[data-role="fullscreen"]');
    const pngBtn = toolbar.querySelector('[data-role="png"]');
    const jsonBtn = toolbar.querySelector('[data-role="json"]');

    if (fitBtn) fitBtn.addEventListener("click", () => cy.fit());
    if (relayoutBtn) relayoutBtn.addEventListener("click", runRelayout);
    if (layoutSelect) layoutSelect.addEventListener("change", runRelayout);
    if (resetBtn)
      resetBtn.addEventListener("click", () => {
        cy.elements().remove();
        cy.add(original);
        state.viewComponents = new Set();
        state.highlightComponents = new Set();
        layoutSelect.value = "preset";
        applyFilters(false);
        cy.layout({ name: "preset", animate: false, fit: true }).run();
      });

    if (pngBtn)
      pngBtn.addEventListener("click", () => {
        const dark = darkChk && darkChk.checked;
        const dataUrl = cy.png({ bg: dark ? "#0f1115" : "#ffffff", full: true, scale: 2 });
        const a = document.createElement("a");
        a.href = dataUrl;
        a.download = `${g.id}-graph.png`;
        document.body.appendChild(a);
        a.click();
        a.remove();
      });

    if (jsonBtn)
      jsonBtn.addEventListener("click", () => {
        const payload = {
          nodes: cy.nodes().map((n) => n.data()),
          edges: cy.edges().map((e) => ({ source: e.data("source"), target: e.data("target"), kind: e.data("kind") })),
          filter: toolbar.querySelector('[data-role="filter-text"]')?.value || "",
          minDegree: parseInt(toolbar.querySelector('[data-role="min-degree"]')?.value || "0", 10) || 0,
          components,
          highlightedComponents: Array.from(state.highlightComponents),
          viewedComponents: Array.from(state.viewComponents),
        };
        download(`${g.id}-graph.json`, JSON.stringify(payload, null, 2), "application/json");
      });

    if (darkChk) {
      darkToggles.add(darkChk);
      darkChk.addEventListener("change", () => applyDarkShared(darkChk.checked));
    }

    const fsTarget = container;
    if (fsBtn && fsTarget && fsTarget.requestFullscreen) {
      fsBtn.addEventListener("click", () => {
        if (document.fullscreenElement) {
          document.exitFullscreen();
        } else {
          fsTarget.requestFullscreen().catch(() => {});
        }
      });
      document.addEventListener("fullscreenchange", () => {
        fsBtn.textContent = document.fullscreenElement ? "exit fullscreen" : "fullscreen";
        if (!document.fullscreenElement) cy.fit();
      });
    }

    // Tooltip on hover/click (sticky behavior)
    const tooltip = document.createElement("div");
    tooltip.style.position = "fixed";
    tooltip.style.pointerEvents = "auto";
    tooltip.style.background = "#111";
    tooltip.style.color = "#fff";
    tooltip.style.padding = "6px 8px";
    tooltip.style.borderRadius = "6px";
    tooltip.style.fontSize = "12px";
    tooltip.style.display = "none";
    tooltip.style.zIndex = 9999;
    document.body.appendChild(tooltip);

    let nodeHover = false;
    let tooltipHover = false;
    let hideTimeout = null;

    const hideTip = () => {
      if (hideTimeout) {
        clearTimeout(hideTimeout);
        hideTimeout = null;
      }
      nodeHover = false;
      tooltipHover = false;
      tooltip.style.display = "none";
    };

    const scheduleHide = () => {
      if (hideTimeout) {
        clearTimeout(hideTimeout);
      }
      hideTimeout = setTimeout(() => {
        if (!nodeHover && !tooltipHover) {
          hideTip();
        }
      }, 220);
    };

    const showTip = (evt, node) => {
      const data = node.data();
      const path = data.full || data.id;
      const comp = componentMap.get(data.component);
      const compLabel = comp ? `C${comp.id} (${comp.size} nodes${comp.detached ? ", detached" : ""})` : "—";
      // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: path escaped via escapeHtml(); other values are numbers
      tooltip.innerHTML = `
        <div style="margin-bottom:4px"><strong>${escapeHtml(path)}</strong></div>
        <div>LOC: ${data.loc || 0} | degree: ${data.degree || 0}</div>
        <div>component: ${compLabel}</div>
        <button data-role="copy-path" style="margin-top:4px;font-size:10px;cursor:pointer">copy path</button>
      `;
      const copyBtn = tooltip.querySelector('[data-role="copy-path"]');
      if (copyBtn)
        copyBtn.addEventListener("click", () => navigator.clipboard.writeText(path));
      const rect = container.getBoundingClientRect();
      const scrollX = window.scrollX || document.documentElement.scrollLeft || 0;
      const scrollY = window.scrollY || document.documentElement.scrollTop || 0;
      let left = rect.left + evt.renderedPosition.x + 12 + scrollX;
      let top = rect.top + evt.renderedPosition.y + 12 + scrollY;
      const maxLeft = scrollX + window.innerWidth - 220;
      if (left > maxLeft) left = maxLeft;
      tooltip.style.left = left + "px";
      tooltip.style.top = top + "px";
      tooltip.style.display = "block";
      nodeHover = true;
      if (hideTimeout) {
        clearTimeout(hideTimeout);
        hideTimeout = null;
      }
    };

    tooltip.addEventListener("mouseenter", () => {
      tooltipHover = true;
      if (hideTimeout) {
        clearTimeout(hideTimeout);
        hideTimeout = null;
      }
    });
    tooltip.addEventListener("mouseleave", () => {
      tooltipHover = false;
      scheduleHide();
    });

    cy.off("mouseover");
    cy.off("mouseout");
    cy.off("tap");
    cy.off("tapdrag");
    cy.on("mouseover", "node", (evt) => {
      nodeHover = true;
      showTip(evt, evt.target);
    });
    cy.on("mouseout", "node", () => {
      nodeHover = false;
      scheduleHide();
    });
    cy.on("tap", "node", (evt) => {
      nodeHover = true;
      showTip(evt, evt.target);
    });
    cy.on("tapdrag", "node", () => {
      nodeHover = false;
      hideTip();
    });

    const updateComponentFilter = () => {
      const val = componentSelect.value;
      const set = new Set();
      if (val === "isolates") {
        components.filter((c) => c.size <= 2 || c.isolated_count > 0).forEach((c) => set.add(c.id));
      } else if (val === "size") {
        components.filter((c) => c.size <= state.sizeThreshold).forEach((c) => set.add(c.id));
      } else if (val.startsWith("comp-")) {
        const id = parseInt(val.replace("comp-", ""), 10);
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
      const filtered = rows.filter((c) => c.size <= limit);
      tableBody.innerHTML = "";
      filtered.forEach((comp) => {
        const sample = comp.sample || (comp.nodes && comp.nodes[0]) || "";
        const sampleHref = openBase ? `${openBase}/open?f=${encodeURIComponent(sample)}&l=1` : null;
        const sampleCell = sampleHref ? `<a href="${sampleHref}">${escapeHtml(sample)}</a>` : `<code>${escapeHtml(sample)}</code>`;
        const warn = comp.detached ? " ⚠️" : "";
        const tr = document.createElement("tr");
        const edgeCount = comp.edges !== undefined ? comp.edges : comp.edge_count;
        // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: all values are numbers or escaped via escapeHtml()
        tr.innerHTML = `<td>C${comp.id}${warn}</td><td>${comp.size}</td><td>${sampleCell}</td><td>${comp.isolated_count}</td><td>${edgeCount || 0}</td><td>${formatNum(comp.loc_sum)}</td><td><button data-role="component-focus" data-comp="${comp.id}">Highlight</button></td>`;
        tableBody.appendChild(tr);
      });
      summaryEl.textContent = `${filtered.length} / ${components.length} components ≤ ${limit} nodes • detached: ${detachedSet.size} • isolates: ${
        components.filter((c) => c.size <= 2 || c.isolated_count > 0).length
      }`;
      tableBody.querySelectorAll('[data-role="component-focus"]').forEach((btn) => {
        btn.addEventListener("click", (evt) => {
          const compId = parseInt(evt.currentTarget.getAttribute("data-comp"), 10);
          if (!Number.isFinite(compId)) return;
          componentSelect.value = `comp-${compId}`;
          state.viewComponents = new Set([compId]);
          state.highlightComponents = new Set([compId]);
          applyFilters();
          const nodes = cy.nodes().filter((n) => n.data("component") === compId);
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
      const nodes = target.size ? cy.nodes().filter((n) => target.has(n.data("component"))) : cy.nodes();
      return nodes.map((n) => n.data());
    };

    if (showIsolatesBtn) showIsolatesBtn.addEventListener("click", () => {
      componentSelect.value = "isolates";
      updateComponentFilter();
    });
    if (componentSelect) componentSelect.addEventListener("change", updateComponentFilter);
    if (sizeSlider)
      sizeSlider.addEventListener("input", (e) => {
        syncSize(parseInt(e.target.value, 10));
        if (componentSelect.value === "size") updateComponentFilter();
        renderComponentTable();
      });
    if (sizeLimitInput)
      sizeLimitInput.addEventListener("input", (e) => {
        syncSize(parseInt(e.target.value, 10));
        if (componentSelect.value === "size") updateComponentFilter();
        renderComponentTable();
      });
    if (componentReset)
      componentReset.addEventListener("click", () => {
        componentSelect.value = "all";
        state.viewComponents = new Set();
        state.highlightComponents = new Set();
        applyFilters();
      });
    if (highlightBtn)
      highlightBtn.addEventListener("click", () => {
        state.dimOthers = false;
        applyHighlight(false);
        const comps = gatherSelectedComponents();
        if (comps.size) {
          const nodes = cy.nodes().filter((n) => comps.has(n.data("component")));
          if (nodes.length) cy.fit(nodes, 30);
        }
      });
    if (dimBtn) dimBtn.addEventListener("click", () => {
      state.dimOthers = true;
      applyHighlight(true);
    });
    if (copyBtn)
      copyBtn.addEventListener("click", () => {
        const nodes = gatherNodesForExport();
        const lines = nodes.map((n) => `${n.id || ""}, loc=${n.loc || 0}, degree=${n.degree || 0}, comp=C${n.component || "?"}`);
        navigator.clipboard.writeText(lines.join("\n"));
      });
    if (exportJsonBtn)
      exportJsonBtn.addEventListener("click", () => {
        const nodes = gatherNodesForExport();
        download(`${g.id}-component.json`, JSON.stringify(nodes, null, 2), "application/json");
      });
    if (exportCsvBtn)
      exportCsvBtn.addEventListener("click", () => {
        const nodes = gatherNodesForExport();
        const header = "path,loc,degree,component";
        const rows = nodes.map((n) => `${n.id || ""},${n.loc || 0},${n.degree || 0},C${n.component || ""}`);
        download(`${g.id}-component.csv`, [header, ...rows].join("\n"), "text/csv");
      });

    toolbar.querySelectorAll("input").forEach((inp) => {
      inp.addEventListener("input", () => applyFilters());
      inp.addEventListener("change", () => applyFilters());
    });

    renderComponentTable();
    applyFilters();
  });

  // Single shared drawer for all graphs (one instance, switchable per root)
  if (graphs.length && !document.querySelector(".graph-drawer")) {
    const graphMap = new Map(graphs.map((g) => [g.id, g]));
    let cyDrawer = null;
    let drawerDarkChk = null;
    const drawer = document.createElement("div");
    drawer.className = "graph-drawer";
    const header = document.createElement("div");
    header.className = "graph-drawer-header";
    const toggleBtn = document.createElement("button");
    toggleBtn.dataset.role = "drawer-toggle";
    toggleBtn.textContent = "hide graph";
    const title = document.createElement("span");
    title.textContent = "Import graph";
    header.appendChild(toggleBtn);
    header.appendChild(title);

    const select = document.createElement("select");
    select.style.minWidth = "160px";
    graphs.forEach((g) => {
      const opt = document.createElement("option");
      opt.value = g.id;
      opt.textContent = g.label || g.id;
      select.appendChild(opt);
    });
    if (graphs.length > 1) {
      const selectWrap = document.createElement("span");
      selectWrap.style.display = "flex";
      selectWrap.style.gap = "6px";
      selectWrap.style.alignItems = "center";
      const lbl = document.createElement("span");
      lbl.textContent = "root:";
      selectWrap.appendChild(lbl);
      selectWrap.appendChild(select);
      header.appendChild(selectWrap);
    }

    const drawerBody = document.createElement("div");
    drawerBody.className = "graph-drawer-body";
    const drawerToolbar = document.createElement("div");
    drawerToolbar.className = "graph-toolbar";
    drawerToolbar.innerHTML = `
      <label>filter: <input type="text" size="16" data-role="drawer-filter" placeholder="substring" /></label>
      <label>min degree: <input type="number" min="0" value="0" style="width:60px" data-role="drawer-min-degree" /></label>
      <label>layout:
        <select data-role="drawer-layout-select">
          <option value="cose" selected>cose (force)</option>
          <option value="dagre">dagre (hierarchy)</option>
          <option value="cose-bilkent">cose-bilkent</option>
          <option value="concentric">concentric</option>
          <option value="breadthfirst">breadthfirst</option>
          <option value="preset">preset (original)</option>
        </select>
      </label>
      <label><input type="checkbox" data-role="drawer-labels" checked /> labels</label>
      <span class="graph-controls">
        <button data-role="drawer-fit">fit</button>
        <button data-role="drawer-relayout">relayout</button>
        <button data-role="drawer-reset">reset</button>
        <label><input type="checkbox" data-role="drawer-dark" /> dark</label>
        <button data-role="drawer-fullscreen">fullscreen</button>
        <button data-role="drawer-png">png</button>
      </span>
    `;
    const drawerCanvas = document.createElement("div");
    drawerCanvas.className = "graph";
    drawerCanvas.style.height = "420px";
    drawerBody.appendChild(drawerToolbar);
    drawerBody.appendChild(drawerCanvas);
    drawer.appendChild(header);
    drawer.appendChild(drawerBody);
    document.body.appendChild(drawer);

    let collapsed = false;
    const applyCollapse = () => {
      drawer.classList.toggle("collapsed", collapsed);
      drawerBody.style.display = collapsed ? "none" : "block";
      toggleBtn.textContent = collapsed ? "show graph" : "hide graph";
    };
    header.addEventListener("click", (evt) => {
      // avoid treating select change as toggle
      if (evt.target === select) return;
      collapsed = !collapsed;
      applyCollapse();
    });
    applyCollapse();

    const drawerFilter = drawerToolbar.querySelector('[data-role="drawer-filter"]');
    const drawerMinDeg = drawerToolbar.querySelector('[data-role="drawer-min-degree"]');
    const drawerLayoutSelect = drawerToolbar.querySelector('[data-role="drawer-layout-select"]');
    const drawerLabels = drawerToolbar.querySelector('[data-role="drawer-labels"]');
    drawerDarkChk = drawerToolbar.querySelector('[data-role="drawer-dark"]');
    const drawerFit = drawerToolbar.querySelector('[data-role="drawer-fit"]');
    const drawerRelayout = drawerToolbar.querySelector('[data-role="drawer-relayout"]');
    const drawerReset = drawerToolbar.querySelector('[data-role="drawer-reset"]');
    const drawerFs = drawerToolbar.querySelector('[data-role="drawer-fullscreen"]');
    const drawerPng = drawerToolbar.querySelector('[data-role="drawer-png"]');

    const getDrawerSelectedLayout = () => drawerLayoutSelect?.value || "cose";

    // Drawer layout configuration helper (same as main graph)
    const getDrawerLayoutConfig = (name, nodeCount) => {
      const animate = nodeCount < 300;
      const configs = {
        cose: {
          name: "cose",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          nodeRepulsion: function(node) { return 8000; },
          idealEdgeLength: function(edge) { return 100; },
          edgeElasticity: function(edge) { return 100; },
          nestingFactor: 1.2,
          gravity: 1,
          numIter: 1000,
          initialTemp: 1000,
          coolingFactor: 0.99,
          minTemp: 1.0,
          randomize: false,
        },
        "cose-bilkent": {
          name: "cose-bilkent",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          nodeRepulsion: 4500,
          idealEdgeLength: 80,
          edgeElasticity: 0.45,
          nestingFactor: 0.1,
          gravity: 0.25,
          numIter: 2500,
          tile: true,
          tilingPaddingVertical: 10,
          tilingPaddingHorizontal: 10,
          gravityRangeCompound: 1.5,
          gravityCompound: 1.0,
          gravityRange: 3.8,
          randomize: true,
        },
        dagre: {
          name: "dagre",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          rankDir: "TB",
          nodeSep: 50,
          rankSep: 80,
          edgeSep: 10,
          ranker: "network-simplex",
        },
        concentric: {
          name: "concentric",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          minNodeSpacing: 50,
          concentric: function(node) { return node.data("degree") || 0; },
          levelWidth: function(nodes) { return Math.max(1, Math.ceil(nodes.length / 8)); },
          clockwise: true,
          startAngle: 3 / 2 * Math.PI,
        },
        breadthfirst: {
          name: "breadthfirst",
          animate,
          animationDuration: animate ? 500 : 0,
          fit: true,
          padding: 30,
          directed: true,
          spacingFactor: 1.5,
          circle: false,
          grid: false,
          avoidOverlap: true,
        },
        preset: {
          name: "preset",
          animate: false,
          fit: true,
        },
      };
      return configs[name] || configs.preset;
    };

    const buildElements = (graph) => {
      const comps = Array.isArray(graph.components) ? graph.components : [];
      const compMap = new Map(comps.map((c) => [c.id, c]));
      const detached = new Set(comps.filter((c) => c.detached).map((c) => c.id));
      const rawNodes = Array.isArray(graph.nodes) ? graph.nodes : [];
      const rawEdges = Array.isArray(graph.edges) ? graph.edges : [];
      const nodes = rawNodes.map((n) => {
        const size = Math.max(4, Math.min(30, Math.sqrt((n && n.loc) || 1)));
        const comp = n.component || 0;
        const compSize = (compMap.get(comp) || {}).size || 0;
        const isDetached = detached.has(comp) || !!n.detached;
        const isolate = (n.degree || 0) === 0 || compSize <= 2;
        const id = n.id || "";
        return {
          data: {
            id,
            label: n.label || id || "",
            loc: n.loc || 0,
            size,
            full: id || "",
            component: comp,
            degree: n.degree || 0,
            detached: isDetached,
            componentSize: compSize,
            isolate: isolate ? 1 : 0,
            color: isDetached ? "#d1830f" : "#4f81e1",
          },
          position: { x: n.x || 0, y: n.y || 0 },
        };
      });
      const nodeToComponent = new Map(nodes.map((n) => [n.data.id, n.data.component]));
      const edges = rawEdges.map((e, idx) => {
        const kind = (e && e[2]) || "import";
        const sourceComp = nodeToComponent.get(e[0]) || nodeToComponent.get(e[1]) || 0;
        const isDetached = detached.has(sourceComp);
        const color = isDetached ? "#d1830f" : kind === "reexport" ? "#e67e22" : "#888";
        return {
          data: {
            id: "e" + idx,
            source: e[0],
            target: e[1],
            label: kind,
            kind,
            color,
            component: sourceComp,
            detached: isDetached ? 1 : 0,
          },
        };
      });
      return { nodes, edges };
    };

    let activeGraph = graphs[0];
    let drawerElements = buildElements(activeGraph);
    cyDrawer = cytoscape({
      container: drawerCanvas,
      elements: drawerElements,
      style: [
        { selector: "node", style: { label: "data(label)", "font-size": 10, "text-wrap": "wrap", "text-max-width": 120, "background-color": "data(color)", color: "#fff", width: "data(size)", height: "data(size)", "overlay-padding": 8, "overlay-opacity": 0 } },
        { selector: "edge", style: { "curve-style": "bezier", width: 1.1, "line-color": "data(color)", "target-arrow-color": "data(color)", "target-arrow-shape": "triangle", "arrow-scale": 0.7, label: "", "font-size": 9, "text-background-color": "#fff", "text-background-opacity": 0.8, "text-background-padding": 2 } },
      ],
      layout: { name: "preset", animate: false, fit: true },
    });
    cyInstances.add(cyDrawer);

    // Drawer tooltip (sticky behavior)
    const drawerTooltip = document.createElement("div");
    drawerTooltip.style.position = "fixed";
    drawerTooltip.style.pointerEvents = "auto";
    drawerTooltip.style.background = "#111";
    drawerTooltip.style.color = "#fff";
    drawerTooltip.style.padding = "6px 8px";
    drawerTooltip.style.borderRadius = "6px";
    drawerTooltip.style.fontSize = "12px";
    drawerTooltip.style.display = "none";
    drawerTooltip.style.zIndex = 9999;
    document.body.appendChild(drawerTooltip);

    let drawerNodeHover = false;
    let drawerTooltipHover = false;
    let drawerHideTimeout = null;

    const hideDrawerTip = () => {
      if (drawerHideTimeout) {
        clearTimeout(drawerHideTimeout);
        drawerHideTimeout = null;
      }
      drawerNodeHover = false;
      drawerTooltipHover = false;
      drawerTooltip.style.display = "none";
    };

    const scheduleDrawerHide = () => {
      if (drawerHideTimeout) {
        clearTimeout(drawerHideTimeout);
      }
      drawerHideTimeout = setTimeout(() => {
        if (!drawerNodeHover && !drawerTooltipHover) {
          hideDrawerTip();
        }
      }, 220);
    };

    const showDrawerTip = (evt, node) => {
      const data = node.data();
      const path = data.full || data.id;
      // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: path escaped via escapeHtml(); other values are numbers
      drawerTooltip.innerHTML = `
        <div style="margin-bottom:4px"><strong>${escapeHtml(path)}</strong></div>
        <div>LOC: ${data.loc || 0} | degree: ${data.degree || 0}</div>
        <div>component: C${data.component || "?"}</div>
      `;
      const rect = drawerCanvas.getBoundingClientRect();
      const scrollX = window.scrollX || document.documentElement.scrollLeft || 0;
      const scrollY = window.scrollY || document.documentElement.scrollTop || 0;
      let left = rect.left + evt.renderedPosition.x + 12 + scrollX;
      let top = rect.top + evt.renderedPosition.y + 12 + scrollY;
      const maxLeft = scrollX + window.innerWidth - 220;
      if (left > maxLeft) left = maxLeft;
      drawerTooltip.style.left = left + "px";
      drawerTooltip.style.top = top + "px";
      drawerTooltip.style.display = "block";
      drawerNodeHover = true;
      if (drawerHideTimeout) {
        clearTimeout(drawerHideTimeout);
        drawerHideTimeout = null;
      }
    };

    drawerTooltip.addEventListener("mouseenter", () => {
      drawerTooltipHover = true;
      if (drawerHideTimeout) {
        clearTimeout(drawerHideTimeout);
        drawerHideTimeout = null;
      }
    });
    drawerTooltip.addEventListener("mouseleave", () => {
      drawerTooltipHover = false;
      scheduleDrawerHide();
    });

    const applyDrawerFilters = (runLayout = true) => {
      const txt = (drawerFilter?.value || "").toLowerCase();
      const minDeg = parseInt(drawerMinDeg?.value || "0", 10) || 0;
      const filtered = filterElements(drawerElements, { text: txt, minDeg, allowedComponents: new Set() });
      const nodes = filtered.nodes;
      const edges = filtered.edges;
      cyDrawer.elements().remove();
      cyDrawer.add({ nodes, edges });
      const labelsOn = drawerLabels?.checked && nodes.length <= 800;
      cyDrawer.style().selector("node").style("label", labelsOn ? "data(label)" : "").update();
      if (runLayout) {
        const layoutName = getDrawerSelectedLayout();
        const layoutConfig = getDrawerLayoutConfig(layoutName, nodes.length);
        cyDrawer.layout(layoutConfig).run();
      }
    };

    const runDrawerRelayout = () => {
      const layoutName = getDrawerSelectedLayout();
      const nodeCount = cyDrawer.nodes().length;
      const layoutConfig = getDrawerLayoutConfig(layoutName, nodeCount);
      cyDrawer.layout(layoutConfig).run();
    };

    if (drawerFilter) drawerFilter.addEventListener("input", () => applyDrawerFilters(false));
    if (drawerMinDeg) drawerMinDeg.addEventListener("input", () => applyDrawerFilters(false));
    if (drawerLabels) drawerLabels.addEventListener("change", () => applyDrawerFilters(false));
    if (drawerLayoutSelect) drawerLayoutSelect.addEventListener("change", runDrawerRelayout);
    if (drawerRelayout) drawerRelayout.addEventListener("click", runDrawerRelayout);
    if (drawerFit) drawerFit.addEventListener("click", () => cyDrawer.fit());
    if (drawerReset)
      drawerReset.addEventListener("click", () => {
        drawerElements = buildElements(activeGraph);
        cyDrawer.elements().remove();
        cyDrawer.add(drawerElements);
        drawerLayoutSelect.value = "preset";
        cyDrawer.layout({ name: "preset", animate: false, fit: true }).run();
        applyDrawerFilters(false);
      });
    if (drawerPng)
      drawerPng.addEventListener("click", () => {
        const dark = drawerDarkChk && drawerDarkChk.checked;
        const dataUrl = cyDrawer.png({ bg: dark ? "#0f1115" : "#ffffff", full: true, scale: 2 });
        const a = document.createElement("a");
        a.href = dataUrl;
        a.download = `${activeGraph.id}-graph.png`;
        document.body.appendChild(a);
        a.click();
        a.remove();
      });
    if (drawerFs && drawerCanvas.requestFullscreen) {
      drawerFs.addEventListener("click", () => {
        if (document.fullscreenElement) {
          document.exitFullscreen();
        } else {
          drawerCanvas.requestFullscreen().catch(() => {});
        }
      });
      document.addEventListener("fullscreenchange", () => {
        drawerFs.textContent = document.fullscreenElement ? "exit fullscreen" : "fullscreen";
        if (!document.fullscreenElement) cyDrawer.fit();
      });
    }
    if (drawerDarkChk) {
      darkToggles.add(drawerDarkChk);
      drawerDarkChk.addEventListener("change", () => applyDarkShared(drawerDarkChk.checked));
    }
    applyDrawerFilters(true);

    cyDrawer.on("mouseover", "node", (evt) => {
      drawerNodeHover = true;
      showDrawerTip(evt, evt.target);
    });
    cyDrawer.on("mouseout", "node", () => {
      drawerNodeHover = false;
      scheduleDrawerHide();
    });
    cyDrawer.on("tap", "node", (evt) => {
      drawerNodeHover = true;
      showDrawerTip(evt, evt.target);
    });
    cyDrawer.on("tapdrag", "node", () => {
      drawerNodeHover = false;
      hideDrawerTip();
    });

    if (select) {
      select.addEventListener("change", () => {
        const next = graphMap.get(select.value);
        if (!next) return;
        activeGraph = next;
        drawerElements = buildElements(activeGraph);
        cyDrawer.elements().remove();
        cyDrawer.add(drawerElements);
        // Use selected layout when switching graphs
        const layoutName = getDrawerSelectedLayout();
        const layoutConfig = getDrawerLayoutConfig(layoutName, drawerElements.nodes.length);
        cyDrawer.layout(layoutConfig).run();
        applyDrawerFilters(false);
        hideDrawerTip();
      });
    }
  }
})();
