//! WASM module for graph rendering in loctree reports.
//!
//! This module provides browser-native graph visualization using Rust/WASM.
//! It accepts graph data as JSON, converts to DOT format, and renders SVG.
//!
//! Developed with 💀 by The Loctree Team (c)2025 

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages in browser console.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Graph node representation matching report-leptos GraphNode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub loc: usize,
    pub x: f64,
    pub y: f64,
    pub component: usize,
    pub degree: usize,
    pub detached: bool,
}

/// Graph component (connected subgraph) representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphComponent {
    pub id: usize,
    pub size: usize,
    pub edge_count: usize,
    pub nodes: Vec<String>,
    pub isolated_count: usize,
    pub sample: String,
    pub loc_sum: usize,
    pub detached: bool,
    pub tauri_frontend: usize,
    pub tauri_backend: usize,
}

/// Complete graph data structure matching report-leptos GraphData.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(String, String, String)>,
    pub components: Vec<GraphComponent>,
    pub main_component_id: usize,
}

impl GraphData {
    /// Convert graph to DOT format for graphviz rendering (light theme).
    pub fn to_dot(&self) -> String {
        self.to_dot_with_theme(false)
    }

    /// Convert graph to DOT format with dark theme colors.
    pub fn to_dot_dark(&self) -> String {
        self.to_dot_with_theme(true)
    }

