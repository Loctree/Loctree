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

/* Test Toggle Button */
.test-toggle-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--theme-hover);
    border: 1px solid var(--theme-border);
    color: var(--theme-text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 12px;
    font-weight: 500;
}

.test-toggle-btn:hover {
    background: var(--theme-hover-strong);
    color: var(--theme-text-primary);
    border-color: var(--theme-border-strong);
}

#test-toggle-icon {
    font-size: 16px;
    transition: opacity 0.2s ease;
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

/* ============================================
   Action Plan Panel
   ============================================ */

.action-plan-panel .action-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.action-plan-panel .action-item {
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
    padding: 12px 14px;
    background: var(--theme-bg-surface-elevated);
}

.action-plan-panel .action-head {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 12px;
}

.action-plan-panel .action-priority {
    background: rgba(var(--theme-accent-rgb), 0.12);
    color: var(--theme-accent);
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 600;
}

.action-plan-panel .action-kind {
    color: var(--theme-text-tertiary);
}

.action-plan-panel .action-risk {
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 700;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.4px;
}

.action-plan-panel .risk-high {
    background: rgba(192, 57, 43, 0.12);
    color: #c0392b;
}

.action-plan-panel .risk-medium {
    background: rgba(230, 126, 34, 0.12);
    color: #e67e22;
}

.action-plan-panel .risk-low {
    background: rgba(39, 174, 96, 0.12);
    color: #27ae60;
}

.action-plan-panel .action-why,
.action-plan-panel .action-fix,
.action-plan-panel .action-verify,
.action-plan-panel .action-location {
    margin-top: 6px;
    font-size: 12px;
    color: var(--theme-text-secondary);
}

.action-plan-panel .action-label {
    display: inline-block;
    min-width: 52px;
    color: var(--theme-text-tertiary);
    font-weight: 700;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.3px;
    margin-right: 6px;
}

.action-plan-panel .action-verify code {
    margin-right: 6px;
}

/* ============================================
   Hub Files Panel
   ============================================ */

.hub-files-panel .hub-table code {
    font-size: 11px;
}

.hub-files-panel .hub-table td {
    vertical-align: top;
}

.hub-files-panel .hub-table .copy-btn {
    margin-left: 6px;
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

.tree-stats {
    font-size: 13px;
    color: var(--theme-text-muted);
    padding: 4px 10px;
    background: var(--theme-surface);
    border-radius: 12px;
    border: 1px solid var(--theme-border);
    cursor: help;
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
    max-height: calc(100vh - 280px);
    min-height: 400px;
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
   Pipeline Component Styles
   ============================================ */

.pipelines-panel {
    max-width: 100%;
}

.pipelines-summary {
    margin-bottom: 16px;
}

.pipeline-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}

.stat-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
}

.stat-total {
    background: var(--theme-bg-surface-elevated);
    color: var(--theme-text-secondary);
    border: 1px solid var(--theme-border);
}

.stat-ok {
    background: rgba(39, 174, 96, 0.15);
    color: #27ae60;
}

.stat-missing {
    background: rgba(231, 76, 60, 0.15);
    color: #e74c3c;
}

.stat-unused {
    background: rgba(149, 165, 166, 0.2);
    color: #95a5a6;
}

.stat-unreg {
    background: rgba(230, 126, 34, 0.15);
    color: #e67e22;
}

.pipelines-filters {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 20px;
    padding: 12px 16px;
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-md);
}

.filter-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
}

.filter-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    background: var(--theme-bg-surface);
    color: var(--theme-text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
}

.filter-btn:hover {
    background: var(--theme-bg-surface-elevated);
    border-color: var(--theme-primary);
}

.filter-btn.active {
    background: var(--theme-primary);
    border-color: var(--theme-primary);
    color: white;
}

.filter-count {
    padding: 2px 6px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    font-size: 10px;
}

.filter-btn:not(.active) .filter-count {
    background: rgba(0, 0, 0, 0.1);
}

