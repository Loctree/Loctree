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
//! - Vista Galaxy Black Steel Theme (Space/Holographic)
//! - App-like layout (Fixed Sidebar + Scrollable Content)
//! - Monospace typography (JetBrains Mono / Inter)
//! - Responsive tables and graphs
//! - Tab navigation styling
//! - Cytoscape graph container styling

/// Content Security Policy for the report
/// Note: Permissive policy to allow local file:// viewing of reports
pub const CSP: &str = "default-src 'self' file: data: blob:; script-src 'self' 'unsafe-inline' 'unsafe-eval' file: data: blob:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' data: https://fonts.gstatic.com; img-src 'self' data: blob: file:; connect-src 'self';";

/// Complete CSS for the report.
pub const REPORT_CSS: &str = r#"
/* ============================================
   loctree Report — Vista Galaxy Black Steel
   ============================================ */

@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap');

/* ============================================
   Theme: Light Mode (Default)
   ============================================ */
:root {
    /* Light Theme Tokens */
    --theme-bg-deep: #f5f7fa;
    --theme-bg-surface: #ffffff;
    --theme-bg-surface-elevated: #fafbfc;

    --theme-text-primary: #1a1f26;
    --theme-text-secondary: #4a5568;
    --theme-text-tertiary: #718096;

    --theme-accent: #3182ce;
    --theme-accent-rgb: 49, 130, 206;

    --theme-border: rgba(0, 0, 0, 0.1);
    --theme-border-strong: rgba(0, 0, 0, 0.15);
    
    --theme-hover: rgba(0, 0, 0, 0.04);
    --theme-hover-strong: rgba(0, 0, 0, 0.08);

    /* Scrollbar (Light) */
    --theme-scrollbar: rgba(0, 0, 0, 0.15);
    --theme-scrollbar-hover: rgba(0, 0, 0, 0.25);
    /* Fallbacks for theme-aware scrollbars */
    --scrollbar-bg: var(--theme-scrollbar, rgba(0, 0, 0, 0.15));
    --scrollbar-bg-hover: var(--theme-scrollbar-hover, rgba(0, 0, 0, 0.25));

    /* Gradients (Light) */
    --gradient-nav: linear-gradient(135deg, rgba(255,255,255,0.98) 0%, rgba(245,247,250,0.95) 100%);
    --gradient-sidebar: linear-gradient(180deg, rgba(250,251,252,0.98) 0%, rgba(245,247,250,0.95) 100%);
    --gradient-main: linear-gradient(180deg, rgba(250,251,252,0.95) 0%, rgba(245,247,250,0.9) 100%);

    /* Dimensions */
    --radius-lg: 20px;
    --radius-md: 12px;
    --radius-sm: 6px;
    
    --sidebar-width: 280px;
    --header-height: 68px;
    
    --font-sans: 'Inter', system-ui, -apple-system, sans-serif;
    --font-mono: 'JetBrains Mono', monospace;
    
    color-scheme: light dark;
}

/* Tooltip safety layer */
.tooltip-floating {
    z-index: 9999 !important;
}

/* ============================================
   Theme: Dark Mode (Vista Galaxy Black Steel)
   ============================================ */
.dark,
html.dark {
    --theme-bg-deep: #0a0a0e;
    --theme-bg-surface: #14171c;
    --theme-bg-surface-elevated: #191d22;

    --theme-text-primary: #e5ecf5;
    --theme-text-secondary: #b2c0d4;
    --theme-text-tertiary: #8897ad;

    --theme-accent: #a3b8c7;
    --theme-accent-rgb: 163, 184, 199;

    --theme-border: rgba(114, 124, 139, 0.18);
    --theme-border-strong: rgba(114, 124, 139, 0.28);
    
    --theme-hover: rgba(255, 255, 255, 0.03);
    --theme-hover-strong: rgba(255, 255, 255, 0.06);

    /* Scrollbar (Dark) */
    --theme-scrollbar: rgba(255, 255, 255, 0.15);
    --theme-scrollbar-hover: rgba(255, 255, 255, 0.25);
    --scrollbar-bg: var(--theme-scrollbar, rgba(255, 255, 255, 0.15));
    --scrollbar-bg-hover: var(--theme-scrollbar-hover, rgba(255, 255, 255, 0.25));

    /* Gradients (Dark) */
    --gradient-nav: linear-gradient(135deg, rgba(10,10,14,0.95) 0%, rgba(32,36,44,0.85) 40%, rgba(120,132,144,0.15) 100%);
    --gradient-sidebar: linear-gradient(180deg, rgba(12,12,16,0.95) 0%, rgba(24,28,34,0.9) 100%);
    --gradient-main: linear-gradient(180deg, rgba(12,12,16,0.95) 0%, rgba(16,18,22,0.9) 55%, rgba(22,24,30,0.85) 100%);
}

