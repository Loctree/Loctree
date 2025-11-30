//! CSS styles for the HTML report.
//!
//! This module contains the complete CSS for rendering reports,
//! including responsive layouts, dark mode support, and graph styling.
//!
//! # Customization
//!
//! To extend or override styles:
//!
//! ```rust
//! use report_leptos::styles::REPORT_CSS;
//!
//! let my_css = ".custom-class { color: red; }";
//! let combined = format!("{}\n{}", REPORT_CSS, my_css);
//! ```
//!
//! # Features
//!
//! - CRT-inspired dark theme matching landing page
//! - Monospace typography (JetBrains Mono)
//! - Responsive tables and graphs
//! - Tab navigation styling
//! - Cytoscape graph container styling

/// Complete CSS for the report - CRT-inspired dark theme.
///
/// This CSS provides:
/// - Base typography and spacing (monospace)
/// - Tab navigation UI
/// - Table styling for data display
/// - Graph container and toolbar
/// - Dark theme by default (matching landing page)
/// - Component panels for graph analysis
pub const REPORT_CSS: &str = r#"
:root {
    --bg-black: #000000;
    --bg-dark: #0a0a0a;
    --bg-mid: #141414;
    --text-bright: #a8a8a8;
    --text-dim: #707070;
    --text-muted: #404040;
    --border-subtle: rgba(168, 168, 168, 0.1);
    --border-visible: rgba(168, 168, 168, 0.2);
    --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
    --container-max: 1000px;
    --accent-blue: #4f81e1;
    --accent-orange: #e67e22;
    --accent-red: #dc2626;
    --accent-green: #059669;
}

*, *::before, *::after {
    box-sizing: border-box;
}

html {
    scroll-behavior: smooth;
}

body {
    font-family: var(--font-mono);
    background: var(--bg-black);
    color: var(--text-bright);
    line-height: 1.6;
    margin: 0;
    min-height: 100vh;
}

html, body {
    height: 100%;
}

::selection {
    background: rgba(168, 168, 168, 0.3);
    color: var(--text-bright);
}

::-webkit-scrollbar {
    width: 6px;
    height: 6px;
}

::-webkit-scrollbar-track {
    background: var(--bg-dark);
}

::-webkit-scrollbar-thumb {
    background: var(--text-muted);
    border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
    background: var(--text-dim);
}

/* Layout */
.container {
    max-width: var(--container-max);
    margin: 0 auto;
    padding: 0 24px;
}

.report-page {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
}

/* Nav Bar */
.report-nav {
    border-bottom: 1px solid var(--border-visible);
    background: var(--bg-black);
    padding: 12px 0;
    position: sticky;
    top: 0;
    z-index: 50;
}

.report-nav-inner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
}

.report-nav-title {
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-size: 11px;
    color: var(--text-bright);
}

.report-nav-meta {
    color: var(--text-dim);
    font-size: 11px;
}

/* Layout Grid */
.report-layout {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
    gap: 0;
    align-items: flex-start;
    flex: 1 0 auto;
}

/* Sidebar */
.report-sidebar {
    border-right: 1px solid var(--border-visible);
    padding: 20px 16px 20px 0;
    font-size: 12px;
    position: sticky;
    top: 48px;
    max-height: calc(100vh - 64px);
    overflow: auto;
}

.report-sidebar h2 {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.2em;
    margin-bottom: 12px;
    color: var(--text-muted);
    font-weight: 400;
}

.report-sidebar ul {
    list-style: none;
    margin: 0;
    padding: 0;
}

.report-sidebar li {
    margin-bottom: 2px;
}

.report-sidebar a {
    color: var(--text-dim);
    text-decoration: none;
    font-size: 11px;
    display: block;
    padding: 4px 8px;
    border-radius: 4px;
    transition: all 0.15s;
}

.report-sidebar a:hover {
    color: var(--text-bright);
    background: var(--border-subtle);
}

/* Main Content */
.report-main {
    padding: 24px 0 80px 24px;
}

.report-header {
    margin-bottom: 24px;
}

.report-header h1 {
    font-size: 24px;
    font-weight: 400;
    margin: 0 0 8px;
    color: var(--text-bright);
}

.report-subtitle {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
}

.report-section-anchor {
    margin-top: 32px;
    padding-top: 16px;
    border-top: 1px solid var(--border-subtle);
}