.search-box {
    flex: 0 0 auto;
}

.search-input {
    padding: 8px 12px;
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    background: var(--theme-bg-surface);
    color: var(--theme-text-primary);
    font-size: 13px;
    min-width: 200px;
}

.search-input:focus {
    outline: none;
    border-color: var(--theme-primary);
    box-shadow: 0 0 0 2px rgba(52, 152, 219, 0.2);
}

.search-input::placeholder {
    color: var(--theme-text-tertiary);
}

.no-results {
    text-align: center;
    padding: 32px;
}

.cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
}

.pipeline-card {
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: all 0.2s ease;
}

.pipeline-card:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.pipeline-card.status-ok {
    border-left: 3px solid #27ae60;
}

.pipeline-card.status-missing {
    border-left: 3px solid #e74c3c;
}

.pipeline-card.status-unused {
    border-left: 3px solid #95a5a6;
}

.pipeline-card.status-unreg {
    border-left: 3px solid #e67e22;
}

.card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    cursor: pointer;
    background: var(--theme-bg-surface-elevated);
}

.card-header:hover {
    background: var(--theme-bg-hover);
}

.card-title {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
}

.command-name {
    font-family: var(--font-mono);
    font-size: 14px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 500;
}

.status-badge.status-ok {
    background: rgba(39, 174, 96, 0.15);
    color: #27ae60;
}

.status-badge.status-missing {
    background: rgba(231, 76, 60, 0.15);
    color: #e74c3c;
}

.status-badge.status-unused {
    background: rgba(149, 165, 166, 0.2);
    color: #7f8c8d;
}

.status-badge.status-unreg {
    background: rgba(230, 126, 34, 0.15);
    color: #e67e22;
}

.expand-icon {
    color: var(--theme-text-tertiary);
    font-size: 12px;
    transition: transform 0.2s ease;
}

/* Chain Visualization */
.chain-viz {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 16px;
    background: var(--theme-bg-surface);
}

.chain-node {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 16px;
    border-radius: var(--radius-md);
    min-width: 80px;
    transition: all 0.2s ease;
}

.chain-node.active {
    background: var(--theme-bg-surface-elevated);
    border: 1px solid var(--theme-border);
}

.chain-node.inactive {
    background: rgba(0, 0, 0, 0.05);
    border: 1px dashed var(--theme-border);
    opacity: 0.6;
}

.chain-node.fe.active {
    border-color: #3498db;
    background: rgba(52, 152, 219, 0.1);
}

.chain-node.be.active {
    border-color: #9b59b6;
    background: rgba(155, 89, 182, 0.1);
}

.node-icon {
    font-size: 12px;
    font-weight: 700;
    padding: 4px 8px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.1);
}

.chain-node.fe .node-icon {
    background: #3498db;
    color: white;
}

.chain-node.be .node-icon {
    background: #9b59b6;
    color: white;
}

.chain-node.inactive .node-icon {
    background: var(--theme-text-tertiary);
    color: white;
}

.node-label {
    font-size: 11px;
    color: var(--theme-text-secondary);
    text-align: center;
}

.chain-arrow {
    display: flex;
    align-items: center;
    color: var(--theme-border);
    font-size: 14px;
}

.chain-arrow.active {
    color: var(--theme-primary);
}

.arrow-line {
    display: block;
    width: 20px;
    height: 2px;
    background: currentColor;
}

.arrow-head {
    font-weight: bold;
}

/* Card Details (Expanded) */
.card-details {
    padding: 16px;
    border-top: 1px solid var(--theme-border);
    background: var(--theme-bg-surface);
}

.detail-section {
    margin-bottom: 16px;
}

.detail-section:last-child {
    margin-bottom: 0;
}

.detail-section h4 {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--theme-text-secondary);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.location-list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.location-list li {
    padding: 6px 10px;
    background: var(--theme-bg-surface-elevated);
    border-radius: var(--radius-sm);
    margin-bottom: 4px;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 4px;
}