/* Auto Dark Mode based on system preference */
@media (prefers-color-scheme: dark) {
    :root:not(.light) {
        --theme-bg-deep: #0a0a0e;
        --theme-bg-surface: #14171c;
        --theme-bg-surface-elevated: #191d22;

        --theme-text-primary: #e5ecf5;
        --theme-text-secondary: #b2c0d4;
        --theme-text-tertiary: #8897ad;

        --theme-accent: #a3b8c7;
        --theme-accent-rgb: 163, 184, 199;

        --theme-border: rgba(114, 124, 139, 0.18);
        --theme-border-strong: rgba(114, 124, 139, 0.28);
        
        --theme-hover: rgba(255, 255, 255, 0.03);
        --theme-hover-strong: rgba(255, 255, 255, 0.06);

        --gradient-nav: linear-gradient(135deg, rgba(10,10,14,0.95) 0%, rgba(32,36,44,0.85) 40%, rgba(120,132,144,0.15) 100%);
        --gradient-sidebar: linear-gradient(180deg, rgba(12,12,16,0.95) 0%, rgba(24,28,34,0.9) 100%);
        --gradient-main: linear-gradient(180deg, rgba(12,12,16,0.95) 0%, rgba(16,18,22,0.9) 55%, rgba(22,24,30,0.85) 100%);
    }
}

/* Reset & Base */
*, *::before, *::after { box-sizing: border-box; }

body {
    font-family: var(--font-sans);
    background: var(--theme-bg-deep);
    color: var(--theme-text-primary);
    line-height: 1.5;
    margin: 0;
    height: 100vh;
    overflow: hidden;
    font-size: 13px;
}

a { color: inherit; text-decoration: none; }
code, pre { font-family: var(--font-mono); }

/* Layout Shell */
.app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--theme-bg-deep);
}

/* Sidebar */
.app-sidebar {
    width: var(--sidebar-width);
    background: var(--gradient-sidebar);
    border-right: 1px solid var(--theme-border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    z-index: 20;
}

.sidebar-header {
    height: var(--header-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
    border-bottom: 1px solid var(--theme-border);
}

/* Theme Toggle Button */
.theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    background: var(--theme-hover);
    border: 1px solid var(--theme-border);
    color: var(--theme-text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    flex-shrink: 0;
}

.theme-toggle:hover {
    background: var(--theme-hover-strong);
    color: var(--theme-text-primary);
    border-color: var(--theme-border-strong);
}

/* Show sun icon in dark mode, moon icon in light mode */
.theme-icon-light { display: block; }
.theme-icon-dark { display: none; }

.dark .theme-icon-light,
html.dark .theme-icon-light { display: none; }
.dark .theme-icon-dark,
html.dark .theme-icon-dark { display: block; }

@media (prefers-color-scheme: dark) {
    :root:not(.light) .theme-icon-light { display: none; }
    :root:not(.light) .theme-icon-dark { display: block; }
}

.logo-box {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 600;
    font-size: 14px;
    color: var(--theme-text-primary);
    letter-spacing: 0.02em;
}

.logo-img {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
}

.logo-text {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
}

.sidebar-nav {
    flex: 1;
    overflow-y: auto;
    padding: 24px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
}

/* Nav items styled like tab buttons - unified design */
.nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    border-radius: var(--radius-lg);
    color: var(--theme-text-secondary);
    transition: all 0.2s ease;
    font-size: 13px;
    font-weight: 500;
    border: none;
    background: transparent;
    cursor: pointer;
    text-decoration: none;
    /* Prevent label overflow */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
}

.nav-item:hover {
    background: var(--theme-hover-strong);
    color: var(--theme-text-primary);
}

.nav-item.active {
    background: var(--theme-bg-surface);
    color: var(--theme-accent);
    box-shadow: 0 1px 3px rgba(0,0,0,0.12);
}

.nav-section-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--theme-text-tertiary);
    margin: 24px 14px 8px;
}

/* Main Area */
.app-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    position: relative;
    background: var(--gradient-main);
    min-width: 0; /* Prevent flex overflow */
}

