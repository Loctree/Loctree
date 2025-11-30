//! Tab content components for report sections.
//!
//! Navigation is handled by sidebar nav-items in document.rs.
//! This module provides only the TabContent wrapper for content panels.

use leptos::prelude::*;

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