.location-list .file-path {
    font-family: var(--font-mono);
    color: var(--theme-primary);
}

.location-list .line-num {
    color: var(--theme-text-tertiary);
    font-family: var(--font-mono);
}

.location-list .impl-name {
    color: var(--theme-text-tertiary);
    font-size: 11px;
}

.card-details .warning {
    color: #e67e22;
    font-size: 12px;
}

/* ============================================
   Split Panel View (FE/BE Side-by-Side)
   ============================================ */

.split-panel-container {
    display: grid;
    grid-template-columns: 1fr 80px 1fr;
    gap: 0;
    min-height: 400px;
    margin-top: 16px;
}

.split-panel {
    background: var(--theme-bg-surface);
    border-radius: var(--radius-md);
    padding: 16px;
    overflow-y: auto;
    max-height: 600px;
    border: 1px solid var(--theme-border);
}

.split-panel h4 {
    margin: 0 0 12px 0;
    font-size: 14px;
    color: var(--theme-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.panel-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.split-item {
    background: var(--theme-bg-surface-elevated);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    cursor: pointer;
    transition: border-color 0.2s;
}

.split-item:hover {
    border-color: var(--theme-accent);
}

.split-item.status-ok {
    border-left: 3px solid #27ae60;
}

.split-item.status-missing {
    border-left: 3px solid #e74c3c;
}

.split-item.status-unused {
    border-left: 3px solid #95a5a6;
}

.split-item-name {
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.split-item-location {
    font-size: 11px;
    color: var(--theme-text-tertiary);
    margin-top: 4px;
}

.connection-svg {
    width: 80px;
    height: 100%;
    min-height: 400px;
}

.connection-line {
    stroke: var(--theme-accent);
    stroke-width: 2;
    fill: none;
}

.connection-line.missing {
    stroke: #e74c3c;
    stroke-dasharray: 4;
}

/* Split panel specific styles */
.split-panel-fe {
    border-right: none;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
}

.split-panel-be {
    border-left: none;
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
}

.split-panel-connections {
    display: flex;
    align-items: stretch;
    background: var(--theme-bg-surface);
    border-top: 1px solid var(--theme-border);
    border-bottom: 1px solid var(--theme-border);
}

.split-panel h4 {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--theme-border);
}

.split-item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.split-item-status {
    display: flex;
    align-items: center;
}

.split-item-status.status-ok {
    color: #27ae60;
}

.split-item-status.status-missing {
    color: #e74c3c;
}

.split-item-status.status-unused {
    color: #95a5a6;
}

.split-item-status.status-unreg {
    color: #e67e22;
}

.split-item.status-unreg {
    border-left: 3px solid #e67e22;
}

.split-item-placeholder {
    background: rgba(231, 76, 60, 0.05);
    border-style: dashed;
}

.split-item-placeholder .split-item-name {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #e74c3c;
}

.split-item-location a {
    color: var(--theme-accent);
}

.split-item-location a:hover {
    text-decoration: underline;
}

/* View toggle buttons */
.view-toggle {
    display: flex;
    gap: 4px;
    margin-left: auto;
}

.view-btn {
    padding: 6px 10px;
    border: 1px solid var(--theme-border);
    background: var(--theme-bg-surface);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 14px;
    color: var(--theme-text-secondary);
    transition: all 0.15s ease;
}

.view-btn:hover {
    background: var(--theme-bg-surface-elevated);
    border-color: var(--theme-border-strong);
}

.view-btn.active {
    background: var(--theme-accent);
    color: white;
    border-color: var(--theme-accent);
}

/* Communication type badge */
.comm-badge {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
    margin-left: 8px;
}

.comm-badge.comm-emit {
    background: rgba(155, 89, 182, 0.2);
    color: #9b59b6;
}

.comm-badge.comm-invoke {
    background: rgba(52, 152, 219, 0.2);
    color: #3498db;
}

/* ============================================
   Health Score Gauge
   ============================================ */

/* Overview Hero - Health Gauge + Summary side by side */
.overview-hero {
    display: flex;
    align-items: flex-start;
    gap: 32px;
    padding: 24px;
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 20px rgba(0,0,0,0.2);
}

.overview-hero > .health-gauge {
    flex-shrink: 0;
}

.overview-summary-wrapper {
    flex: 1;
    min-width: 0;
}

.overview-summary-wrapper .analysis-summary {
    margin-bottom: 0;
}

@media (max-width: 768px) {
    .overview-hero {
        flex-direction: column;
        align-items: center;
        gap: 20px;
    }

    .overview-summary-wrapper {
        width: 100%;
    }
}

.health-gauge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 16px;
}

.gauge-svg {
    display: block;
}

.gauge-progress {
    transition: stroke-dashoffset 0.6s ease-out;
}

.gauge-status {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

/* Compact inline health indicator */
.health-indicator {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-mono);
    font-size: 12px;
}

/* ============================================
   Audit Panel Component
   ============================================ */

.audit-panel {
    background: var(--theme-bg-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-lg);
    padding: 24px;
}

.audit-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--theme-border);
}

