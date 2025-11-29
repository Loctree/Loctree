//! Tab navigation components

use leptos::prelude::*;

/// Tab bar with navigation buttons
#[component]
pub fn TabBar(root_id: String) -> impl IntoView {
    view! {
        <div class="tab-bar" data-tab-scope=root_id.clone()>
            <button class="active" data-tab=format!("{}-overview", root_id)>
                "Overview"
            </button>
            <button data-tab=format!("{}-dups", root_id)>
                "Duplicates"
            </button>
            <button data-tab=format!("{}-dynamic", root_id)>
                "Dynamic imports"
            </button>
            <button data-tab=format!("{}-commands", root_id)>
                "Tauri coverage"
            </button>
            <button data-tab=format!("{}-graph", root_id)>
                "Graph"
            </button>
        </div>
    }
}

/// Tab content panel
#[component]
pub fn TabContent(
    root_id: String,
    tab_name: &'static str,
    active: bool,
    children: Children,
) -> impl IntoView {
    let class = if active { "tab-content active" } else { "tab-content" };
    let panel_id = format!("{}-{}", root_id, tab_name);

    view! {
        <div class=class data-tab-panel=panel_id>
            {children()}
        </div>
    }
}