/* Sticky Header */
.app-header {
    height: var(--header-height);
    flex-shrink: 0;
    background: var(--gradient-nav);
    border-bottom: 1px solid var(--theme-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 32px;
    backdrop-filter: blur(12px);
    z-index: 10;
}

.header-title h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.header-title p,
.header-path {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--theme-text-tertiary);
    font-family: var(--font-mono);
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Header Stats Badges */
.header-stats {
    display: flex;
    gap: 8px;
}

.stat-badge {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px 14px;
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    min-width: 60px;
}

.stat-badge-value {
    font-size: 16px;
    font-weight: 600;
    color: var(--theme-accent);
    font-family: var(--font-mono);
}

.stat-badge-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--theme-text-tertiary);
    margin-top: 2px;
}

/* Tabs */
.header-tabs {
    display: flex;
    gap: 6px;
    background: rgba(0,0,0,0.2);
    padding: 4px;
    border-radius: var(--radius-md);
    border: 1px solid var(--theme-border);
}

.tab-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 16px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--theme-text-secondary);
    cursor: pointer;
    transition: all 0.2s;
    background: transparent;
    border: none;
    /* Prevent label overflow */
    white-space: nowrap;
    flex-shrink: 0;
}

.tab-btn:hover {
    color: var(--theme-text-primary);
    background: var(--theme-hover);
}

.tab-btn.active {
    background: rgba(163, 184, 199, 0.15);
    color: var(--theme-accent);
    box-shadow: 0 1px 2px rgba(0,0,0,0.2);
}

/* Content Scroll Area */
.app-content {
    flex: 1;
    overflow-y: auto;
    padding: 32px;
    scroll-behavior: smooth;
}

/* Content Panels */
.content-container {
    max-width: 1100px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 24px;
}

.panel {
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-lg);
    padding: 24px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.2);
}

.panel h3 {
    margin-top: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--theme-text-primary);
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
}

/* Tables & Lists */
.data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
}

.data-table th {
    text-align: left;
    color: var(--theme-text-tertiary);
    font-weight: 500;
    padding: 12px 16px;
    border-bottom: 1px solid var(--theme-border);
}

.data-table td {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(114, 124, 139, 0.08);
    color: var(--theme-text-secondary);
}

.data-table tr:last-child td { border-bottom: none; }
.data-table tr:hover td { background: var(--theme-hover); }

code {
    background: rgba(0,0,0,0.2);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--theme-accent);
    font-size: 0.9em;
}

/* Analysis Summary */
.analysis-summary {
    margin-bottom: 24px;
}

.analysis-summary h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
}

.summary-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 16px;
}

.summary-stat {
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    padding: 16px;
    text-align: center;
}

.stat-value {
    display: block;
    font-size: 28px;
    font-weight: 600;
    color: var(--theme-accent);
    margin-bottom: 4px;
}