.audit-header h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.health-badge {
    display: flex;
    align-items: baseline;
    gap: 2px;
    padding: 8px 16px;
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-weight: 700;
    border: 2px solid;
}

.health-badge.critical {
    background: rgba(192, 57, 43, 0.1);
    border-color: rgba(192, 57, 43, 0.5);
    color: #c0392b;
}

.health-badge.warning {
    background: rgba(230, 126, 34, 0.1);
    border-color: rgba(230, 126, 34, 0.5);
    color: #e67e22;
}

.health-badge.good {
    background: rgba(39, 174, 96, 0.1);
    border-color: rgba(39, 174, 96, 0.5);
    color: #27ae60;
}

.health-value {
    font-size: 24px;
}

.health-max {
    font-size: 14px;
    opacity: 0.6;
}

/* Audit sections */
.audit-section {
    margin-bottom: 24px;
    padding: 16px;
    border-radius: var(--radius-md);
    border: 1px solid var(--theme-border);
}

.audit-section:last-of-type {
    margin-bottom: 16px;
}

.audit-critical {
    background: rgba(192, 57, 43, 0.05);
    border-color: rgba(192, 57, 43, 0.2);
}

.audit-warning {
    background: rgba(230, 126, 34, 0.05);
    border-color: rgba(230, 126, 34, 0.2);
}

.audit-quick-wins {
    background: rgba(39, 174, 96, 0.05);
    border-color: rgba(39, 174, 96, 0.2);
}

.audit-section-title {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--theme-text-primary);
}

.audit-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
}

.audit-critical .audit-icon {
    background: rgba(192, 57, 43, 0.2);
    color: #c0392b;
}

.audit-warning .audit-icon {
    background: rgba(230, 126, 34, 0.2);
    color: #e67e22;
}

.audit-quick-wins .audit-icon {
    background: rgba(39, 174, 96, 0.2);
    color: #27ae60;
}

.audit-section-desc {
    margin: 0 0 12px 0;
    font-size: 12px;
    color: var(--theme-text-tertiary);
}

/* Audit list */
.audit-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.audit-item {
    padding: 8px 12px;
    background: var(--theme-bg-surface);
    border-radius: var(--radius-sm);
    border: 1px solid var(--theme-border);
    font-size: 13px;
    transition: background 0.15s ease;
}

.audit-item:hover {
    background: var(--theme-bg-surface-elevated);
}

.audit-checkbox-label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
}

.audit-checkbox {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    accent-color: var(--theme-accent);
    cursor: pointer;
}

.audit-checkbox:checked + .audit-symbol,
.audit-checkbox:checked + .audit-cycle,
.audit-checkbox:checked ~ span {
    text-decoration: line-through;
    opacity: 0.5;
}