.report-section-anchor:first-of-type {
    border-top: none;
    margin-top: 0;
    padding-top: 0;
}

/* Footer */
.report-footer {
    border-top: 1px solid var(--border-visible);
    padding: 16px 0;
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
    margin-top: auto;
}

.report-footer a {
    color: var(--text-muted);
    text-decoration: none;
}

.report-footer a:hover {
    color: var(--text-dim);
}

/* Responsive */
@media (max-width: 960px) {
    .report-layout {
        display: block;
    }
    .report-sidebar {
        position: static;
        max-height: none;
        border-right: none;
        border-bottom: 1px solid var(--border-visible);
        margin-bottom: 16px;
        padding: 16px 0;
    }
    .report-main {
        padding-left: 0;
    }
}

/* Typography */
h1, h2, h3 {
    margin-bottom: 0.3em;
    margin-top: 0;
    font-weight: 400;
    color: var(--text-bright);
}

h2 {
    font-size: 18px;
}

h3 {
    font-size: 14px;
    color: var(--text-dim);
}

/* Tables */
table {
    border-collapse: collapse;
    width: 100%;
    margin: 12px 0;
    font-size: 12px;
}

th, td {
    border: 1px solid var(--border-visible);
    padding: 8px 10px;
    text-align: left;
}

th {
    background: var(--bg-mid);
    color: var(--text-dim);
    font-weight: 600;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
}

td {
    background: var(--bg-dark);
    color: var(--text-bright);
}

/* Code */
code {
    background: var(--bg-mid);
    color: var(--text-bright);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 11px;
}

/* Utility */
.muted {
    color: var(--text-muted);
}

.section-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 12px;
}

/* Pills */
.pill {
    background: var(--bg-mid);
    color: var(--text-dim);
    padding: 4px 10px;
    border-radius: 12px;
    font-size: 11px;
    border: 1px solid var(--border-visible);
}

/* Tabs */
.tab-bar {
    display: flex;
    gap: 6px;
    margin: 16px 0 8px 0;
    flex-wrap: wrap;
}

.tab-bar button {
    border: 1px solid var(--border-visible);
    background: var(--bg-dark);
    color: var(--text-dim);
    border-radius: 6px;
    padding: 6px 12px;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 400;
    transition: all 0.15s;
}

.tab-bar button:hover {
    border-color: var(--text-muted);
    color: var(--text-bright);
}

.tab-bar button.active {
    background: var(--accent-blue);
    color: #fff;
    border-color: var(--accent-blue);
}

.tab-content {
    display: none;
    padding: 12px 0;
}

.tab-content.active {
    display: block;
}

/* Graph */
.graph {
    height: 520px;
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    margin: 12px 0;
    background: var(--bg-dark);
}

.graph-toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin: 8px 0;
    font-size: 11px;
}

.graph-toolbar label {
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 6px;
}

.graph-toolbar input[type="text"],
.graph-toolbar input[type="number"],
.graph-toolbar select {
    background: var(--bg-mid);
    border: 1px solid var(--border-visible);
    color: var(--text-bright);
    padding: 4px 8px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 11px;
}

.graph-toolbar input[type="checkbox"] {
    accent-color: var(--accent-blue);
}

.graph-legend {
    display: flex;
    gap: 12px;
    font-size: 11px;
    color: var(--text-dim);
}

.legend-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    display: inline-block;
    margin-right: 4px;
}

.graph-hint {
    font-size: 11px;
    color: var(--text-muted);
    margin: 4px 0 8px;
}

.graph-empty {
    font-size: 12px;
    color: var(--text-muted);
    text-align: center;
    padding: 32px;
}

.graph-anchor {
    margin-top: 16px;
    font-size: 12px;
    color: var(--text-dim);
}

.graph-anchor .muted {
    display: block;
    margin-top: 4px;
}

/* Graph Controls */
.graph-controls {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
}

.graph-controls button {
    font-size: 11px;
    padding: 4px 10px;
    border: 1px solid var(--border-visible);
    background: var(--bg-mid);
    color: var(--text-dim);
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: all 0.15s;
}

.graph-controls button:hover {
    border-color: var(--text-muted);
    color: var(--text-bright);
}