.stat-label {
    display: block;
    font-size: 12px;
    color: var(--theme-text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

/* Command Coverage Summary */
.coverage-summary {
    padding: 12px 16px;
    background: var(--theme-bg-surface-elevated);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    margin-bottom: 16px;
    font-size: 13px;
}

.text-warning {
    color: #e67e22;
}

.text-muted {
    color: var(--theme-text-tertiary);
}

/* AI Insights */
.insight-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.insight-item {
    padding: 16px;
    border-radius: var(--radius-md);
    background: var(--theme-hover);
    border: 1px solid var(--theme-border);
    display: flex;
    gap: 12px;
}

.insight-icon { flex-shrink: 0; margin-top: 2px; }
.insight-content strong { display: block; margin-bottom: 4px; color: var(--theme-text-primary); }
.insight-content p { margin: 0; color: var(--theme-text-secondary); line-height: 1.5; }

/* Graph */
.graph-wrapper {
    width: 100%;
    height: calc(100vh - var(--header-height) - 64px);
    background: var(--theme-bg-deep);
    border-radius: var(--radius-lg);
    border: 1px solid var(--theme-border);
    overflow: hidden;
    position: relative;
}

#cy { width: 100%; height: 100%; }

/* Scrollbar - theme-aware */
::-webkit-scrollbar { width: 8px; height: 8px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--theme-scrollbar, rgba(114, 124, 139, 0.2)); border-radius: 4px; }
::-webkit-scrollbar-thumb:hover { background: var(--theme-scrollbar-hover, rgba(114, 124, 139, 0.4)); }

/* Firefox scrollbar */
* {
    scrollbar-width: thin;
    scrollbar-color: var(--theme-scrollbar, rgba(114, 124, 139, 0.2)) transparent;
}

/* Footer */
.app-footer {
    margin-top: auto;
    padding: 24px 16px;
    text-align: center;
    color: var(--theme-text-tertiary);
    font-size: 11px;
    border-top: 1px solid var(--theme-border);
}

/* ============================================
   Section & Tab Visibility (CRITICAL)
   ============================================ */

/* Section views - only show active */
.section-view {
    display: none;
    height: 100%;
    flex-direction: column;
}

.section-view.active {
    display: flex;
}

/* Tab panels - only show active */
.tab-panel {
    display: none;
}

.tab-panel.active {
    display: block;
}

/* Tab bar alias for JS selector */
.tab-bar {
    /* Inherits from .header-tabs */
}

/* ============================================
   Graph Container & Toolbars
   ============================================ */

/* ============================================
   Graph Split Layout (Side-by-Side)
   ============================================ */

.graph-split-container {
    display: flex;
    height: calc(100vh - var(--header-height) - 32px);
    gap: 0;
    position: relative;
}

.graph-left-panel {
    width: 380px;
    min-width: 280px;
    max-width: 600px;
    display: flex;
    flex-direction: column;
    background: var(--theme-bg-surface);
    border-right: 1px solid var(--theme-border);
    overflow: hidden;
}

.graph-left-panel .component-panel {
    flex: 1;
    overflow-y: auto;
    margin: 0;
    border: none;
    border-radius: 0;
}

.graph-left-panel .component-panel-header {
    position: sticky;
    top: 0;
    z-index: 5;
    padding: 8px 10px;
    font-size: 11px;
}

/* Compact table for left panel */
.graph-left-panel .component-panel table {
    font-size: 11px;
}

.graph-left-panel .component-panel th {
    padding: 6px 8px;
    font-size: 10px;
}

.graph-left-panel .component-panel td {
    padding: 4px 8px;
}

.graph-left-panel .component-panel code {
    font-size: 10px;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: inline-block;
}

.graph-left-panel .component-toolbar {
    padding: 6px 10px;
    font-size: 11px;
    flex-wrap: wrap;
    gap: 6px;
}

.graph-left-panel .component-toolbar label {
    font-size: 10px;
}

.graph-left-panel .component-toolbar button {
    padding: 3px 6px;
    font-size: 10px;
}

.graph-left-panel .panel-actions {
    gap: 6px;
}

.graph-left-panel .panel-actions label {
    font-size: 10px;
}

.graph-left-panel .panel-actions input {
    padding: 2px 4px;
    width: 50px !important;
}

.graph-right-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 400px;
    overflow: hidden;
}

.graph-right-panel .graph-toolbar {
    flex-shrink: 0;
    margin: 0;
    border-radius: 0;
    border-left: none;
    border-right: none;
}

.graph-right-panel .graph {
    flex: 1;
    min-height: 0;
    border-radius: 0;
    border: none;
    border-top: 1px solid var(--theme-border);
}

/* Resize handle */
.graph-resize-handle {
    width: 6px;
    cursor: col-resize;
    background: var(--theme-border);
    transition: background 0.15s;
    flex-shrink: 0;
}

.graph-resize-handle:hover,
.graph-resize-handle.active {
    background: var(--theme-accent);
}

/* Graph container - fallback for non-split */
.graph {
    width: 100%;
    height: calc(100vh - var(--header-height) - 200px);
    min-height: 400px;
    background: var(--theme-bg-deep);
    border-radius: var(--radius-md);
    border: 1px solid var(--theme-border);
}

.graph-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--theme-text-tertiary);
    font-style: italic;
    background: var(--theme-bg-surface);
    border-radius: var(--radius-md);
    border: 1px dashed var(--theme-border);
}

/* Graph toolbars */
.graph-toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    margin-bottom: 12px;
    font-size: 12px;
}

.graph-toolbar label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--theme-text-secondary);
}

.graph-toolbar input[type="text"],
.graph-toolbar input[type="number"],
.graph-toolbar select {
    background: var(--theme-bg-deep);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    padding: 4px 8px;
    color: var(--theme-text-primary);
    font-size: 12px;
    font-family: var(--font-mono);
}

.graph-toolbar input[type="checkbox"] {
    accent-color: var(--theme-accent);
}