.audit-symbol {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-accent);
    background: rgba(163, 184, 199, 0.1);
    padding: 2px 6px;
    border-radius: 4px;
}

.audit-cycle {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 500px;
}

.audit-location {
    font-size: 11px;
    color: var(--theme-text-tertiary);
    font-family: var(--font-mono);
}

.audit-category {
    background: transparent;
    border: none;
    padding: 4px 0;
    font-weight: 600;
}

.audit-category-icon {
    color: var(--theme-text-secondary);
}

.audit-count {
    font-family: var(--font-mono);
    color: var(--theme-accent);
}

.audit-sub-item {
    margin-left: 20px;
}

.audit-more {
    font-style: italic;
    color: var(--theme-text-tertiary);
    background: transparent;
    border: none;
}

/* Quick win categories */
.audit-category-cleanup {
    color: var(--theme-text-secondary);
}

.audit-category-refactor {
    color: #3498db;
}

.audit-category-optimize {
    color: #9b59b6;
}

.audit-category-test {
    color: #e67e22;
}

/* Empty state */
.audit-empty {
    padding: 32px;
    text-align: center;
    background: rgba(39, 174, 96, 0.05);
    border-radius: var(--radius-md);
    border: 1px dashed rgba(39, 174, 96, 0.3);
}

.audit-empty p {
    margin: 0;
    color: #27ae60;
    font-size: 14px;
}

/* Footer */
.audit-footer {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--theme-border);
}

.audit-tip {
    margin: 0;
    font-size: 12px;
    color: var(--theme-text-tertiary);
}

.audit-tip code {
    font-size: 11px;
    background: var(--theme-bg-deep);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--theme-accent);
}

/* ============================================
   Refactor Plan Panel
   Vibecrafted with AI Agents by VetCoders (c)2026 VetCoders
   ============================================ */

.refactor-plan-panel {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.refactor-summary {
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.refactor-summary h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 16px;
    color: var(--theme-text-primary);
}

.refactor-stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
}

.refactor-stats-grid .stat-item {
    text-align: center;
    padding: 12px;
    background: var(--theme-hover);
    border-radius: var(--radius-md);
}

.refactor-stats-grid .stat-value {
    font-size: 24px;
    font-weight: 600;
    color: var(--theme-accent);
    display: block;
}

.refactor-stats-grid .stat-label {
    font-size: 11px;
    color: var(--theme-text-tertiary);
    text-transform: uppercase;
}

.risk-badges {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
}

.risk-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
}

.risk-badge.risk-low {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
}

.risk-badge.risk-medium {
    background: rgba(234, 179, 8, 0.15);
    color: #eab308;
}

.risk-badge.risk-high {
    background: rgba(239, 68, 68, 0.15);
    color: #ef4444;
}

/* Layer Distribution */
.layer-distribution {
    padding: 16px;
}

.layer-distribution h4 {
    margin: 0 0 16px 0;
    font-size: 14px;
    color: var(--theme-text-secondary);
}

.distribution-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
}

.distribution-column h5 {
    margin: 0 0 12px 0;
    font-size: 12px;
    color: var(--theme-text-secondary);
    text-transform: uppercase;
}

.layer-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
}

.layer-name {
    width: 70px;
    font-size: 11px;
    color: var(--theme-text-secondary);
    text-transform: capitalize;
}

.bar-track {
    flex: 1;
    height: 8px;
    background: var(--theme-hover);
    border-radius: 4px;
    overflow: hidden;
}

.bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.3s ease;
}

.bar-fill.before {
    background: var(--theme-text-tertiary);
}

.bar-fill.after {
    background: var(--theme-accent);
}

.layer-count {
    width: 30px;
    text-align: right;
    font-size: 11px;
    color: var(--theme-text-tertiary);
    font-family: var(--font-mono);
}

