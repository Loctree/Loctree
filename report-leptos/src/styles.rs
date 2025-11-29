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
//! - System font stack for native look
//! - Dark mode via `prefers-color-scheme` and manual toggle
//! - Responsive tables and graphs
//! - Tab navigation styling
//! - Cytoscape graph container styling

/// Complete CSS for the report, including dark mode support.
///
/// This CSS provides:
/// - Base typography and spacing
/// - Tab navigation UI
/// - Table styling for data display
/// - Graph container and toolbar
/// - Dark mode theme (auto-detects system preference)
/// - Component panels for graph analysis
pub const REPORT_CSS: &str = r#"
body{font-family:system-ui,-apple-system,Segoe UI,Helvetica,Arial,sans-serif;margin:24px;line-height:1.5;padding-bottom:140px;}
h1,h2,h3{margin-bottom:0.2em;margin-top:0;}
table{border-collapse:collapse;width:100%;margin:0.5em 0;}
th,td{border:1px solid #ddd;padding:6px 8px;font-size:14px;}
th{background:#f5f5f5;text-align:left;}
code{background:#f6f8fa;padding:2px 4px;border-radius:4px;}
.muted{color:#666;}
.section-head{display:flex;justify-content:space-between;align-items:flex-end;gap:12px;flex-wrap:wrap;}
.pill{background:#eef2ff;color:#2b2f3a;padding:4px 8px;border-radius:12px;font-size:12px;}
.tab-bar{display:flex;gap:8px;margin:12px 0 6px 0;flex-wrap:wrap;}
.tab-bar button{border:1px solid #cfd4de;background:#f7f9fc;border-radius:10px;padding:6px 10px;cursor:pointer;font-weight:600;}
.tab-bar button.active{background:#4f81e1;color:#fff;border-color:#4f81e1;box-shadow:0 4px 16px rgba(0,0,0,.12);}
.tab-content{display:none;padding:8px 0;}
.tab-content.active{display:block;}
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
.component-toolbar select,.component-toolbar input[type="range"],.component-toolbar input[type="number"]{font-size:12px;}
.graph-controls button{font-size:12px;padding:4px 8px;border:1px solid #ccc;background:#f8f8f8;border-radius:6px;cursor:pointer;}
.graph-controls button:hover{background:#eee;}
.command-table th,.command-table td{vertical-align:top;}
.command-table code{background:transparent;color:inherit;font-weight:600;}
.command-pill{display:inline-block;padding:3px 6px;border-radius:6px;background:#eef2ff;color:#2b2f3a;font-size:12px;margin:2px 4px 2px 0;}
.dark .command-pill{background:#1f2635;color:#e9ecf5;}
.command-col{width:50%;}
.module-header{font-weight:700;margin-top:4px;}
.module-group{margin-bottom:10px;}
.graph-anchor{margin-top:14px;font-size:13px;color:#444;}
.graph-anchor .muted{display:block;margin-top:4px;}
.report-section .graph,.report-section .graph-toolbar,.report-section .component-panel,.report-section .graph-hint{display:none;}
.graph-drawer{position:fixed;left:16px;right:16px;bottom:12px;z-index:1100;background:#f5f7fb;border:1px solid #cfd4de;border-radius:12px;box-shadow:0 8px 32px rgba(0,0,0,.25);padding:8px 10px;}
.graph-drawer{max-height:82vh;overflow:auto;}
.graph-drawer.collapsed{opacity:0.9;}
.graph-drawer-header{display:flex;align-items:center;gap:10px;cursor:pointer;font-weight:600;}
.graph-drawer-header button{font-size:12px;padding:4px 8px;border:1px solid #ccc;background:#fff;border-radius:6px;cursor:pointer;}
.graph-drawer-body{margin-top:6px;max-height:72vh;overflow:auto;padding-right:6px;}
.graph-drawer .graph{margin:0;border-color:#cfd4de;}

/* AI Summary Panel */
.ai-summary-panel{background:#f8fafc;border:1px solid #d5dce6;border-radius:10px;padding:12px 16px;margin:12px 0;}
.ai-summary-panel h3{margin:0 0 10px;font-size:16px;}
.ai-summary-panel h4{margin:12px 0 6px;font-size:14px;}
.health-badge{display:inline-block;padding:6px 12px;border-radius:8px;font-weight:600;font-size:13px;margin-bottom:10px;}
.health-critical{background:#fecaca;color:#991b1b;}
.health-warning{background:#fef3c7;color:#92400e;}
.health-debt{background:#dbeafe;color:#1e40af;}
.health-good{background:#d1fae5;color:#065f46;}
.summary-table{width:auto;margin:8px 0;}
.summary-table td{padding:4px 12px 4px 0;border:none;font-size:13px;}
.summary-table .row-critical td{color:#dc2626;font-weight:600;}
.summary-table .row-warning td{color:#d97706;font-weight:600;}
.quick-wins{margin-top:12px;}
.quick-wins ul{list-style:none;padding:0;margin:0;}
.quick-wins li{display:flex;gap:8px;align-items:center;padding:6px 0;border-bottom:1px solid #e5e7eb;font-size:13px;flex-wrap:wrap;}
.quick-wins li:last-child{border-bottom:none;}
.quick-wins .action{font-weight:600;min-width:180px;}
.quick-wins code{background:#eef2ff;padding:2px 6px;border-radius:4px;font-size:12px;}
.quick-wins .location{color:#6b7280;font-size:12px;}
.quick-wins .impact{color:#059669;font-size:12px;font-style:italic;}
.priority-1 .action{color:#dc2626;}
.priority-2 .action{color:#d97706;}
.priority-3 .action{color:#2563eb;}
.no-issues{color:#059669;font-style:italic;margin:8px 0;}

/* Dark mode */
.dark body{background:#0f1115;color:#d7dde5;}
.dark table th{background:#1c2029;color:#d7dde5;}
.dark table td{background:#0f1115;color:#d7dde5;border-color:#2a2f3a;}
.dark code{background:#1c2029;color:#f0f4ff;}
.dark .graph{border-color:#2a2f3a;}
.dark .graph-drawer{background:#0b0d11;border-color:#2a2f3a;box-shadow:0 8px 32px rgba(0,0,0,.45);}
.dark .graph-drawer-header button{background:#111522;color:#e9ecf5;border-color:#2a2f3a;}
.dark .component-panel{background:#0f131c;border-color:#2a2f3a;}
.dark .component-chip{background:#1f2635;color:#e9ecf5;}
.dark .pill{background:#1f2635;color:#e9ecf5;}
.dark .tab-bar button{background:#0f131c;color:#e9ecf5;border-color:#2a2f3a;}
.dark .tab-bar button.active{background:#4f81e1;color:#fff;border-color:#4f81e1;}
.dark .ai-summary-panel{background:#0f131c;border-color:#2a2f3a;}
.dark .quick-wins li{border-color:#2a2f3a;}
.dark .quick-wins code{background:#1f2635;color:#e9ecf5;}
.dark .quick-wins .location{color:#9ca3af;}
"#;

/// Content Security Policy header value
pub const CSP: &str = "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'none'; font-src 'self' data:;";