.graph-toolbar button {
    background: var(--theme-bg-surface-elevated);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    color: var(--theme-text-secondary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
}

.graph-toolbar button:hover {
    background: rgba(163, 184, 199, 0.1);
    color: var(--theme-text-primary);
    border-color: var(--theme-accent);
}

.component-toolbar {
    background: var(--theme-bg-surface-elevated);
}

.graph-controls {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-left: auto;
}

/* Graph legend */
.graph-legend {
    display: flex;
    gap: 16px;
    padding: 8px 0;
    font-size: 11px;
    color: var(--theme-text-tertiary);
}

.graph-legend span {
    display: flex;
    align-items: center;
    gap: 6px;
}

.legend-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    display: inline-block;
}

/* Graph hint */
.graph-hint {
    padding: 12px 16px;
    background: rgba(163, 184, 199, 0.05);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    font-size: 12px;
    color: var(--theme-text-tertiary);
    margin-top: 12px;
}

/* ============================================
   Component Panel (Disconnected Components)
   ============================================ */

.component-panel {
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    margin-bottom: 12px;
    overflow: hidden;
}

.component-panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: var(--theme-bg-surface-elevated);
    border-bottom: 1px solid var(--theme-border);
    font-size: 13px;
}

.component-panel-header strong {
    color: var(--theme-text-primary);
}

.panel-actions {
    display: flex;
    align-items: center;
    gap: 12px;
}

.panel-actions label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--theme-text-secondary);
}

.panel-actions input {
    background: var(--theme-bg-deep);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    padding: 4px 8px;
    color: var(--theme-text-primary);
    font-size: 12px;
}

.panel-actions button {
    background: var(--theme-bg-deep);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    color: var(--theme-text-secondary);
    font-size: 11px;
    cursor: pointer;
}

.panel-actions button:hover {
    border-color: var(--theme-accent);
    color: var(--theme-text-primary);
}

.component-panel table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
}

.component-panel th {
    text-align: left;
    padding: 10px 16px;
    color: var(--theme-text-tertiary);
    font-weight: 500;
    border-bottom: 1px solid var(--theme-border);
    background: var(--theme-bg-surface);
}

.component-panel td {
    padding: 10px 16px;
    color: var(--theme-text-secondary);
    border-bottom: 1px solid rgba(114, 124, 139, 0.08);
}

.component-panel tr:hover td {
    background: var(--theme-hover);
}

.component-panel button {
    background: transparent;
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    padding: 3px 8px;
    color: var(--theme-text-tertiary);
    font-size: 10px;
    cursor: pointer;
}

.component-panel button:hover {
    border-color: var(--theme-accent);
    color: var(--theme-accent);
}

/* ============================================
   Tauri Command Tables
   ============================================ */

.command-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    margin-top: 16px;
}

.command-table th {
    text-align: left;
    padding: 12px 16px;
    color: var(--theme-text-tertiary);
    font-weight: 500;
    border-bottom: 1px solid var(--theme-border);
}

.command-table td {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(114, 124, 139, 0.08);
    color: var(--theme-text-secondary);
}

.command-pill {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 12px;
    background: rgba(163, 184, 199, 0.1);
    color: var(--theme-accent);
}

/* Module grouping */
.module-group {
    margin-bottom: 24px;
}

.module-header {
    font-size: 13px;
    font-weight: 500;
    color: var(--theme-text-secondary);
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--theme-border);
}

/* FE↔BE Bridge Comparison Table */
.bridge-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
    margin-top: 16px;
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    overflow: hidden;
}

.bridge-table thead th {
    text-align: left;
    padding: 10px 14px;
    color: var(--theme-text-tertiary);
    font-weight: 500;
    background: var(--theme-bg-surface-elevated);
    border-bottom: 1px solid var(--theme-border);
}

.bridge-table tbody td {
    padding: 8px 14px;
    border-bottom: 1px solid rgba(114, 124, 139, 0.08);
    color: var(--theme-text-secondary);
    vertical-align: top;
}

.bridge-table tbody tr:last-child td {
    border-bottom: none;
}

.bridge-table tbody tr:hover td {
    background: var(--theme-hover);
}

.bridge-table .status-cell {
    font-weight: 500;
    white-space: nowrap;
}

.bridge-table .loc-cell {
    font-family: var(--font-mono);
    font-size: 11px;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
}

.bridge-table .loc-cell a {
    color: var(--theme-accent);
}

.bridge-table .loc-cell a:hover {
    text-decoration: underline;
}

/* Bridge row status colors */
.bridge-table tr.status-ok .status-cell {
    color: #27ae60;
}

