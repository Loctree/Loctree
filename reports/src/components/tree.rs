use crate::components::icons::{
    Icon, ICON_ARROWS_IN, ICON_ARROWS_OUT, ICON_CARET_RIGHT, ICON_FILE, ICON_FILE_CODE,
    ICON_FOLDER, ICON_FOLDER_OPEN,
};
use crate::types::TreeNode;
use leptos::prelude::*;
use std::collections::HashSet;

fn node_matches(node: &TreeNode, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f_lc = filter.to_lowercase();
    node.path.to_lowercase().contains(&f_lc) || node.children.iter().any(|c| node_matches(c, &f_lc))
}

fn is_code_file(path: &str) -> bool {
    let code_extensions = [
        "rs", "ts", "tsx", "js", "jsx", "py", "vue", "svelte", "go", "rb", "java", "kt", "swift",
        "c", "cpp", "h", "hpp", "cs", "php", "scala", "clj", "ex", "exs", "elm", "hs",
    ];
    path.rsplit('.')
        .next()
        .map(|ext| code_extensions.contains(&ext))
        .unwrap_or(false)
}

fn get_max_loc(nodes: &[TreeNode]) -> usize {
    nodes.iter().map(|n| n.loc).max().unwrap_or(1).max(1)
}

/// Count total files and LOC in tree (for summary display)
fn count_tree_stats(nodes: &[TreeNode]) -> (usize, usize) {
    fn count_recursive(nodes: &[TreeNode], files: &mut usize, loc: &mut usize) {
        for node in nodes {
            if node.children.is_empty() {
                // Leaf node = file
                *files += 1;
                *loc += node.loc;
            } else {
                // Directory - recurse
                count_recursive(&node.children, files, loc);
            }
        }
    }
    let mut files = 0;
    let mut loc = 0;
    count_recursive(nodes, &mut files, &mut loc);
    (files, loc)
}

#[component]
fn TreeNodeView(
    node: TreeNode,
    depth: usize,
    is_last: bool,
    prefix: String,
    filter: RwSignal<String>,
    collapsed: RwSignal<HashSet<String>>,
    max_loc: usize,
) -> impl IntoView {
    let path = node.path.clone();
    let path_for_toggle = path.clone();
    let path_check_1 = path.clone();
    let path_check_2 = path.clone();
    let is_dir = !node.children.is_empty();
    let loc = node.loc;
    let children = node.children.clone();

    // Calculate LOC bar width (percentage of max)
    let loc_pct = ((loc as f64 / max_loc as f64) * 100.0).min(100.0);

    // Build the tree connector
    let connector = if depth == 0 {
        String::new()
    } else if is_last {
        format!("{}└─ ", prefix)
    } else {
        format!("{}├─ ", prefix)
    };

    // Build prefix for children
    let child_prefix = if depth == 0 {
        String::new()
    } else if is_last {
        format!("{}   ", prefix)
    } else {
        format!("{}│  ", prefix)
    };

    let toggle_collapse = move |_| {
        // Only toggle collapse state for directories (nodes with children)
        if is_dir {
            collapsed.update(|set| {
                if set.contains(&path_for_toggle) {
                    set.remove(&path_for_toggle);
                } else {
                    set.insert(path_for_toggle.clone());
                }
            });
        }
    };

    let is_collapsed_chevron = move || collapsed.get().contains(&path_check_1);
    let is_collapsed_children = move || collapsed.get().contains(&path_check_2);

    let filter_val = filter.get();
    if !node_matches(&node, &filter_val) {
        return view! { <div></div> }.into_any();
    }

    // Highlight matching text
    let display_path =
        if !filter_val.is_empty() && path.to_lowercase().contains(&filter_val.to_lowercase()) {
            let lower_path = path.to_lowercase();
            let lower_filter = filter_val.to_lowercase();
            if let Some(idx) = lower_path.find(&lower_filter) {
                let before = &path[..idx];
                let matched = &path[idx..idx + filter_val.len()];
                let after = &path[idx + filter_val.len()..];
                view! {
                    <span>
                        {before.to_string()}
                        <mark class="tree-highlight">{matched.to_string()}</mark>
                        {after.to_string()}
                    </span>
                }
                .into_any()
            } else {
                view! { <span>{path.clone()}</span> }.into_any()
            }
        } else {
            view! { <span>{path.clone()}</span> }.into_any()
        };

    let loc_bar_style = format!("width: {}%", loc_pct);

    // Select appropriate icon
    let file_icon = if is_code_file(&path) {
        ICON_FILE_CODE
    } else {
        ICON_FILE
    };

    view! {
        <div class="tree-node">
            <div
                class="tree-row"
                class:tree-row-dir=is_dir
                on:click=toggle_collapse
            >
                <div class="tree-left">
                    <span class="tree-connector">{connector}</span>
                    {is_dir.then(|| {
                        let is_open = !is_collapsed_chevron();
                        view! {
                            <span class="tree-chevron" class:collapsed=is_collapsed_chevron>
                                <Icon path=ICON_CARET_RIGHT size="14" />
                            </span>
                            <span class="tree-icon">
                                {if is_open {
                                    view! { <Icon path=ICON_FOLDER_OPEN size="16" /> }.into_any()
                                } else {
                                    view! { <Icon path=ICON_FOLDER size="16" /> }.into_any()
                                }}
                            </span>
                        }
                    })}
                    {(!is_dir).then(|| view! {
                        <span class="tree-icon">
                            <Icon path=file_icon size="16" />
                        </span>
                    })}
                    <span class="tree-path">{display_path}</span>
                </div>
                <div class="tree-right">
                    <div class="tree-loc-bar">
                        <div class="tree-loc-fill" style=loc_bar_style></div>
                    </div>
                    <span class="tree-loc">{format!("{} LOC", loc)}</span>
                </div>
            </div>
            {is_dir.then(|| {
                let children_len = children.len();
                view! {
                    <div class="tree-children" class:collapsed=is_collapsed_children>
                        {children.into_iter().enumerate().map(|(i, child)| {
                            view! {
                                <TreeNodeView
                                    node=child
                                    depth=depth + 1
                                    is_last=i == children_len - 1
                                    prefix=child_prefix.clone()
                                    filter=filter
                                    collapsed=collapsed
                                    max_loc=max_loc
                                />
                            }
                        }).collect_view()}
                    </div>
                }
            })}
        </div>
    }
    .into_any()
}

