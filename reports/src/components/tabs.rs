//! Tab navigation components
//!
//! Provides the navigation tabs for the report sections.
//! Uses the new "App-like" styling.

use leptos::prelude::*;
use crate::components::{Icon, ICON_SQUARES_FOUR, ICON_COPY, ICON_LIGHTNING, ICON_TERMINAL, ICON_GRAPH};

/// Tab bar with navigation buttons (rendered in the Sticky Header)
#[component]
pub fn TabBar(root_id: String) -> impl IntoView {
    view! {
        <nav class="header-tabs tab-bar" data-tab-scope=root_id.clone()>
            <button class="tab-btn active" data-tab="overview">
                <Icon path=ICON_SQUARES_FOUR />
                "Overview"
            </button>
            <button class="tab-btn" data-tab="dups">
                <Icon path=ICON_COPY />
                "Duplicates"
            </button>
            <button class="tab-btn" data-tab="dynamic">
                <Icon path=ICON_LIGHTNING />
                "Dynamic imports"
            </button>
            <button class="tab-btn" data-tab="commands">
                <Icon path=ICON_TERMINAL />
                "Tauri coverage"
            </button>
            <button class="tab-btn" data-tab="graph">
                <Icon path=ICON_GRAPH />
                "Graph"
            </button>
        </nav>
    }
}

/// Tab content panel (rendered in the Scrollable Area)
#[component]
pub fn TabContent(
    root_id: String,
    tab_name: &'static str,
    active: bool,
    children: Children,
) -> impl IntoView {
    let class = if active { "tab-panel active" } else { "tab-panel" };
    
    view! {
        <div 
            class=class 
            data-tab-scope=root_id 
            data-tab-name=tab_name
        >
            {children()}
        </div>
    }
}
