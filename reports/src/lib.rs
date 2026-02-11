//! # report-leptos
//!
//! Leptos SSR renderer for generating static HTML reports.
//!
//! This crate provides a type-safe, component-based approach to generating
//! beautiful HTML reports using [Leptos](https://leptos.dev/) server-side rendering.
//! Originally built for [loctree](https://github.com/Loctree/Loctree) codebase
//! analysis, it can be used independently for any static report generation needs.
//!
//! ## Features
//!
//! - **Zero JavaScript Runtime** - Pure SSR, no hydration needed
//! - **Component-Based** - Modular, reusable UI components
//! - **Type-Safe** - Full Rust type safety from data to HTML
//! - **Interactive Graphs** - Cytoscape.js integration for dependency visualization
//!
//! ## Quick Start
//!
//! ```rust
//! use report_leptos::{render_report, JsAssets, types::ReportSection};
//!
//! // Create report data
//! let section = ReportSection {
//!     root: "my-project".into(),
//!     files_analyzed: 42,
//!     ..Default::default()
//! };
//!
//! // Configure JS assets (optional, for graph visualization)
//! let js_assets = JsAssets::default();
//!
//! // Render to HTML string
//! let html = render_report(&[section], &js_assets, false);
//!
//! // Write to file
//! std::fs::write("report.html", html).unwrap();
//! ```
//!
//! ## Architecture
//!
//! The crate is organized into modules:
//!
//! - [`types`] - Data structures for report content
//! - [`components`] - Leptos UI components
//! - [`styles`] - CSS constants
//!
//! ## Leptos 0.8 SSR
//!
//! This library uses Leptos 0.8's `RenderHtml` trait:
//!
//! ```rust,ignore
//! use leptos::tachys::view::RenderHtml;
//!
//! let view = view! { <MyComponent /> };
//! let html: String = view.to_html();
//! ```
//!
//! No reactive runtime or hydration is needed - pure static HTML generation.
//!
//! ---
//!
//! Developed with 💀 by The Loctree Team (c)2025

#![doc(html_root_url = "https://docs.rs/report-leptos/0.1.0")]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod components;
pub mod styles;
pub mod types;

use components::ReportDocument;
use leptos::prelude::*;
use leptos::tachys::view::RenderHtml;
use types::ReportSection;

/// Render a complete HTML report from analyzed sections.
///
/// This is the main entry point for generating reports. It takes a slice of
/// [`ReportSection`] data and produces a complete HTML document as a string.
///
/// # Arguments
///
/// * `sections` - Slice of report sections to render
/// * `js_assets` - Paths to JavaScript assets for graph visualization
/// * `has_tauri` - Whether to show Tauri coverage tab (only for Tauri projects)
///
/// # Returns
///
/// A complete HTML document as a `String`, including `<!DOCTYPE html>`.
///
/// # Example
///
/// ```rust
/// use report_leptos::{render_report, JsAssets, types::ReportSection};
///
/// let section = ReportSection {
///     root: "src".into(),
///     files_analyzed: 100,
///     ..Default::default()
/// };
///
/// let html = render_report(&[section], &JsAssets::default(), false);
/// assert!(html.starts_with("<!DOCTYPE html>"));
/// ```
pub fn render_report(sections: &[ReportSection], js_assets: &JsAssets, has_tauri: bool) -> String {
    let doc = view! {
        <ReportDocument sections=sections.to_vec() js_assets=js_assets.clone() has_tauri=has_tauri />
    };

    let html = doc.to_html();

    // Leptos doesn't include DOCTYPE, so we add it
    format!("<!DOCTYPE html>\n{}", html)
}

