//! Graph visualization container component

use leptos::prelude::*;
use crate::types::ReportSection;

/// Container for the Cytoscape graph visualization
#[component]
pub fn GraphContainer(
    section: ReportSection,
    root_id: String,
) -> impl IntoView {
    view! {
        <div class="graph-anchor">
            <strong>"Import graph"</strong>
            <span class="muted">"Use the toolbar below to filter, relayout, and export the graph."</span>
        </div>

        {section.graph_warning.map(|warn| {
            view! { <div class="graph-empty">{warn}</div> }
        })}

        {section.graph.map(|graph| {
            let graph_id = format!("graph-{}", root_id);
            let nodes_json = serde_json::to_string(&graph.nodes).unwrap_or("[]".into());
            let edges_json = serde_json::to_string(&graph.edges).unwrap_or("[]".into());
            let components_json = serde_json::to_string(&graph.components).unwrap_or("[]".into());
            let open_json = serde_json::to_string(&section.open_base).unwrap_or("null".into());
            let label_json = serde_json::to_string(&section.root).unwrap_or("\"\"".into());

            let script_content = format!(
                r#"window.__LOCTREE_GRAPHS = window.__LOCTREE_GRAPHS || [];
window.__LOCTREE_GRAPHS.push({{
  id: "{graph_id}",
  label: {label_json},
  nodes: {nodes_json},
  edges: {edges_json},
  components: {components_json},
  mainComponent: {main_component},
  openBase: {open_json}
}});"#,
                main_component = graph.main_component_id,
            );

            view! {
                <div class="graph" id=graph_id></div>
                <script>{script_content}</script>
            }
        })}
    }
}
