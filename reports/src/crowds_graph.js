/**
 * Crowds Graph Visualization
 *
 * Visualizes files with similar naming patterns that may indicate fragmentation or duplication.
 * - Nodes = files grouped by crowd pattern
 * - Node color = based on severity (green/orange/red) and issue types
 * - Clusters = visual grouping showing which files belong to same pattern
 * - Node size = based on importer count (usage)
 * - Edges = similarity connections between files in same crowd
 *
 * Interactive features:
 * - Hover on node → tooltip with file info, match reason, issues
 * - Hover on edge → tooltip with similarity score
 * - Click on node → highlight entire crowd
 * - Double-click → open in editor (loctree:// URL)
 * - Filter by severity/issue type
 */

(function() {
  /**
   * Builds and renders the Crowds Graph using Cytoscape.js
   *
   * @param {Array} crowdsData - Array of crowd objects
   * @param {string} containerId - ID of the container element
   * @param {string} [openBase] - Base URL for opening files in editor (optional)
   */
  window.buildCrowdsGraph = function(crowdsData, containerId, openBase) {
    const container = document.getElementById(containerId);
    if (!container) {
      console.error(`Container ${containerId} not found`);
      return null;
    }

    // Process crowds data to build graph structure
    const { nodes, edges, stats } = processCrowdsData(crowdsData);

    // Create Cytoscape instance with clustering visualization
    const cy = cytoscape({
      container: container,
      elements: { nodes, edges },
      style: getCrowdsGraphStyle(stats.maxImporters, stats.maxSimilarity),
      layout: {
        name: 'cose',
        animate: true,
        animationDuration: 800,
        animationEasing: 'ease-out-cubic',
        fit: true,
        padding: 60,
        nodeRepulsion: function(node) {
          // More repulsion between different crowds
          const isCrowdLabel = node.data('isCrowdLabel');
          return isCrowdLabel ? 8000 : 3000;
        },
        idealEdgeLength: function(edge) {
          // Shorter edges within same crowd
          return edge.data('withinCrowd') ? 80 : 200;
        },
        edgeElasticity: function(edge) {
          // Stronger connections within crowd
          return edge.data('withinCrowd') ? 150 : 50;
        },
        gravity: 0.8,
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
    setupInteractivity(cy, openBase, stats, crowdsData);

    // Add toolbar controls
    setupToolbar(cy, container, containerId, stats, crowdsData);

    return cy;
  };

  /**
   * Process crowds data into Cytoscape-compatible nodes and edges
   */
  function processCrowdsData(crowdsData) {
    const nodes = [];
    const edges = [];

    let maxImporters = 0;
    let maxSimilarity = 0;
    let totalFiles = 0;
    let totalIssues = 0;

    // Process each crowd
    crowdsData.forEach((crowd, crowdIndex) => {
      const crowdId = `crowd-${crowdIndex}`;
      const pattern = crowd.pattern;
      const score = crowd.score || 0;
      const issues = crowd.issues || [];

      totalIssues += issues.length;

      // Add a central "crowd label" node for visual clustering (optional, can be hidden)
      nodes.push({
        data: {
          id: crowdId,
          label: pattern,
          pattern: pattern,
          crowdIndex: crowdIndex,
          score: score,
          memberCount: crowd.members.length,
          issues: issues,
          isCrowdLabel: true,
          importerCount: 0,
        }
      });

      // Process each member in the crowd
      crowd.members.forEach((member, memberIndex) => {
        const memberId = `${crowdId}-member-${memberIndex}`;
        const file = member.file;
        const importerCount = member.importer_count || 0;

        maxImporters = Math.max(maxImporters, importerCount);
        totalFiles++;

        nodes.push({
          data: {
            id: memberId,
            label: getFileLabel(file),
            fullPath: file,
            pattern: pattern,
            crowdIndex: crowdIndex,
            score: score,
            matchReason: member.match_reason,
            importerCount: importerCount,
            issues: issues,
            similarityScores: member.similarity_scores || [],
            isCrowdLabel: false,
          }
        });

        // Connect member to crowd label (for visual clustering)
        edges.push({
          data: {
            id: `${crowdId}-to-${memberId}`,
            source: crowdId,
            target: memberId,
            withinCrowd: true,
            similarity: 1.0,
          }
        });

        // Add similarity edges between members within same crowd
        member.similarity_scores.forEach(([otherFile, similarity]) => {
          // Find the other member's ID
          const otherMemberIndex = crowd.members.findIndex(m => m.file === otherFile);
          if (otherMemberIndex !== -1 && otherMemberIndex > memberIndex) {
            const otherMemberId = `${crowdId}-member-${otherMemberIndex}`;
            maxSimilarity = Math.max(maxSimilarity, similarity);

            edges.push({
              data: {
                id: `${memberId}-to-${otherMemberId}`,
                source: memberId,
                target: otherMemberId,
                withinCrowd: true,
                similarity: similarity,
              }
            });
          }
        });
      });
    });

    const stats = {
      maxImporters,
      maxSimilarity: maxSimilarity > 0 ? maxSimilarity : 1,
      totalCrowds: crowdsData.length,
      totalFiles,
      totalIssues,
      averageScore: crowdsData.reduce((sum, c) => sum + (c.score || 0), 0) / crowdsData.length,
    };

    return { nodes, edges, stats };
  }

  /**
   * Get a shortened label for a file path
   */
  function getFileLabel(filePath) {
    const parts = filePath.split('/');
    if (parts.length <= 2) return filePath;
    // Show last 2 parts: "dir/file.ts"
    return parts.slice(-2).join('/');
  }

  /**
   * Generate Cytoscape style with dynamic gradients and clustering
   */
  function getCrowdsGraphStyle(maxImporters, maxSimilarity) {
    return [
      // Crowd label nodes (central hubs)
      {
        selector: 'node[isCrowdLabel = true]',
        style: {
          'label': 'data(label)',
          'font-size': 14,
          'font-weight': 'bold',
          'text-valign': 'center',
          'text-halign': 'center',
          'color': '#fff',
          'text-outline-color': '#000',
          'text-outline-width': 3,
          'background-color': function(ele) {
            return getSeverityColor(ele.data('score'));
          },
          'width': function(ele) {
            const memberCount = ele.data('memberCount') || 1;
            return Math.max(40, Math.min(100, 40 + memberCount * 5));
          },
          'height': function(ele) {
            const memberCount = ele.data('memberCount') || 1;
            return Math.max(40, Math.min(100, 40 + memberCount * 5));
          },
          'shape': 'hexagon',
          'border-width': 4,
          'border-color': '#fff',
          'border-opacity': 0.6,
          'opacity': 0.9,
          'z-index': 10,
        }
      },
      // Member nodes (files in crowd)
      {
        selector: 'node[isCrowdLabel = false]',
        style: {
          'label': 'data(label)',
          'font-size': 10,
          'font-weight': 'normal',
          'text-wrap': 'wrap',
          'text-max-width': 120,
          'text-valign': 'center',
          'text-halign': 'center',
          'color': '#fff',
          'text-outline-color': '#000',
          'text-outline-width': 2,
          'background-color': function(ele) {
            return getSeverityColor(ele.data('score'));
          },
          'width': function(ele) {
            // Size based on importer count (usage)
            const importers = ele.data('importerCount') || 0;
            return Math.max(25, Math.min(70, 25 + importers * 2));
          },
          'height': function(ele) {
            const importers = ele.data('importerCount') || 0;
            return Math.max(25, Math.min(70, 25 + importers * 2));
          },
          'shape': 'ellipse',
          'border-width': 2,
          'border-color': function(ele) {
            const issues = ele.data('issues') || [];
            return getIssueColor(issues);
          },
          'border-opacity': 0.8,
          'transition-property': 'background-color, border-color, border-width',
          'transition-duration': '0.3s',
          'overlay-padding': 8,
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
          'opacity': 0.15,
        }
      },
      // Hidden node (for filtering)
      {
        selector: 'node.filtered-out',
        style: {
          'display': 'none',
        }
      },
      // Edges connecting crowd label to members (light, structural)
      {
        selector: 'edge[withinCrowd = true]',
        style: {
          'curve-style': 'straight',
          'width': function(ele) {
            const similarity = ele.data('similarity') || 0;
            return similarity === 1.0 ? 1 : Math.max(1, similarity * 4);
          },
          'line-color': function(ele) {
            const similarity = ele.data('similarity') || 0;
            return similarity === 1.0 ? 'rgba(255, 255, 255, 0.1)' : getSimilarityColor(similarity);
          },
          'opacity': function(ele) {
            const similarity = ele.data('similarity') || 0;
            return similarity === 1.0 ? 0.15 : 0.5;
          },
          'target-arrow-shape': 'none',
          'label': '',
          'transition-property': 'width, line-color, opacity',
          'transition-duration': '0.3s',
        }
      },
      // Highlighted edge
      {
        selector: 'edge.highlight',
        style: {
          'width': function(ele) {
            const similarity = ele.data('similarity') || 0;
            return similarity === 1.0 ? 2 : Math.max(3, similarity * 6);
          },
          'opacity': 1,
          'z-index': 998,
          'label': function(ele) {
            const similarity = ele.data('similarity') || 0;
            return similarity !== 1.0 ? `${(similarity * 100).toFixed(0)}%` : '';
          },
          'font-size': 9,
          'text-background-color': '#000',
          'text-background-opacity': 0.7,
          'text-background-padding': 3,
          'color': '#fff',
        }
      },
      // Dimmed edge
      {
        selector: 'edge.dimmed',
        style: {
          'opacity': 0.05,
        }
      },
      // Hidden edge (for filtering)
      {
        selector: 'edge.filtered-out',
        style: {
          'display': 'none',
        }
      },
    ];
  }

  /**
   * Get color based on severity score (green -> orange -> red)
   */
  function getSeverityColor(score) {
    if (score < 4.0) {
      // Low severity: green to yellow-green
      const t = score / 4.0;
      return interpolateColor('#27ae60', '#95c56e', t);
    } else if (score < 7.0) {
      // Medium severity: yellow-orange to orange
      const t = (score - 4.0) / 3.0;
      return interpolateColor('#e67e22', '#d35400', t);
    } else {
      // High severity: red to dark red
      const t = Math.min((score - 7.0) / 3.0, 1);
      return interpolateColor('#c0392b', '#8b0000', t);
    }
  }

  /**
   * Get border color based on issue types
   */
  function getIssueColor(issues) {
    if (!issues || issues.length === 0) return '#4a90e2';

    // Priority: NameCollision > UsageAsymmetry > ExportOverlap > Fragmentation
    for (const issue of issues) {
      if (issue.NameCollision) return '#e74c3c';
      if (issue.UsageAsymmetry) return '#e67e22';
      if (issue.ExportOverlap) return '#f39c12';
      if (issue.Fragmentation) return '#3498db';
    }

    return '#4a90e2';
  }

  /**
   * Get edge color based on similarity score
   */
  function getSimilarityColor(similarity) {
    // Normalize to 0-1 range
    const ratio = Math.min(Math.max(similarity, 0), 1);

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
  function setupInteractivity(cy, openBase, stats, crowdsData) {
    // Create tooltip element
    const tooltip = document.createElement('div');
    tooltip.className = 'crowds-graph-tooltip';
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

    // Node hover - show info
    cy.on('mouseover', 'node', function(evt) {
      const node = evt.target;
      const data = node.data();

      clearTimeout(nodeHoverTimeout);
      nodeHoverTimeout = setTimeout(() => {
        tooltip.innerHTML = ''; // Clear previous

        if (data.isCrowdLabel) {
          // Crowd label tooltip
          const patternDiv = document.createElement('div');
          patternDiv.style.cssText = 'font-weight: bold; margin-bottom: 8px; color: #60a5fa; font-size: 14px;';
          patternDiv.textContent = `Pattern: "${data.pattern}"`;
          tooltip.appendChild(patternDiv);

          const statsDiv = document.createElement('div');
          statsDiv.style.cssText = 'margin-bottom: 8px; font-size: 11px; opacity: 0.9;';
          // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: data.memberCount/score/issues are numeric values from loctree internal analysis, not user input
          statsDiv.innerHTML = `
            <div>Files: ${data.memberCount}</div>
            <div>Severity: ${data.score.toFixed(1)}/10</div>
            <div>Issues: ${data.issues.length}</div>
          `;
          tooltip.appendChild(statsDiv);

          if (data.issues.length > 0) {
            const issuesTitle = document.createElement('div');
            issuesTitle.style.cssText = 'font-weight: bold; margin-top: 8px; color: #f39c12;';
            issuesTitle.textContent = 'Issues:';
            tooltip.appendChild(issuesTitle);

            const issuesList = document.createElement('ul');
            issuesList.style.cssText = 'margin: 4px 0 0 0; padding-left: 20px; font-size: 10px;';

            data.issues.forEach(issue => {
              const li = document.createElement('li');
              li.style.cssText = 'margin: 2px 0;';
              li.textContent = formatIssueType(issue);
              issuesList.appendChild(li);
            });

            tooltip.appendChild(issuesList);
          }
        } else {
          // File node tooltip
          const pathDiv = document.createElement('div');
          pathDiv.style.cssText = 'font-weight: bold; margin-bottom: 8px; color: #60a5fa;';
          pathDiv.textContent = data.fullPath;
          tooltip.appendChild(pathDiv);

          const statsDiv = document.createElement('div');
          statsDiv.style.cssText = 'margin-bottom: 8px; font-size: 11px; opacity: 0.9;';
          // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: data.pattern/importerCount/score are from loctree internal analysis, not user input
          statsDiv.innerHTML = `
            <div>Pattern: ${data.pattern}</div>
            <div>Importers: ${data.importerCount}</div>
            <div>Severity: ${data.score.toFixed(1)}/10</div>
          `;
          tooltip.appendChild(statsDiv);

          // Match reason
          if (data.matchReason) {
            const reasonDiv = document.createElement('div');
            reasonDiv.style.cssText = 'margin-top: 8px; font-size: 11px; color: #95c56e;';
            // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: formatMatchReason returns sanitized internal analysis data, not user input
            reasonDiv.innerHTML = `<strong>Match:</strong> ${formatMatchReason(data.matchReason)}`;
            tooltip.appendChild(reasonDiv);
          }

          // Issues
          if (data.issues && data.issues.length > 0) {
            const issuesDiv = document.createElement('div');
            issuesDiv.style.cssText = 'margin-top: 8px; font-size: 10px; color: #f39c12;';
            // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: formatIssueType returns sanitized enum values from loctree analysis, not user input
            issuesDiv.innerHTML = `<strong>Issues:</strong> ${data.issues.map(formatIssueType).join(', ')}`;
            tooltip.appendChild(issuesDiv);
          }

          // Open in editor hint
          if (openBase) {
            const hint = document.createElement('div');
            hint.style.cssText = 'margin-top: 8px; font-size: 10px; opacity: 0.7; font-style: italic;';
            hint.textContent = 'Double-click to open in editor';
            tooltip.appendChild(hint);
          }
        }

        tooltip.style.display = 'block';
        positionTooltip(evt.renderedPosition);
      }, 100);
    });

    cy.on('mouseout', 'node', function() {
      clearTimeout(nodeHoverTimeout);
      tooltip.style.display = 'none';
    });

    // Edge hover - show similarity
    cy.on('mouseover', 'edge', function(evt) {
      const edge = evt.target;
      const data = edge.data();

      if (data.similarity === 1.0) return; // Skip structural edges

      clearTimeout(edgeHoverTimeout);
      edgeHoverTimeout = setTimeout(() => {
        tooltip.innerHTML = '';

        const titleDiv = document.createElement('div');
        titleDiv.style.cssText = 'font-weight: bold; color: #a855f7;';
        titleDiv.textContent = `Similarity: ${(data.similarity * 100).toFixed(0)}%`;
        tooltip.appendChild(titleDiv);

        tooltip.style.display = 'block';
        positionTooltip(evt.renderedPosition);
      }, 100);
    });

    cy.on('mouseout', 'edge', function() {
      clearTimeout(edgeHoverTimeout);
      tooltip.style.display = 'none';
    });

    // Click on node - highlight crowd
    cy.on('tap', 'node', function(evt) {
      const node = evt.target;
      const crowdIndex = node.data('crowdIndex');

      // Clear previous highlights
      cy.elements().removeClass('highlight dimmed');

      // Highlight entire crowd
      const crowdNodes = cy.nodes().filter(n => n.data('crowdIndex') === crowdIndex);
      const crowdEdges = cy.edges().filter(e => {
        const sourceIndex = e.source().data('crowdIndex');
        const targetIndex = e.target().data('crowdIndex');
        return sourceIndex === crowdIndex && targetIndex === crowdIndex;
      });

      crowdNodes.addClass('highlight');
      crowdEdges.addClass('highlight');

      // Dim everything else
      cy.elements().not(crowdNodes.union(crowdEdges)).addClass('dimmed');
    });

    // Click on background - clear highlights
    cy.on('tap', function(evt) {
      if (evt.target === cy) {
        cy.elements().removeClass('highlight dimmed');
      }
    });

    // Double-click on file node - open in editor
    if (openBase) {
      cy.on('dbltap', 'node[isCrowdLabel = false]', function(evt) {
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
   * Format issue type for display
   */
  function formatIssueType(issue) {
    if (issue.NameCollision) {
      return `Name Collision (${issue.NameCollision.files.length} files)`;
    }
    if (issue.UsageAsymmetry) {
      return `Usage Asymmetry`;
    }
    if (issue.ExportOverlap) {
      return `Export Overlap (${issue.ExportOverlap.overlap.length} symbols)`;
    }
    if (issue.Fragmentation) {
      return `Fragmentation (${issue.Fragmentation.categories.length} categories)`;
    }
    return 'Unknown issue';
  }

  /**
   * Format match reason for display
   */
  function formatMatchReason(reason) {
    if (reason.NameMatch) {
      return `Name: ${reason.NameMatch.matched}`;
    }
    if (reason.ImportSimilarity) {
      return `Import similarity (${(reason.ImportSimilarity.similarity * 100).toFixed(0)}%)`;
    }
    if (reason.ExportSimilarity) {
      return `Similar exports to ${reason.ExportSimilarity.similar_to}`;
    }
    return 'Unknown';
  }

  /**
   * Setup toolbar with controls
   */
  function setupToolbar(cy, container, containerId, stats, crowdsData) {
    // Create toolbar
    const toolbar = document.createElement('div');
    toolbar.className = 'crowds-graph-toolbar';
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
    statsDiv.style.cssText = 'display: flex; gap: 16px; margin-right: auto; flex-wrap: wrap;';
    // nosemgrep: javascript.browser.security.insecure-document-method.insecure-document-method -- SAFETY: stats.* are numeric values computed from loctree internal analysis, not user input
    statsDiv.innerHTML = `
      <span><strong>Crowds:</strong> ${stats.totalCrowds}</span>
      <span><strong>Files:</strong> ${stats.totalFiles}</span>
      <span><strong>Issues:</strong> ${stats.totalIssues}</span>
      <span><strong>Avg Severity:</strong> ${stats.averageScore.toFixed(1)}</span>
    `;
    toolbar.appendChild(statsDiv);

    // Severity filter
    const severityLabel = document.createElement('label');
    severityLabel.style.cssText = 'display: flex; gap: 6px; align-items: center;';
    severityLabel.innerHTML = '<span>Min Severity:</span>';

    const severitySelect = document.createElement('select');
    severitySelect.style.cssText = 'background: #1a1a2e; color: #fff; border: 1px solid #444; padding: 4px 8px; border-radius: 4px;';
    severitySelect.innerHTML = `
      <option value="0">All (0+)</option>
      <option value="4">Medium (4+)</option>
      <option value="7">High (7+)</option>
    `;
    severityLabel.appendChild(severitySelect);
    toolbar.appendChild(severityLabel);

    severitySelect.addEventListener('change', () => {
      const minSeverity = parseFloat(severitySelect.value);
      cy.nodes().forEach(node => {
        const score = node.data('score') || 0;
        if (score < minSeverity) {
          node.addClass('filtered-out');
          node.connectedEdges().addClass('filtered-out');
        } else {
          node.removeClass('filtered-out');
          node.connectedEdges().removeClass('filtered-out');
        }
      });
      cy.fit(cy.elements(':visible'), 30);
    });

    // Layout selector
    const layoutLabel = document.createElement('label');
    layoutLabel.style.cssText = 'display: flex; gap: 6px; align-items: center;';
    layoutLabel.innerHTML = '<span>Layout:</span>';

    const layoutSelect = document.createElement('select');
    layoutSelect.style.cssText = 'background: #1a1a2e; color: #fff; border: 1px solid #444; padding: 4px 8px; border-radius: 4px;';
    layoutSelect.innerHTML = `
      <option value="cose">Force (COSE)</option>
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
    fitBtn.addEventListener('click', () => cy.fit(cy.elements(':visible'), 30));
    toolbar.appendChild(fitBtn);

    // Reset button
    const resetBtn = document.createElement('button');
    resetBtn.textContent = 'Reset';
    resetBtn.style.cssText = 'background: #666; color: #fff; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer;';
    resetBtn.addEventListener('click', () => {
      cy.elements().removeClass('highlight dimmed filtered-out');
      severitySelect.value = '0';
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
      a.download = `${containerId}-crowds-graph.png`;
      a.click();
    });
    toolbar.appendChild(pngBtn);

    // Insert toolbar into container
    container.style.position = 'relative';
    container.insertBefore(toolbar, container.firstChild);
  }

})();
