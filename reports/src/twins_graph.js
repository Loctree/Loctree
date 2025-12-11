/**
 * Twin DNA Graph Visualization
 *
 * Visualizes files that export symbols with the same name (twins/dead parrots).
 * - Nodes = files that export symbols
 * - Edges = shared symbol names (twins) between files
 * - Edge thickness = number of shared symbols
 * - Node color = gradient from green (0 twins) to red (many twins)
 * - Node size = total exports from that file
 *
 * Interactive features:
 * - Hover on node → tooltip with file path, dead parrots list
 * - Hover on edge → tooltip with shared symbols
 * - Click on node → highlight all connections
 * - Double-click → open in editor (loctree:// URL)
 */

(function() {
  /**
   * Builds and renders the Twins Graph using Cytoscape.js
   *
   * @param {Object} twinsData - The twins analysis data
   * @param {Array} twinsData.exactTwins - Array of {symbol: string, files: string[]}
   * @param {Array} twinsData.deadParrots - Array of {name: string, file: string, line: number}
   * @param {string} containerId - ID of the container element
   * @param {string} [openBase] - Base URL for opening files in editor (optional)
   */
  window.buildTwinsGraph = function(twinsData, containerId, openBase) {
    const container = document.getElementById(containerId);
    if (!container) {
      console.error(`Container ${containerId} not found`);
      return null;
    }

    // Process twins data to build graph structure
    const { nodes, edges, stats } = processTwinsData(twinsData);

    // Create Cytoscape instance with stunning visuals
    const cy = cytoscape({
      container: container,
      elements: { nodes, edges },
      style: getTwinsGraphStyle(stats.maxDeadParrots, stats.maxSharedSymbols),
      layout: {
        name: 'cose',
        animate: true,
        animationDuration: 800,
        animationEasing: 'ease-out-cubic',
        fit: true,
        padding: 50,
        nodeRepulsion: function(node) {
          // More repulsion for nodes with many dead parrots (spread them out)
          const deadParrots = node.data('deadParrots').length;
          return 4000 + (deadParrots * 500);
        },
        idealEdgeLength: function(edge) {
          // Shorter edges for more shared symbols (pull them together)
          const sharedCount = edge.data('sharedSymbols').length;
          return Math.max(80, 200 - (sharedCount * 10));
        },
        edgeElasticity: function(edge) {
          // More elastic edges for stronger connections
          const sharedCount = edge.data('sharedSymbols').length;
          return 100 + (sharedCount * 20);
        },
        gravity: 0.5,
        numIter: 1500,
        initialTemp: 1000,
        coolingFactor: 0.95,
        minTemp: 1.0,
      },
      minZoom: 0.3,
      maxZoom: 3,
      wheelSensitivity: 0.2,
    });

    // Add interactive features
    setupInteractivity(cy, openBase, stats);

    // Add toolbar controls
    setupToolbar(cy, container, containerId, stats);

    return cy;
  };

  /**
   * Process twins data into Cytoscape-compatible nodes and edges
   */
  function processTwinsData(twinsData) {
    const { exactTwins, deadParrots } = twinsData;

    // Build file->exports map and file->deadParrots map
    const fileExports = new Map(); // file -> Set of symbol names
    const fileDeadParrots = new Map(); // file -> Array of dead parrot objects
    const fileConnections = new Map(); // file -> Set of connected files

    // Process dead parrots
    deadParrots.forEach(dp => {
      if (!fileDeadParrots.has(dp.file)) {
        fileDeadParrots.set(dp.file, []);
      }
      fileDeadParrots.get(dp.file).push(dp);
    });

    // Process exact twins to build connections
    exactTwins.forEach(twin => {
      const { symbol, files } = twin;

      // Add symbol to each file's exports
      files.forEach(file => {
        if (!fileExports.has(file)) {
          fileExports.set(file, new Set());
        }
        fileExports.get(file).add(symbol);
      });

      // Create connections between files that share this symbol
      for (let i = 0; i < files.length; i++) {
        for (let j = i + 1; j < files.length; j++) {
          const file1 = files[i];
          const file2 = files[j];

          if (!fileConnections.has(file1)) {
            fileConnections.set(file1, new Map());
          }
          if (!fileConnections.has(file2)) {
            fileConnections.set(file2, new Map());
          }

          // Track shared symbols between these two files
          if (!fileConnections.get(file1).has(file2)) {
            fileConnections.get(file1).set(file2, []);
          }
          if (!fileConnections.get(file2).has(file1)) {
            fileConnections.get(file2).set(file1, []);
          }

          fileConnections.get(file1).get(file2).push(symbol);
          fileConnections.get(file2).get(file1).push(symbol);
        }
      }
    });

    // Build nodes
    const nodes = [];
    const allFiles = new Set([...fileExports.keys(), ...fileDeadParrots.keys()]);

    let maxDeadParrots = 0;
    let maxExports = 0;

    allFiles.forEach(file => {
      const exports = fileExports.get(file) || new Set();
      const deadParrotsForFile = fileDeadParrots.get(file) || [];

      maxDeadParrots = Math.max(maxDeadParrots, deadParrotsForFile.length);
      maxExports = Math.max(maxExports, exports.size);

      nodes.push({
        data: {
          id: file,
          label: getFileLabel(file),
          fullPath: file,
          exportCount: exports.size,
          deadParrots: deadParrotsForFile,
          deadParrotCount: deadParrotsForFile.length,
        }
      });
    });

    // Build edges
    const edges = [];
    const processedPairs = new Set();

    fileConnections.forEach((connections, sourceFile) => {
      connections.forEach((sharedSymbols, targetFile) => {
        // Avoid duplicate edges
        const pairKey = [sourceFile, targetFile].sort().join('|||');
        if (processedPairs.has(pairKey)) return;
        processedPairs.add(pairKey);

        edges.push({
          data: {
            id: `${sourceFile}--${targetFile}`,
            source: sourceFile,
            target: targetFile,
            sharedSymbols: sharedSymbols,
            sharedCount: sharedSymbols.length,
          }
        });
      });
    });

    const stats = {
      maxDeadParrots,
      maxExports,
      maxSharedSymbols: Math.max(...edges.map(e => e.data.sharedCount), 1),
      totalFiles: allFiles.size,
      totalTwins: exactTwins.length,
      totalDeadParrots: deadParrots.length,
    };

    return { nodes, edges, stats };
  }

  /**
   * Get a shortened label for a file path
   */
  function getFileLabel(filePath) {
    const parts = filePath.split('/');
    if (parts.length <= 2) return filePath;
    // Show last 2 parts: "dir/file.rs"
    return parts.slice(-2).join('/');
  }

  /**
   * Generate Cytoscape style with dynamic gradients
   */
  function getTwinsGraphStyle(maxDeadParrots, maxSharedSymbols) {
    return [
      // Base node style
      {
        selector: 'node',
        style: {
          'label': 'data(label)',
          'font-size': 11,
          'font-weight': 'bold',
          'text-wrap': 'wrap',
          'text-max-width': 140,
          'text-valign': 'center',
          'text-halign': 'center',
          'color': '#fff',
          'text-outline-color': '#000',
          'text-outline-width': 2,
          'background-color': function(ele) {
            return getNodeColor(ele.data('deadParrotCount'), maxDeadParrots);
          },
          'width': function(ele) {
            // Size based on export count
            const exportCount = ele.data('exportCount') || 0;
            return Math.max(30, Math.min(80, 30 + exportCount * 3));
          },
          'height': function(ele) {
            const exportCount = ele.data('exportCount') || 0;
            return Math.max(30, Math.min(80, 30 + exportCount * 3));
          },
          'border-width': 3,
          'border-color': function(ele) {
            const deadParrots = ele.data('deadParrotCount') || 0;
            return deadParrots > 0 ? '#ff0000' : '#4a90e2';
          },
          'border-opacity': function(ele) {
            const deadParrots = ele.data('deadParrotCount') || 0;
            return deadParrots > 0 ? 0.8 : 0.4;
          },
          'transition-property': 'background-color, border-color, border-width',
          'transition-duration': '0.3s',
          'overlay-padding': 10,
          'overlay-opacity': 0,
        }
      },
      // Highlighted node
      {
        selector: 'node.highlight',
        style: {
          'border-width': 5,
          'border-color': '#ffd700',
          'border-opacity': 1,
          'shadow-blur': 20,
          'shadow-color': '#ffd700',
          'shadow-opacity': 0.8,
          'shadow-offset-x': 0,
          'shadow-offset-y': 0,
          'z-index': 999,
        }
      },
      // Dimmed node
      {
        selector: 'node.dimmed',
        style: {
          'opacity': 0.2,
        }
      },
      // Base edge style
      {
        selector: 'edge',
        style: {
          'curve-style': 'bezier',
          'width': function(ele) {
            // Thickness based on shared symbols count
            const count = ele.data('sharedCount') || 1;
            return Math.max(1, Math.min(12, count * 1.5));
          },
          'line-color': function(ele) {
            return getEdgeColor(ele.data('sharedCount'), maxSharedSymbols);
          },
          'target-arrow-color': function(ele) {
            return getEdgeColor(ele.data('sharedCount'), maxSharedSymbols);
          },
          'target-arrow-shape': 'none',
          'opacity': 0.6,
          'label': '',
          'font-size': 9,
          'text-background-color': '#000',
          'text-background-opacity': 0.7,
          'text-background-padding': 3,
          'color': '#fff',
          'transition-property': 'width, line-color, opacity',
          'transition-duration': '0.3s',
        }
      },
      // Highlighted edge
      {
        selector: 'edge.highlight',
        style: {
          'width': function(ele) {
            const count = ele.data('sharedCount') || 1;
            return Math.max(3, Math.min(16, count * 2));
          },
          'opacity': 1,
          'z-index': 998,
          'label': function(ele) {
            const symbols = ele.data('sharedSymbols') || [];
            return symbols.slice(0, 3).join(', ') + (symbols.length > 3 ? '...' : '');
          },
        }
      },
      // Dimmed edge
      {
        selector: 'edge.dimmed',
        style: {
          'opacity': 0.1,
        }
      },
    ];
  }

  /**
   * Get node color based on dead parrot count (green -> yellow -> orange -> red)
   */
  function getNodeColor(deadParrotCount, maxDeadParrots) {
    if (deadParrotCount === 0) {
      return '#22c55e'; // green - no dead parrots
    }

    // Normalize to 0-1 range
    const ratio = Math.min(deadParrotCount / Math.max(maxDeadParrots, 1), 1);

    // Color gradient: green -> yellow -> orange -> red
    if (ratio < 0.25) {
      // Green to yellow
      const t = ratio / 0.25;
      return interpolateColor('#22c55e', '#eab308', t);
    } else if (ratio < 0.5) {
      // Yellow to orange
      const t = (ratio - 0.25) / 0.25;
      return interpolateColor('#eab308', '#f97316', t);
    } else if (ratio < 0.75) {
      // Orange to deep orange
      const t = (ratio - 0.5) / 0.25;
      return interpolateColor('#f97316', '#ea580c', t);
    } else {
      // Deep orange to red
      const t = (ratio - 0.75) / 0.25;
      return interpolateColor('#ea580c', '#dc2626', t);
    }
  }

  /**
   * Get edge color based on shared symbols count
   */
  function getEdgeColor(sharedCount, maxSharedSymbols) {
    // Normalize to 0-1 range
    const ratio = Math.min(sharedCount / Math.max(maxSharedSymbols, 1), 1);

    // Color gradient: light blue -> purple -> magenta
    if (ratio < 0.5) {
      const t = ratio / 0.5;
      return interpolateColor('#60a5fa', '#a855f7', t); // blue to purple
    } else {
      const t = (ratio - 0.5) / 0.5;
      return interpolateColor('#a855f7', '#ec4899', t); // purple to magenta
    }
  }

  /**
   * Interpolate between two hex colors
   */
  function interpolateColor(color1, color2, t) {
    const c1 = hexToRgb(color1);
    const c2 = hexToRgb(color2);

    const r = Math.round(c1.r + (c2.r - c1.r) * t);
    const g = Math.round(c1.g + (c2.g - c1.g) * t);
    const b = Math.round(c1.b + (c2.b - c1.b) * t);

    return rgbToHex(r, g, b);
  }

  function hexToRgb(hex) {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
      r: parseInt(result[1], 16),
      g: parseInt(result[2], 16),
      b: parseInt(result[3], 16)
    } : null;
  }

  function rgbToHex(r, g, b) {
    return "#" + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
  }

  /**
   * Setup interactive features (hover, click, double-click)
   */
  function setupInteractivity(cy, openBase, stats) {
    // Create tooltip element
    const tooltip = document.createElement('div');
    tooltip.className = 'twins-graph-tooltip';
    tooltip.style.cssText = `
      position: fixed;
      pointer-events: none;
      background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
      color: #fff;
      padding: 12px 16px;
      border-radius: 8px;
      font-size: 12px;
      display: none;
      z-index: 10000;
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
      border: 1px solid rgba(255, 255, 255, 0.1);
      max-width: 400px;
      backdrop-filter: blur(10px);
    `;
    document.body.appendChild(tooltip);

    let nodeHoverTimeout = null;
    let edgeHoverTimeout = null;

    // Node hover - show file info and dead parrots
    cy.on('mouseover', 'node', function(evt) {
      const node = evt.target;
      const data = node.data();

      clearTimeout(nodeHoverTimeout);
      nodeHoverTimeout = setTimeout(() => {
        // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: clearing tooltip with empty string, no user input
        tooltip.innerHTML = ''; // Clear previous

        // File path
        const pathDiv = document.createElement('div');
        pathDiv.style.cssText = 'font-weight: bold; margin-bottom: 8px; color: #60a5fa;';
        pathDiv.textContent = data.fullPath;
        tooltip.appendChild(pathDiv);

        // Stats
        const statsDiv = document.createElement('div');
        statsDiv.style.cssText = 'margin-bottom: 8px; font-size: 11px; opacity: 0.9;';
        const createStatDiv = (text) => {
          const div = document.createElement('div');
          div.textContent = text;
          return div;
        };
        statsDiv.appendChild(createStatDiv('Exports: ' + data.exportCount));
        statsDiv.appendChild(createStatDiv('Dead Parrots: ' + data.deadParrotCount));
        statsDiv.appendChild(createStatDiv('Connections: ' + node.degree()));
        tooltip.appendChild(statsDiv);

        // Dead parrots list
        if (data.deadParrots.length > 0) {
          const dpTitle = document.createElement('div');
          dpTitle.style.cssText = 'font-weight: bold; margin-top: 8px; color: #f87171;';
          dpTitle.textContent = 'Dead Parrots:';
          tooltip.appendChild(dpTitle);

          const dpList = document.createElement('ul');
          dpList.style.cssText = 'margin: 4px 0 0 0; padding-left: 20px; font-size: 10px;';

          data.deadParrots.slice(0, 10).forEach(dp => {
            const li = document.createElement('li');
            li.style.cssText = 'margin: 2px 0;';
            li.textContent = `${dp.name} (line ${dp.line})`;
            dpList.appendChild(li);
          });

          if (data.deadParrots.length > 10) {
            const more = document.createElement('li');
            more.style.cssText = 'margin: 2px 0; font-style: italic;';
            more.textContent = `... and ${data.deadParrots.length - 10} more`;
            dpList.appendChild(more);
          }

          tooltip.appendChild(dpList);
        }

        // Open in editor hint
        if (openBase) {
          const hint = document.createElement('div');
          hint.style.cssText = 'margin-top: 8px; font-size: 10px; opacity: 0.7; font-style: italic;';
          hint.textContent = 'Double-click to open in editor';
          tooltip.appendChild(hint);
        }

        tooltip.style.display = 'block';
        positionTooltip(evt.renderedPosition);
      }, 100);
    });

    cy.on('mouseout', 'node', function() {
      clearTimeout(nodeHoverTimeout);
      tooltip.style.display = 'none';
    });

    // Edge hover - show shared symbols
    cy.on('mouseover', 'edge', function(evt) {
      const edge = evt.target;
      const data = edge.data();

      clearTimeout(edgeHoverTimeout);
      edgeHoverTimeout = setTimeout(() => {
        // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: clearing tooltip with empty string, no user input
        tooltip.innerHTML = '';

        const titleDiv = document.createElement('div');
        titleDiv.style.cssText = 'font-weight: bold; margin-bottom: 8px; color: #a855f7;';
        titleDiv.textContent = `${data.sharedCount} Shared Symbol${data.sharedCount > 1 ? 's' : ''}`;
        tooltip.appendChild(titleDiv);

        const symbolList = document.createElement('ul');
        symbolList.style.cssText = 'margin: 0; padding-left: 20px; font-size: 11px;';

        data.sharedSymbols.forEach(symbol => {
          const li = document.createElement('li');
          li.style.cssText = 'margin: 2px 0;';
          li.textContent = symbol;
          symbolList.appendChild(li);
        });

        tooltip.appendChild(symbolList);
        tooltip.style.display = 'block';
        positionTooltip(evt.renderedPosition);
      }, 100);
    });

    cy.on('mouseout', 'edge', function() {
      clearTimeout(edgeHoverTimeout);
      tooltip.style.display = 'none';
    });

    // Click on node - highlight connections
    cy.on('tap', 'node', function(evt) {
      const node = evt.target;

      // Clear previous highlights
      cy.elements().removeClass('highlight dimmed');

      // Highlight this node and its neighborhood
      node.addClass('highlight');
      node.neighborhood().addClass('highlight');

      // Dim everything else
      cy.elements().not(node.neighborhood().union(node)).addClass('dimmed');
    });

    // Click on background - clear highlights
    cy.on('tap', function(evt) {
      if (evt.target === cy) {
        cy.elements().removeClass('highlight dimmed');
      }
    });

    // Double-click on node - open in editor
    if (openBase) {
      cy.on('dbltap', 'node', function(evt) {
        const node = evt.target;
        const filePath = node.data('fullPath');
        const url = `${openBase}/open?f=${encodeURIComponent(filePath)}&l=1`;
        window.open(url, '_blank');
      });
    }

    function positionTooltip(renderedPos) {
      const containerRect = cy.container().getBoundingClientRect();
      let left = containerRect.left + renderedPos.x + 15;
      let top = containerRect.top + renderedPos.y + 15;

      // Keep tooltip within viewport
      const tooltipRect = tooltip.getBoundingClientRect();
      const maxLeft = window.innerWidth - tooltipRect.width - 10;
      const maxTop = window.innerHeight - tooltipRect.height - 10;

      if (left > maxLeft) left = Math.max(10, renderedPos.x - tooltipRect.width - 15);
      if (top > maxTop) top = Math.max(10, renderedPos.y - tooltipRect.height - 15);

      tooltip.style.left = left + 'px';
      tooltip.style.top = top + 'px';
    }
  }

  /**
   * Setup toolbar with controls
   */
  function setupToolbar(cy, container, containerId, stats) {
    // Create toolbar
    const toolbar = document.createElement('div');
    toolbar.className = 'twins-graph-toolbar';
    toolbar.style.cssText = `
      position: absolute;
      top: 10px;
      left: 10px;
      right: 10px;
      background: rgba(0, 0, 0, 0.8);
      backdrop-filter: blur(10px);
      padding: 12px 16px;
      border-radius: 8px;
      display: flex;
      gap: 16px;
      align-items: center;
      flex-wrap: wrap;
      z-index: 100;
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
      font-size: 12px;
      color: #fff;
    `;

    // Stats display
    const statsDiv = document.createElement('div');
    statsDiv.style.cssText = 'display: flex; gap: 16px; margin-right: auto;';
    const createStatSpan = (label, value) => {
      const span = document.createElement('span');
      const strong = document.createElement('strong');
      strong.textContent = label + ':';
      span.appendChild(strong);
      span.appendChild(document.createTextNode(' ' + value));
      return span;
    };
    statsDiv.appendChild(createStatSpan('Files', stats.totalFiles));
    statsDiv.appendChild(createStatSpan('Twins', stats.totalTwins));
    statsDiv.appendChild(createStatSpan('Dead Parrots', stats.totalDeadParrots));
    toolbar.appendChild(statsDiv);

    // Layout selector
    const layoutLabel = document.createElement('label');
    layoutLabel.style.cssText = 'display: flex; gap: 6px; align-items: center;';
    // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: static HTML label with no user input
    layoutLabel.innerHTML = '<span>Layout:</span>';

    const layoutSelect = document.createElement('select');
    layoutSelect.style.cssText = 'background: #1a1a2e; color: #fff; border: 1px solid #444; padding: 4px 8px; border-radius: 4px;';
    // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: static dropdown options with no user input
    layoutSelect.innerHTML = `
      <option value="cose">Force (COSE)</option>
      <option value="cose-bilkent">Force (Bilkent)</option>
      <option value="concentric">Concentric</option>
      <option value="circle">Circle</option>
      <option value="grid">Grid</option>
    `;
    layoutLabel.appendChild(layoutSelect);
    toolbar.appendChild(layoutLabel);

    layoutSelect.addEventListener('change', () => {
      const layoutName = layoutSelect.value;
      cy.layout({
        name: layoutName,
        animate: true,
        animationDuration: 600,
        fit: true,
        padding: 50,
      }).run();
    });

    // Fit button
    const fitBtn = document.createElement('button');
    fitBtn.textContent = 'Fit';
    fitBtn.style.cssText = 'background: #4a90e2; color: #fff; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;';
    fitBtn.addEventListener('click', () => cy.fit(null, 30));
    toolbar.appendChild(fitBtn);

    // Reset button
    const resetBtn = document.createElement('button');
    resetBtn.textContent = 'Reset';
    resetBtn.style.cssText = 'background: #666; color: #fff; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;';
    resetBtn.addEventListener('click', () => {
      cy.elements().removeClass('highlight dimmed');
      cy.fit(null, 30);
    });
    toolbar.appendChild(resetBtn);

    // Export PNG button
    const pngBtn = document.createElement('button');
    pngBtn.textContent = 'Export PNG';
    pngBtn.style.cssText = 'background: #22c55e; color: #fff; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;';
    pngBtn.addEventListener('click', () => {
      const dataUrl = cy.png({ bg: '#0f1115', full: true, scale: 2 });
      const a = document.createElement('a');
      a.href = dataUrl;
      a.download = `${containerId}-twins-graph.png`;
      a.click();
    });
    toolbar.appendChild(pngBtn);

    // Insert toolbar into container
    container.style.position = 'relative';
    container.insertBefore(toolbar, container.firstChild);
  }

})();