.bridge-table tr.status-missing .status-cell {
    color: #e67e22;
}

.bridge-table tr.status-unused .status-cell {
    color: var(--theme-text-tertiary);
}

.bridge-table tr.status-unregistered .status-cell {
    color: #c0392b;
}

.bridge-table tr.status-missing {
    background: rgba(230, 126, 34, 0.05);
}

.bridge-table tr.status-unregistered {
    background: rgba(192, 57, 43, 0.05);
}

/* Gap details toggle */
.gap-details {
    margin-top: 24px;
}

.gap-details summary {
    cursor: pointer;
    color: var(--theme-text-tertiary);
    font-size: 12px;
    padding: 8px 0;
}

.gap-details summary:hover {
    color: var(--theme-text-secondary);
}

/* Text success color */
.text-success {
    color: #27ae60;
}

/* ============================================
   Utility Classes
   ============================================ */

.muted {
    color: var(--theme-text-tertiary);
}

.icon-sm {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
}

/* Range slider styling */
input[type="range"] {
    -webkit-appearance: none;
    background: var(--theme-bg-deep);
    border-radius: 4px;
    height: 6px;
    cursor: pointer;
}

input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    background: var(--theme-accent);
    border-radius: 50%;
    cursor: pointer;
}

/* ============================================
   Quick Commands Panel (v0.6 features)
   ============================================ */

.quick-commands-panel {
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-lg);
    padding: 20px 24px;
    margin-top: 8px;
}

.quick-commands-panel h3 {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 16px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.badge-new {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    background: linear-gradient(135deg, rgba(163, 184, 199, 0.2) 0%, rgba(79, 129, 225, 0.2) 100%);
    color: var(--theme-accent);
    padding: 3px 8px;
    border-radius: 6px;
    border: 1px solid rgba(163, 184, 199, 0.3);
}

.commands-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 16px;
}

.command-group {
    background: var(--theme-bg-surface-elevated);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    padding: 16px;
}

.command-group.highlight {
    border-color: rgba(163, 184, 199, 0.3);
    background: linear-gradient(135deg, var(--theme-bg-surface-elevated) 0%, rgba(163, 184, 199, 0.05) 100%);
}

.command-group h4 {
    margin: 0 0 6px 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.command-group .command-desc {
    margin: 0 0 12px 0;
    font-size: 11px;
    color: var(--theme-text-tertiary);
}

.command-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.command-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: var(--theme-bg-deep);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    font-size: 11px;
}

.command-item:hover {
    border-color: var(--theme-border-strong);
}

.command-code {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    background: transparent;
    padding: 0;
}

.command-desc-inline {
    font-size: 10px;
    color: var(--theme-text-tertiary);
    white-space: nowrap;
}

.copy-btn {
    flex-shrink: 0;
    background: transparent;
    border: none;
    padding: 2px 4px;
    cursor: pointer;
    font-size: 12px;
    opacity: 0.6;
    transition: opacity 0.15s;
}

.copy-btn:hover {
    opacity: 1;
}

.commands-footer {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--theme-border);
}

.commands-footer p {
    margin: 0;
    font-size: 11px;
    color: var(--theme-text-tertiary);
}

.commands-footer code {
    font-size: 10px;
    background: var(--theme-bg-deep);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--theme-accent);
}

/* ============================================
   Tree Component Styles
   ============================================ */

.tree-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.tree-header {
    display: flex;
    align-items: center;
    gap: 12px;
}

.tree-header h3 {
    margin: 0;
    white-space: nowrap;
}

.tree-controls {
    display: flex;
    gap: 4px;
}

.tree-btn {
    padding: 6px 10px;
    border: 1px solid var(--theme-border);
    border-radius: 6px;
    background: var(--theme-surface);
    color: var(--theme-text);
    cursor: pointer;
    font-size: 14px;
    transition: all 0.15s ease;
}

.tree-btn:hover {
    background: var(--theme-bg-surface-elevated);
    border-color: var(--theme-border-strong);
}

.tree-filter {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid var(--theme-border);
    border-radius: 8px;
    background: var(--theme-surface);
    color: var(--theme-text);
    font-size: 13px;
}

.tree-filter:focus {
    outline: none;
    border-color: var(--theme-accent);
}

.tree-container {
    max-height: 600px;
    overflow-y: auto;
    padding-right: 8px;
}

.tree-node {
    font-family: "JetBrains Mono", "SFMono-Regular", monospace;
    font-size: 12px;
}

