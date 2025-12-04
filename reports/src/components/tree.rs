use crate::types::TreeNode;
use leptos::prelude::*;

fn node_matches(node: &TreeNode, f: &str) -> bool {
    if f.is_empty() {
        return true;
    }
    let f_lc = f.to_lowercase();
    node.path.to_lowercase().contains(&f_lc) || node.children.iter().any(|c| node_matches(c, f))
}

fn render_node(node: &TreeNode, depth: usize, f: &str) -> AnyView {
    if !node_matches(node, f) {
        return ().into_any();
    }
    let indent = format!("{}{}", "• ".repeat(depth), node.path.clone());
    let loc_text = format!("{} LOC", node.loc);
    view! {
        <li>
            <div class="tree-row">
                <span class="tree-path">{indent}</span>
                <span class="tree-loc">{loc_text}</span>
            </div>
            {(!node.children.is_empty()).then(|| {
                view! {
                    <ul class="tree-list">
                        {node.children.iter().map(|c| render_node(c, depth + 1, f)).collect_view()}
                    </ul>
                }
            })}
        </li>
    }
    .into_any()
}

#[component]
pub fn TreeView(root_id: String, tree: Vec<TreeNode>) -> impl IntoView {
    let filter = RwSignal::new(String::new());

    view! {
        <div class="tree-panel">
            <div class="tree-header">
                <h3>{"Project tree"}</h3>
                <input
                    class="tree-filter"
                    type="text"
                    placeholder="Filter by path or symbol..."
                    on:input=move |ev| filter.set(event_target_value(&ev))
                />
            </div>
            <ul class="tree-list" data-tab-scope=root_id.clone() data-tab-name="tree">
                {tree.iter().map(|n| render_node(n, 0, &filter.get())).collect_view()}
            </ul>
        </div>
    }
}