#[component]
pub fn TreeView(root_id: String, tree: Vec<TreeNode>) -> impl IntoView {
    let filter = RwSignal::new(String::new());
    let collapsed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let max_loc = get_max_loc(&tree);
    let tree_len = tree.len();
    let (file_count, total_loc) = count_tree_stats(&tree);

    let expand_all = move |_| {
        collapsed.set(HashSet::new());
    };

    let collapse_all = {
        let tree = tree.clone();
        move |_| {
            fn collect_dirs(nodes: &[TreeNode], set: &mut HashSet<String>) {
                for node in nodes {
                    if !node.children.is_empty() {
                        set.insert(node.path.clone());
                        collect_dirs(&node.children, set);
                    }
                }
            }
            let mut all_dirs = HashSet::new();
            collect_dirs(&tree, &mut all_dirs);
            collapsed.set(all_dirs);
        }
    };

    // Format LOC with K/M suffix for readability
    let loc_display = if total_loc >= 1_000_000 {
        format!("{:.1}M", total_loc as f64 / 1_000_000.0)
    } else if total_loc >= 1_000 {
        format!("{:.1}K", total_loc as f64 / 1_000.0)
    } else {
        format!("{}", total_loc)
    };

    view! {
        <div class="tree-panel" data-tab-scope=root_id.clone() data-tab-name="tree">
            <div class="tree-header">
                <h3>"Analyzed files "</h3>
                <span class="tree-stats" title="Only code files loctree can analyze (JS/TS, Python, Rust, Go, etc.) are shown">
                    {format!("{} files · {} LOC", file_count, loc_display)}
                </span>
                <div class="tree-controls">
                    <button class="tree-btn" on:click=expand_all title="Expand all">
                        <Icon path=ICON_ARROWS_OUT size="16" />
                    </button>
                    <button class="tree-btn" on:click=collapse_all title="Collapse all">
                        <Icon path=ICON_ARROWS_IN size="16" />
                    </button>
                </div>
                <input
                    class="tree-filter"
                    type="text"
                    placeholder="Filter by path..."
                    on:input=move |ev| filter.set(event_target_value(&ev))
                />
            </div>
            <div class="tree-container">
                {tree.into_iter().enumerate().map(|(i, node)| {
                    view! {
                        <TreeNodeView
                            node=node
                            depth=0
                            is_last=i == tree_len - 1
                            prefix=String::new()
                            filter=filter
                            collapsed=collapsed
                            max_loc=max_loc
                        />
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