.tree-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: default;
    transition: background 0.1s ease;
}

.tree-row:hover {
    background: var(--theme-bg-surface-elevated);
}

.tree-row-dir {
    cursor: pointer;
}

.tree-row-dir:hover {
    background: rgba(var(--theme-accent-rgb), 0.1);
}

.tree-left {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    flex: 1;
}

.tree-connector {
    color: var(--theme-text-tertiary);
    white-space: pre;
    flex-shrink: 0;
}

.tree-chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    font-size: 10px;
    color: var(--theme-text-secondary);
    transition: transform 0.15s ease;
    flex-shrink: 0;
}

.tree-chevron.collapsed {
    transform: rotate(0deg);
}

.tree-chevron:not(.collapsed) {
    transform: rotate(90deg);
}

.tree-icon {
    flex-shrink: 0;
    font-size: 14px;
}

.tree-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--theme-text);
}

.tree-highlight {
    background: rgba(255, 200, 0, 0.3);
    color: inherit;
    padding: 0 2px;
    border-radius: 2px;
}

.tree-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    margin-left: 12px;
}

.tree-loc-bar {
    width: 60px;
    height: 4px;
    background: var(--theme-border);
    border-radius: 2px;
    overflow: hidden;
}

.tree-loc-fill {
    height: 100%;
    background: var(--theme-accent);
    border-radius: 2px;
    transition: width 0.2s ease;
}

.tree-loc {
    color: var(--theme-text-tertiary);
    font-size: 11px;
    min-width: 60px;
    text-align: right;
}

.tree-children {
    overflow: hidden;
    transition: max-height 0.2s ease, opacity 0.15s ease;
}

.tree-children.collapsed {
    max-height: 0 !important;
    opacity: 0;
    pointer-events: none;
}

/* ============================================
   Crowds Component Styles
   ============================================ */

.crowds-list {
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-top: 16px;
}

.crowd-card {
    background: var(--theme-bg-surface-elevated);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-lg);
    padding: 20px;
    transition: border-color 0.2s ease;
}

.crowd-card:hover {
    border-color: var(--theme-border-strong);
}

.crowd-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--theme-border);
}

.crowd-pattern {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
}

.crowd-pattern code {
    font-size: 14px;
    font-weight: 600;
    color: var(--theme-accent);
    background: rgba(163, 184, 199, 0.1);
    padding: 6px 12px;
    border-radius: var(--radius-md);
}

.crowd-member-count {
    font-size: 12px;
}

.crowd-score {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px 16px;
    background: var(--theme-bg-deep);
    border-radius: var(--radius-md);
    border: 2px solid var(--score-color, var(--theme-border));
    min-width: 80px;
}

.score-value {
    font-size: 24px;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--score-color, var(--theme-text-primary));
    line-height: 1;
}

.score-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--theme-text-tertiary);
    margin-top: 4px;
}

.crowd-issues {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 16px;
}

.issue-badge {
    display: inline-block;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-weight: 500;
    border: 1px solid;
}

.issue-critical {
    background: rgba(192, 57, 43, 0.1);
    border-color: rgba(192, 57, 43, 0.3);
    color: #c0392b;
}

.issue-warning {
    background: rgba(230, 126, 34, 0.1);
    border-color: rgba(230, 126, 34, 0.3);
    color: #e67e22;
}

.issue-info {
    background: rgba(49, 130, 206, 0.1);
    border-color: rgba(49, 130, 206, 0.3);
    color: #3182ce;
}

.crowd-members {
    margin-top: 12px;
}

.crowd-members .data-table {
    font-size: 12px;
}

.crowd-members .data-table th {
    padding: 8px 12px;
    font-size: 11px;
}

.crowd-members .data-table td {
    padding: 8px 12px;
}

.file-path {
    max-width: 400px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: inline-block;
}

/* ============================================
   Dead Code Component Styles
   ============================================ */

.dead-code-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: var(--theme-bg-surface-elevated);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    margin-bottom: 16px;
}

.filter-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--theme-text-secondary);
    cursor: pointer;
}

.filter-toggle input[type="checkbox"] {
    accent-color: var(--theme-accent);
    cursor: pointer;
}

.dead-exports-table {
    font-size: 13px;
}

.dead-exports-table .file-cell code,
.dead-exports-table .symbol-cell code {
    font-family: var(--font-mono);
    font-size: 12px;
}

.dead-exports-table .file-cell a {
    color: var(--theme-accent);
    text-decoration: none;
}