/* Cyclic Warning */
.cyclic-warning {
    background: rgba(234, 179, 8, 0.08);
    border: 1px solid rgba(234, 179, 8, 0.3);
    border-radius: var(--radius-md);
    padding: 16px;
}

.cyclic-warning h4 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px 0;
    color: #eab308;
    font-size: 14px;
}

.cycle-group {
    margin: 12px 0;
    padding-left: 16px;
}

.cycle-group strong {
    font-size: 12px;
    color: var(--theme-text-secondary);
}

.cycle-group ul {
    margin: 4px 0;
    padding-left: 20px;
}

.cycle-group li {
    font-size: 12px;
    margin: 2px 0;
}

.cycle-group code {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-primary);
}

/* Execution Phases */
.execution-phases {
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.phase-card {
    border-radius: var(--radius-md);
    border: 1px solid var(--theme-border);
    overflow: hidden;
    background: var(--theme-bg-surface);
}

.phase-card.risk-low {
    border-left: 4px solid #22c55e;
}

.phase-card.risk-medium {
    border-left: 4px solid #eab308;
}

.phase-card.risk-high {
    border-left: 4px solid #ef4444;
}

.phase-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--theme-hover);
    cursor: pointer;
    user-select: none;
}

.phase-header:hover {
    background: var(--theme-hover-strong);
}

.phase-toggle {
    font-size: 10px;
    color: var(--theme-text-tertiary);
    transition: transform 0.2s ease;
}

.phase-card.collapsed .phase-toggle {
    transform: rotate(-90deg);
}

.phase-card.collapsed .phase-content {
    display: none;
}

.phase-icon {
    display: flex;
    align-items: center;
}

.phase-name {
    font-weight: 500;
    color: var(--theme-text-primary);
}

.phase-count {
    color: var(--theme-text-tertiary);
    font-size: 12px;
    margin-left: auto;
}

.phase-content {
    padding: 16px;
}

.moves-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
}

.moves-table th {
    text-align: left;
    padding: 8px;
    background: var(--theme-hover);
    font-weight: 500;
    color: var(--theme-text-secondary);
    font-size: 11px;
    text-transform: uppercase;
}

.moves-table td {
    padding: 8px;
    border-bottom: 1px solid var(--theme-border);
    color: var(--theme-text-primary);
}

.moves-table code {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-primary);
}

.moves-table tr:last-child td {
    border-bottom: none;
}

.phase-commands {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--theme-border);
}

.phase-commands strong {
    font-size: 12px;
    color: var(--theme-text-secondary);
}

.phase-commands pre {
    margin: 8px 0;
    padding: 12px;
    background: var(--theme-bg-deep);
    border-radius: var(--radius-sm);
    overflow-x: auto;
    font-size: 11px;
}

.phase-commands code {
    font-family: var(--font-mono);
    color: var(--theme-text-primary);
}

/* Shimming Strategy */
.shimming-strategy {
    padding: 16px;
}

.shimming-strategy h4 {
    margin: 0 0 8px 0;
    font-size: 14px;
    color: var(--theme-text-secondary);
}

.shim-item {
    margin: 16px 0;
    padding: 12px;
    background: var(--theme-hover);
    border-radius: var(--radius-sm);
}

.shim-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}

.shim-header code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-primary);
}

.shim-code {
    margin: 8px 0;
    padding: 12px;
    background: var(--theme-bg-deep);
    border-radius: var(--radius-sm);
    font-size: 11px;
    overflow-x: auto;
}

.shim-code code {
    font-family: var(--font-mono);
    color: var(--theme-text-primary);
}

/* Empty State */
.refactor-empty {
    text-align: center;
    padding: 40px;
    color: var(--theme-text-tertiary);
}

.refactor-empty p {
    margin: 0;
    font-size: 14px;
}

.refactor-empty code {
    display: inline-block;
    margin: 8px 0;
    padding: 4px 8px;
    background: var(--theme-hover);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-accent);
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