/// JavaScript asset paths for graph visualization.
///
/// The report uses [Cytoscape.js](https://js.cytoscape.org/) with layout plugins
/// for interactive dependency graph visualization. You can provide paths to:
///
/// - CDN URLs (e.g., unpkg.com)
/// - Local bundled files (for offline use)
/// - Empty strings (graph will show placeholder)
///
/// # Example
///
/// ```rust
/// use report_leptos::JsAssets;
///
/// // CDN paths (with Cytoscape fallback, no WASM)
/// let assets = JsAssets {
///     cytoscape_path: "https://unpkg.com/cytoscape@3/dist/cytoscape.min.js".into(),
///     dagre_path: "https://unpkg.com/dagre@0.8/dist/dagre.min.js".into(),
///     cytoscape_dagre_path: "https://unpkg.com/cytoscape-dagre@2/cytoscape-dagre.js".into(),
///     layout_base_path: "https://unpkg.com/layout-base@2/layout-base.js".into(),
///     cose_base_path: "https://unpkg.com/cose-base@2/cose-base.js".into(),
///     cytoscape_cose_bilkent_path: "https://unpkg.com/cytoscape-cose-bilkent@4/cytoscape-cose-bilkent.js".into(),
///     ..Default::default() // wasm_base64, wasm_js_glue = None
/// };
///
/// // Or use defaults (empty paths - graph shows placeholder)
/// let assets = JsAssets::default();
/// ```
#[derive(Clone, Default, Debug)]
pub struct JsAssets {
    /// Path to cytoscape.min.js
    pub cytoscape_path: String,
    /// Path to dagre.min.js (for hierarchical layouts)
    pub dagre_path: String,
    /// Path to cytoscape-dagre.js plugin
    pub cytoscape_dagre_path: String,
    /// Path to layout-base.js (required by cose-base)
    pub layout_base_path: String,
    /// Path to cose-base.js (required by cytoscape-cose-bilkent)
    pub cose_base_path: String,
    /// Path to cytoscape-cose-bilkent.js plugin (for force-directed layouts)
    pub cytoscape_cose_bilkent_path: String,
    /// Inline WASM module (base64 encoded) for native graph rendering
    pub wasm_base64: Option<String>,
    /// Inline JS glue code for WASM module
    pub wasm_js_glue: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_report() {
        let sections: Vec<ReportSection> = vec![];
        let assets = JsAssets::default();
        let html = render_report(&sections, &assets, false);

        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("loctree"));
    }

    #[test]
    fn renders_section_with_data() {
        let section = ReportSection {
            root: "test-root".into(),
            files_analyzed: 42,
            ..Default::default()
        };
        let assets = JsAssets::default();
        let html = render_report(&[section], &assets, false);

        assert!(html.contains("test-root"));
        assert!(html.contains("42"));
    }

    #[test]
    fn graph_data_to_dot_format() {
        use types::{GraphData, GraphNode};

        let graph = GraphData {
            nodes: vec![
                GraphNode {
                    id: "src/main.ts".into(),
                    label: "main.ts".into(),
                    loc: 150,
                    x: 0.5,
                    y: 0.5,
                    component: 0,
                    degree: 2,
                    detached: false,
                },
                GraphNode {
                    id: "src/utils.ts".into(),
                    label: "utils.ts".into(),
                    loc: 50,
                    x: 0.3,
                    y: 0.7,
                    component: 0,
                    degree: 1,
                    detached: false,
                },
            ],
            edges: vec![("src/main.ts".into(), "src/utils.ts".into(), "import".into())],
            components: vec![],
            main_component_id: 0,
            ..Default::default()
        };

        let dot = graph.to_dot();

        // Verify DOT structure
        assert!(dot.starts_with("digraph loctree"));
        assert!(dot.contains("src/main.ts"));
        assert!(dot.contains("src/utils.ts"));
        assert!(dot.contains("->"));
        assert!(dot.contains("fillcolor"));
    }

    #[test]
    fn graph_data_to_dot_escapes_special_chars() {
        use types::{GraphData, GraphNode};

        let graph = GraphData {
            nodes: vec![GraphNode {
                id: "src/file\"with\"quotes.ts".into(),
                label: "file\"quotes".into(),
                loc: 10,
                x: 0.0,
                y: 0.0,
                component: 0,
                degree: 0,
                detached: false,
            }],
            edges: vec![],
            components: vec![],
            main_component_id: 0,
            ..Default::default()
        };

        let dot = graph.to_dot();

        // Quotes should be escaped
        assert!(dot.contains("\\\""));
        // Raw unescaped quote should not appear in node definitions
        assert!(!dot.contains("file\"with\"quotes"));
    }

    #[test]
    fn renders_action_plan_panel() {
        use types::PriorityTask;

        let section = ReportSection {
            root: "test-root".into(),
            files_analyzed: 1,
            priority_tasks: vec![PriorityTask {
                priority: 1,
                kind: "dead_export".into(),
                target: "GhostFunc".into(),
                location: "src/ghost.rs:42".into(),
                why: "Exported but unused".into(),
                risk: "high".into(),
                fix_hint: "Remove unused export".into(),
                verify_cmd: "loct dead --confidence high".into(),
            }],
            ..Default::default()
        };
        let assets = JsAssets::default();
        let html = render_report(&[section], &assets, false);

        assert!(html.contains("Action Plan"));
        assert!(html.contains("GhostFunc"));
        assert!(html.contains("src/ghost.rs:42"));
    }

    #[test]
    fn renders_hub_files_panel() {
        use types::HubFile;

        let section = ReportSection {
            root: "test-root".into(),
            files_analyzed: 1,
            hub_files: vec![HubFile {
                path: "src/lib.rs".into(),
                loc: 120,
                imports_count: 3,
                exports_count: 7,
                importers_count: 5,
                commands_count: 1,
                slice_cmd: "loct slice src/lib.rs".into(),
            }],
            ..Default::default()
        };
        let assets = JsAssets::default();
        let html = render_report(&[section], &assets, false);

        assert!(html.contains("Context Anchors"));
        assert!(html.contains("src/lib.rs"));
        assert!(html.contains("loct slice src/lib.rs"));
    }
}