.dead-exports-table .file-cell a:hover {
    text-decoration: underline;
}

.dead-exports-table .line-cell {
    font-family: var(--font-mono);
    font-size: 11px;
    text-align: center;
    color: var(--theme-text-tertiary);
}

.dead-exports-table .confidence-cell {
    text-align: center;
}

.confidence-badge {
    display: inline-block;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.confidence-badge.confidence-very-high {
    background: rgba(192, 57, 43, 0.15);
    color: #c0392b;
    border: 1px solid rgba(192, 57, 43, 0.3);
}

.confidence-badge.confidence-high {
    background: rgba(230, 126, 34, 0.15);
    color: #e67e22;
    border: 1px solid rgba(230, 126, 34, 0.3);
}

.confidence-badge.confidence-medium {
    background: rgba(49, 130, 206, 0.15);
    color: #3182ce;
    border: 1px solid rgba(49, 130, 206, 0.3);
}

.dead-exports-table .reason-cell {
    font-size: 12px;
    max-width: 300px;
    color: var(--theme-text-secondary);
}

.dead-exports-table tr.confidence-very-high {
    background: rgba(192, 57, 43, 0.03);
}

.dead-exports-table tr.confidence-high {
    background: rgba(230, 126, 34, 0.03);
}

.dead-exports-table tr:hover {
    background: var(--theme-hover-strong) !important;
}

/* ============================================
   Cycles Component
   ============================================ */

/* Count badges */
.count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 20px;
    padding: 0 8px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    margin-left: auto;
}

.count-badge-success {
    background: rgba(39, 174, 96, 0.15);
    color: #27ae60;
    border: 1px solid rgba(39, 174, 96, 0.3);
}

.count-badge-warning {
    background: rgba(230, 126, 34, 0.15);
    color: #e67e22;
    border: 1px solid rgba(230, 126, 34, 0.3);
}

.count-badge-critical {
    background: rgba(192, 57, 43, 0.15);
    color: #c0392b;
    border: 1px solid rgba(192, 57, 43, 0.3);
}

/* Empty state */
.cycles-empty {
    padding: 32px;
    text-align: center;
    background: rgba(39, 174, 96, 0.05);
    border-radius: var(--radius-md);
    border: 1px dashed rgba(39, 174, 96, 0.3);
}

.cycles-empty p {
    color: #27ae60;
    font-size: 13px;
    margin: 0;
}

/* Cycles section */
.cycles-section {
    margin-bottom: 24px;
    padding: 20px;
    border-radius: var(--radius-md);
    border: 1px solid var(--theme-border);
}

.cycles-section-strict {
    background: rgba(192, 57, 43, 0.05);
    border-color: rgba(192, 57, 43, 0.2);
}

.cycles-section-lazy {
    background: rgba(230, 126, 34, 0.05);
    border-color: rgba(230, 126, 34, 0.2);
}

.cycles-section-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
}

.cycles-section-header h4 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.cycles-section-desc {
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0 0 16px 0;
    padding-left: 30px;
}

/* Cycles list */
.cycles-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

/* Individual cycle item */
.cycle-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: var(--theme-bg-surface);
    border-radius: var(--radius-md);
    border: 1px solid var(--theme-border);
}

.cycle-item-strict {
    border-left: 3px solid #c0392b;
}

.cycle-item-lazy {
    border-left: 3px solid #e67e22;
}

.cycle-number {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    height: 24px;
    padding: 0 8px;
    background: var(--theme-hover);
    border: 1px solid var(--theme-border);
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--theme-text-tertiary);
    flex-shrink: 0;
}

.cycle-path {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: rgba(0, 0, 0, 0.2);
    padding: 4px 8px;
    border-radius: 4px;
}

/* ============================================
   Responsive
   ============================================ */

@media (max-width: 900px) {
    .app-shell {
        flex-direction: column;
    }
    
    .app-sidebar {
        width: 100%;
        height: auto;
        border-right: none;
        border-bottom: 1px solid var(--theme-border);
    }
    
    .sidebar-nav {
        flex-direction: row;
        overflow-x: auto;
        padding: 12px;
    }
    
    .nav-section-title {
        display: none;
    }
    
    .app-footer {
        display: none;
    }
    
    .header-tabs {
        flex-wrap: wrap;
    }
    
    .graph-toolbar {
        flex-direction: column;
        align-items: stretch;
    }
    
    .graph-controls {
        margin-left: 0;
        justify-content: center;
    }
}
"#;