/* Component Panel */
.component-panel {
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    padding: 12px 14px;
    margin: 12px 0;
    background: var(--bg-dark);
}

.component-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 8px;
}

.component-panel table {
    margin: 8px 0 0 0;
}

.component-panel .muted {
    font-size: 11px;
}

.component-chip {
    display: inline-block;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--bg-mid);
    color: var(--text-dim);
    font-size: 11px;
    border: 1px solid var(--border-subtle);
}

.component-panel .panel-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
}

.component-toolbar {
    margin-bottom: 8px;
}

.component-toolbar select,
.component-toolbar input[type="range"],
.component-toolbar input[type="number"] {
    font-size: 11px;
    background: var(--bg-mid);
    border: 1px solid var(--border-visible);
    color: var(--text-bright);
    padding: 4px 8px;
    border-radius: 4px;
}

/* Command Tables */
.command-table td {
    vertical-align: top;
}

.command-table th,
.command-table td {
    vertical-align: top;
}

.command-table code {
    background: transparent;
    color: inherit;
    font-weight: 600;
    padding: 0;
}

.command-pill {
    display: inline-block;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--bg-mid);
    color: var(--text-dim);
    font-size: 11px;
    margin: 2px 4px 2px 0;
    border: 1px solid var(--border-subtle);
}

.command-col {
    width: 50%;
}

.command-list {
    margin: 0;
    padding-left: 1.2rem;
    columns: 2;
    column-gap: 1.5rem;
    list-style: disc;
}

.command-list li {
    break-inside: avoid;
    word-break: break-word;
    margin-bottom: 6px;
    color: var(--text-dim);
}

.module-header {
    font-weight: 600;
    margin-top: 8px;
    color: var(--text-bright);
}

.module-group {
    margin-bottom: 12px;
}

/* AI Summary Panel */
.ai-summary-panel {
    background: var(--bg-dark);
    border: 1px solid var(--border-visible);
    border-radius: 8px;
    padding: 16px 20px;
    margin: 16px 0;
}

.ai-summary-panel h3 {
    margin: 0 0 12px;
    font-size: 14px;
    color: var(--text-bright);
}

.ai-summary-panel h4 {
    margin: 16px 0 8px;
    font-size: 12px;
    color: var(--text-dim);
}

.health-badge {
    display: inline-block;
    padding: 6px 12px;
    border-radius: 6px;
    font-weight: 600;
    font-size: 11px;
    margin-bottom: 12px;
}

.health-critical {
    background: rgba(220, 38, 38, 0.15);
    color: #f87171;
    border: 1px solid rgba(220, 38, 38, 0.3);
}

.health-warning {
    background: rgba(217, 119, 6, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(217, 119, 6, 0.3);
}

.health-debt {
    background: rgba(59, 130, 246, 0.15);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.3);
}

.health-good {
    background: rgba(5, 150, 105, 0.15);
    color: #34d399;
    border: 1px solid rgba(5, 150, 105, 0.3);
}

.summary-table {
    width: auto;
    margin: 8px 0;
}

.summary-table td {
    padding: 4px 16px 4px 0;
    border: none;
    font-size: 12px;
    background: transparent;
}

.summary-table .row-critical td {
    color: #f87171;
    font-weight: 600;
}

.summary-table .row-warning td {
    color: #fbbf24;
    font-weight: 600;
}

/* Quick Wins */
.quick-wins {
    margin-top: 16px;
}

.quick-wins ul {
    list-style: none;
    padding: 0;
    margin: 0;
}

.quick-wins li {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12px;
    flex-wrap: wrap;
}

.quick-wins li:last-child {
    border-bottom: none;
}

.quick-wins .action {
    font-weight: 600;
    min-width: 180px;
}

.quick-wins code {
    background: var(--bg-mid);
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
}

.quick-wins .location {
    color: var(--text-muted);
    font-size: 11px;
}

.quick-wins .impact {
    color: var(--accent-green);
    font-size: 11px;
    font-style: italic;
}

.priority-1 .action {
    color: #f87171;
}

.priority-2 .action {
    color: #fbbf24;
}

.priority-3 .action {
    color: #60a5fa;
}

.no-issues {
    color: var(--accent-green);
    font-style: italic;
    margin: 8px 0;
}
"#;

/// Content Security Policy header value
pub const CSP: &str = "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'none'; font-src 'self' data:;";