    fn to_dot_with_theme(&self, dark: bool) -> String {
        let mut dot = String::with_capacity(self.nodes.len() * 100 + self.edges.len() * 50);

        // Graph header with layout settings
        dot.push_str("digraph loctree {\n");
        dot.push_str(
            "  graph [rankdir=TB, splines=true, overlap=false, nodesep=0.5, ranksep=0.8];\n",
        );
        dot.push_str(
            "  node [shape=box, style=\"rounded,filled\", fontname=\"sans-serif\", fontsize=10];\n",
        );
        dot.push_str("  edge [arrowsize=0.7];\n\n");

        // Theme colors
        let (bg_main, bg_detached, font_color, edge_color) = if dark {
            ("#3b82f6", "#6b7280", "#e5e7eb", "#9ca3af")
        } else {
            ("#dbeafe", "#f3f4f6", "#1f2937", "#6b7280")
        };

        // Group nodes by component for subgraph clusters
        let mut components_map: std::collections::HashMap<usize, Vec<&GraphNode>> =
            std::collections::HashMap::new();

        for node in &self.nodes {
            components_map.entry(node.component).or_default().push(node);
        }

        // Render each component as a subgraph cluster
        for (comp_id, nodes) in &components_map {
            let is_main = *comp_id == self.main_component_id;
            let cluster_color = if is_main { bg_main } else { bg_detached };

            dot.push_str(&format!("  subgraph cluster_{} {{\n", comp_id));
            dot.push_str(&format!("    style=filled;\n"));
            dot.push_str(&format!("    color=\"{}\";\n", cluster_color));
            dot.push_str(&format!(
                "    label=\"Component {} ({} nodes)\";\n",
                comp_id,
                nodes.len()
            ));

            for node in nodes {
                // Node size based on LOC (min 0.5, max 2.0)
                let node_width = 0.5 + (node.loc as f64 / 500.0).min(1.5);
                let fill = if node.detached { bg_detached } else { bg_main };

                dot.push_str(&format!(
                    "    \"{}\" [label=\"{}\\n({} LOC)\", width={:.2}, fillcolor=\"{}\", fontcolor=\"{}\"];\n",
                    escape_dot_string(&node.id),
                    escape_dot_string(&node.label),
                    node.loc,
                    node_width,
                    fill,
                    font_color
                ));
            }

            dot.push_str("  }\n\n");
        }

        // Render edges
        for (from, to, kind) in &self.edges {
            let style = match kind.as_str() {
                "reexport" => "dashed",
                _ => "solid",
            };
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [style={}, color=\"{}\"];\n",
                escape_dot_string(from),
                escape_dot_string(to),
                style,
                edge_color
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

/// Escape special characters for DOT format strings.
fn escape_dot_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

// ============================================================================
// WASM Exports
// ============================================================================

/// Parse JSON graph data and convert to DOT format.
///
/// # Arguments
/// * `json_data` - JSON string containing GraphData
/// * `dark_mode` - Whether to use dark theme colors
///
/// # Returns
/// DOT format string or error message
#[wasm_bindgen]
pub fn graph_to_dot(json_data: &str, dark_mode: bool) -> Result<String, JsValue> {
    let graph: GraphData = serde_json::from_str(json_data)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse graph data: {}", e)))?;

    let dot = if dark_mode {
        graph.to_dot_dark()
    } else {
        graph.to_dot()
    };

    Ok(dot)
}

/// Render graph as SVG.
///
/// Currently returns a placeholder SVG. Full implementation will use
/// graphviz-wasm or dot_ix for actual rendering.
///
/// # Arguments
/// * `json_data` - JSON string containing GraphData
/// * `dark_mode` - Whether to use dark theme colors
///
/// # Returns
/// SVG string or error message
#[wasm_bindgen]
pub fn render_graph_svg(json_data: &str, dark_mode: bool) -> Result<String, JsValue> {
    let graph: GraphData = serde_json::from_str(json_data)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse graph data: {}", e)))?;

    // For now, generate a simple placeholder SVG
    // TODO: Integrate graphviz-wasm or dot_ix for real rendering
    let node_count = graph.nodes.len();
    let edge_count = graph.edges.len();

    let bg_color = if dark_mode { "#1f2937" } else { "#ffffff" };
    let text_color = if dark_mode { "#e5e7eb" } else { "#1f2937" };
    let accent_color = if dark_mode { "#3b82f6" } else { "#2563eb" };

    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
  <rect width="100%" height="100%" fill="{}"/>
  <text x="200" y="80" text-anchor="middle" font-family="sans-serif" font-size="16" fill="{}">
    Graph: {} nodes, {} edges
  </text>
  <text x="200" y="110" text-anchor="middle" font-family="sans-serif" font-size="12" fill="{}">
    WASM renderer placeholder
  </text>
  <text x="200" y="140" text-anchor="middle" font-family="sans-serif" font-size="10" fill="{}">
    DOT output ready, SVG rendering coming soon
  </text>
</svg>"#,
        bg_color, text_color, node_count, edge_count, accent_color, accent_color
    );

    Ok(svg)
}

/// Get DOT string for debugging/export purposes.
#[wasm_bindgen]
pub fn get_dot_string(json_data: &str, dark_mode: bool) -> Result<String, JsValue> {
    graph_to_dot(json_data, dark_mode)
}

/// Check if WASM module is loaded and functional.
#[wasm_bindgen]
pub fn health_check() -> String {
    "report-wasm v0.1.0 ready".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_to_dot() {
        let graph = GraphData {
            nodes: vec![GraphNode {
                id: "src/main.ts".into(),
                label: "main.ts".into(),
                loc: 100,
                x: 0.5,
                y: 0.5,
                component: 0,
                degree: 1,
                detached: false,
            }],
            edges: vec![],
            components: vec![],
            main_component_id: 0,
        };

        let dot = graph.to_dot();
        assert!(dot.contains("digraph loctree"));
        assert!(dot.contains("src/main.ts"));
        assert!(dot.contains("100 LOC"));
    }

    #[test]
    fn test_escape_special_chars() {
        let escaped = escape_dot_string("file\"with\"quotes");
        assert!(escaped.contains("\\\""));
    }

    #[test]
    fn test_dark_mode_colors() {
        let graph = GraphData {
            nodes: vec![GraphNode {
                id: "test".into(),
                label: "test".into(),
                loc: 50,
                x: 0.0,
                y: 0.0,
                component: 0,
                degree: 0,
                detached: false,
            }],
            edges: vec![],
            components: vec![],
            main_component_id: 0,
        };

        let light = graph.to_dot();
        let dark = graph.to_dot_dark();

        // Light theme uses lighter colors
        assert!(light.contains("#dbeafe"));
        // Dark theme uses darker colors
        assert!(dark.contains("#3b82f6"));
    }
}
