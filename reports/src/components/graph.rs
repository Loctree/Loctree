//! Graph visualization container component
//!
//! Supports both Cytoscape.js (fallback) and WASM rendering.

use crate::types::ReportSection;
use leptos::prelude::*;

/// Container for the graph visualization (Cytoscape.js or WASM)
#[component]
pub fn GraphContainer(section: ReportSection, root_id: String) -> impl IntoView {
    view! {
        {section.graph_warning.as_ref().map(|warn| {
            view! { <div class="graph-empty">{warn.clone()}</div> }
        })}

        {section.graph.as_ref().map(|graph| {
            let graph_id = format!("graph-{}", root_id);

            // JSON for Cytoscape fallback
            let nodes_json = serde_json::to_string(&graph.nodes).unwrap_or("[]".into());
            let edges_json = serde_json::to_string(&graph.edges).unwrap_or("[]".into());
            let components_json = serde_json::to_string(&graph.components).unwrap_or("[]".into());
            let open_json = serde_json::to_string(&section.open_base).unwrap_or("null".into());
            let label_json = serde_json::to_string(&section.root).unwrap_or("\"\"".into());

            // DOT format for WASM rendering
            let dot_light = graph.to_dot();
            let dot_dark = graph.to_dot_dark();

            // Full graph JSON for WASM (so it can do its own processing)
            let graph_json = serde_json::to_string(&graph).unwrap_or("{}".into());

            let script_content = format!(
                r#"window.__LOCTREE_GRAPHS = window.__LOCTREE_GRAPHS || [];
window.__LOCTREE_GRAPHS.push({{
  id: "{graph_id}",
  label: {label_json},
  nodes: {nodes_json},
  edges: {edges_json},
  components: {components_json},
  mainComponent: {main_component},
  openBase: {open_json},
  dot: {dot_light_json},
  dotDark: {dot_dark_json},
  graphJson: {graph_json}
}});"#,
                main_component = graph.main_component_id,
                dot_light_json = serde_json::to_string(&dot_light).unwrap_or("\"\"".into()),
                dot_dark_json = serde_json::to_string(&dot_dark).unwrap_or("\"\"".into()),
            );

            view! {
                <div class="graph" id=graph_id.clone()>
                    // WASM SVG will be inserted here, or Cytoscape canvas
                    <div class="graph-wasm-target" data-graph-id=graph_id.clone()></div>
                </div>
                <script>{script_content}</script>
            }
        })}
    }
}
