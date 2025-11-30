//! # report-leptos
//!
//! Leptos SSR renderer for generating static HTML reports.
//!
//! This crate provides a type-safe, component-based approach to generating
//! beautiful HTML reports using [Leptos](https://leptos.dev/) server-side rendering.
//! Originally built for [loctree](https://github.com/LibraxisAI/Loctree) codebase
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
//! let html = render_report(&[section], &js_assets);
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
//! Created by M&K (c)2025 The LibraxisAI Team

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
/// let html = render_report(&[section], &JsAssets::default());
/// assert!(html.starts_with("<!DOCTYPE html>"));
/// ```
pub fn render_report(sections: &[ReportSection], js_assets: &JsAssets) -> String {
    let doc = view! {
        <ReportDocument sections=sections.to_vec() js_assets=js_assets.clone() />
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
/// // CDN paths
/// let assets = JsAssets {
///     cytoscape_path: "https://unpkg.com/cytoscape@3/dist/cytoscape.min.js".into(),
///     dagre_path: "https://unpkg.com/dagre@0.8/dist/dagre.min.js".into(),
///     cytoscape_dagre_path: "https://unpkg.com/cytoscape-dagre@2/cytoscape-dagre.js".into(),
///     cytoscape_cose_bilkent_path: "https://unpkg.com/cytoscape-cose-bilkent@4/cytoscape-cose-bilkent.js".into(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_empty_report() {
        let sections: Vec<ReportSection> = vec![];
        let assets = JsAssets::default();
        let html = render_report(&sections, &assets);

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
        let html = render_report(&[section], &assets);

        assert!(html.contains("test-root"));
        assert!(html.contains("42"));
    }
}
