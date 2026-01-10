//! Tauri Command Pipelines visualization component
//!
//! Interactive visualization of FE ↔ BE command bridges showing:
//! - Full pipeline chains (Frontend invoke → Backend handler → Emitted events)
//! - Status filtering (ok, missing handler, unused, unregistered)
//! - Expandable details with clickable file locations
//! - Split panel view with FE/BE side-by-side and connecting lines
//!
//! Uses vanilla JS for interactivity (no WASM hydration needed)

use crate::components::icons::{
    Icon, ICON_CHECK_CIRCLE, ICON_GHOST, ICON_LIGHTNING, ICON_PLUG, ICON_WARNING_CIRCLE,
};
use crate::types::CommandBridge;
use leptos::prelude::*;

/// Create a clickable link to open file at line (if open_base is set)
fn linkify(base: Option<&str>, file: &str, line: usize) -> String {
    if let Some(base) = base {
        let encoded_file = url_encode(file);
        format!(
            "<a href=\"{}/open?f={}&l={}\" class=\"file-link\">{file}:{line}</a>",
            base, encoded_file, line
        )
    } else {
        format!("{file}:{line}")
    }
}

/// Simple URL encoding for file paths
fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// Main Pipelines panel - shows all command bridges with interactive filtering
#[component]
pub fn Pipelines(bridges: Vec<CommandBridge>, open_base: Option<String>) -> impl IntoView {
    let total = bridges.len();

    if total == 0 {
        return view! {
            <div class="panel">
                <h3>
                    <Icon path=ICON_PLUG />
                    " Command Pipelines"
                </h3>
                <div class="graph-empty">
                    <div style="text-align: center; padding: 32px;">
                        <Icon path=ICON_PLUG size="48" color="var(--theme-text-tertiary)" />
                        <p style="margin-top: 16px; color: var(--theme-text-secondary);">
                            "No Tauri commands detected"
                        </p>
                        <p class="muted" style="font-size: 12px; margin-top: 8px;">
                            "This view shows frontend invoke() calls and their backend handlers"
                        </p>
                    </div>
                </div>
            </div>
        }
        .into_any();
    }

    // Count by status
    let ok_count = bridges.iter().filter(|b| b.status == "ok").count();
    let missing_count = bridges
        .iter()
        .filter(|b| b.status == "missing_handler")
        .count();
    let unused_count = bridges
        .iter()
        .filter(|b| b.status == "unused_handler")
        .count();
    let unreg_count = bridges
        .iter()
        .filter(|b| b.status == "unregistered_handler")
        .count();

    view! {
        <div class="panel pipelines-panel">
            <h3>
                <Icon path=ICON_PLUG />
                " Command Pipelines"
            </h3>

            // Summary stats
            <div class="pipelines-summary">
                <div class="pipeline-stats">
                    <span class="stat-chip stat-total">{total}" total"</span>
                    <span class="stat-chip stat-ok">{ok_count}" OK"</span>
                    {(missing_count > 0).then(|| view! {
                        <span class="stat-chip stat-missing">{missing_count}" missing"</span>
                    })}
                    {(unused_count > 0).then(|| view! {
                        <span class="stat-chip stat-unused">{unused_count}" unused"</span>
                    })}
                    {(unreg_count > 0).then(|| view! {
                        <span class="stat-chip stat-unreg">{unreg_count}" unregistered"</span>
                    })}
                </div>
            </div>

            // Filters row - static buttons with data attributes for JS
            <div class="pipelines-filters">
                <div class="filter-buttons">
                    <button class="filter-btn active" data-pipeline-filter="all">
                        "All"<span class="filter-count">{total}</span>
                    </button>
                    <button class="filter-btn" data-pipeline-filter="ok">
                        "OK"<span class="filter-count">{ok_count}</span>
                    </button>
                    <button class="filter-btn" data-pipeline-filter="missing_handler">
                        "Missing"<span class="filter-count">{missing_count}</span>
                    </button>
                    <button class="filter-btn" data-pipeline-filter="unused_handler">
                        "Unused"<span class="filter-count">{unused_count}</span>
                    </button>
                    <button class="filter-btn" data-pipeline-filter="unregistered_handler">
                        "Unregistered"<span class="filter-count">{unreg_count}</span>
                    </button>
                </div>

                <div class="search-box">
                    <input
                        type="text"
                        placeholder="Search commands..."
                        class="search-input"
                        data-pipeline-search="true"
                    />
                </div>

                // View toggle buttons
                <div class="view-toggle">
                    <button class="view-btn active" data-pipeline-view="grid" title="Grid View">"#"</button>
                    <button class="view-btn" data-pipeline-view="split" title="Split View">"||"</button>
                </div>
            </div>

            // Grid View - Pipeline cards (default, visible)
            <div class="pipeline-cards pipeline-view-grid" data-pipeline-view-container="grid">
                <div class="cards-grid">
                    {bridges.iter().map(|bridge| {
                        view! { <PipelineCard bridge=bridge.clone() open_base=open_base.clone() /> }
                    }).collect::<Vec<_>>()}
                </div>
            </div>

            // Split View - FE/BE side-by-side (hidden by default)
            <div class="pipeline-view-split" data-pipeline-view-container="split" style="display: none;">
                <SplitPanelView bridges=bridges open_base=open_base />
            </div>
        </div>
    }
    .into_any()
}

/// Split Panel View component - shows FE and BE side-by-side with connecting lines
#[component]
fn SplitPanelView(bridges: Vec<CommandBridge>, open_base: Option<String>) -> impl IntoView {
    // Separate FE items (commands with frontend calls) and BE items (handlers)
    let fe_items: Vec<_> = bridges
        .iter()
        .filter(|b| !b.fe_locations.is_empty())
        .cloned()
        .collect();

    let be_items: Vec<_> = bridges
        .iter()
        .filter(|b| b.be_location.is_some())
        .cloned()
        .collect();

    // For SVG connections, we need to know which items have both FE and BE
    let connected_commands: Vec<_> = bridges
        .iter()
        .filter(|b| !b.fe_locations.is_empty() && b.be_location.is_some())
        .map(|b| b.name.clone())
        .collect();

    view! {
        <div class="split-panel-container">
            // Left panel - Frontend calls
            <div class="split-panel split-panel-fe">
                <h4>
                    <Icon path=ICON_LIGHTNING size="16" />
                    " Frontend Calls"
                </h4>
                <div class="panel-items">
                    {fe_items.into_iter().map(|bridge| {
                        view! { <SplitItem bridge=bridge.clone() side="fe" open_base=open_base.clone() /> }
                    }).collect::<Vec<_>>()}
                </div>
            </div>

            // Center - SVG connection lines
            <div class="split-panel-connections">
                <svg class="connection-svg" data-split-svg="true">
                    // Lines will be drawn by JS based on element positions
                    // We render placeholder data attributes for JS to read
                    {connected_commands.iter().map(|name| {
                        view! {
                            <line
                                class="connection-line"
                                data-split-connection=name.clone()
                                x1="0"
                                y1="0"
                                x2="80"
                                y2="0"
                            />
                        }
                    }).collect::<Vec<_>>()}
                </svg>
            </div>

            // Right panel - Backend handlers
            <div class="split-panel split-panel-be">
                <h4>
                    <Icon path=ICON_PLUG size="16" />
                    " Backend Handlers"
                </h4>
                <div class="panel-items">
                    {be_items.into_iter().map(|bridge| {
                        view! { <SplitItem bridge=bridge.clone() side="be" open_base=open_base.clone() /> }
                    }).collect::<Vec<_>>()}

                    // Show missing handler placeholders
                    {bridges.iter()
                        .filter(|b| b.status == "missing_handler")
                        .map(|bridge| {
                            view! {
                                <div
                                    class="split-item split-item-placeholder status-missing"
                                    data-split-be=bridge.name.clone()
                                >
                                    <div class="split-item-name">
                                        <Icon path=ICON_WARNING_CIRCLE size="14" />
                                        " "{bridge.name.clone()}
                                    </div>
                                    <div class="split-item-location warning">
                                        "Missing handler"
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }
}

/// Individual item in the split panel view
#[component]
fn SplitItem(
    bridge: CommandBridge,
    side: &'static str,
    open_base: Option<String>,
) -> impl IntoView {
    let status_class = match bridge.status.as_str() {
        "ok" => "status-ok",
        "missing_handler" => "status-missing",
        "unused_handler" => "status-unused",
        "unregistered_handler" => "status-unreg",
        _ => "status-unknown",
    };

    let status_icon = match bridge.status.as_str() {
        "ok" => ICON_CHECK_CIRCLE,
        "missing_handler" => ICON_WARNING_CIRCLE,
        "unused_handler" => ICON_GHOST,
        "unregistered_handler" => ICON_WARNING_CIRCLE,
        _ => ICON_WARNING_CIRCLE,
    };

    // Build location string
    let location_html = if side == "fe" {
        // Frontend: show first location (or count if multiple)
        if let Some((file, line)) = bridge.fe_locations.first() {
            let link = linkify(open_base.as_deref(), file, *line);
            if bridge.fe_locations.len() > 1 {
                format!(
                    "{} <span class=\"muted\">+{} more</span>",
                    link,
                    bridge.fe_locations.len() - 1
                )
            } else {
                link
            }
        } else {
            "<span class=\"muted\">No calls found</span>".to_string()
        }
    } else {
        // Backend: show handler location
        if let Some((file, line, impl_name)) = &bridge.be_location {
            let link = linkify(open_base.as_deref(), file, *line);
            if impl_name != &bridge.name {
                format!(
                    "{} <span class=\"impl-name\">(impl: {})</span>",
                    link, impl_name
                )
            } else {
                link
            }
        } else {
            "<span class=\"muted warning\">No handler</span>".to_string()
        }
    };

    // Build the item based on side (fe or be)
    if side == "fe" {
        view! {
            <div
                class=format!("split-item {}", status_class)
                data-pipeline-status=bridge.status.clone()
                data-pipeline-name=bridge.name.to_lowercase()
                data-split-fe=bridge.name.clone()
            >
                <div class="split-item-header">
                    <span class="split-item-name">
                        <code>{bridge.name.clone()}</code>
                    </span>
                    <span class=format!("split-item-status {}", status_class)>
                        <Icon path=status_icon size="12" />
                    </span>
                </div>
                <div class="split-item-location" inner_html=location_html></div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div
                class=format!("split-item {}", status_class)
                data-pipeline-status=bridge.status.clone()
                data-pipeline-name=bridge.name.to_lowercase()
                data-split-be=bridge.name.clone()
            >
                <div class="split-item-header">
                    <span class="split-item-name">
                        <code>{bridge.name.clone()}</code>
                    </span>
                    <span class=format!("split-item-status {}", status_class)>
                        <Icon path=status_icon size="12" />
                    </span>
                </div>
                <div class="split-item-location" inner_html=location_html></div>
            </div>
        }
        .into_any()
    }
}

/// Individual pipeline card with chain visualization
#[component]
fn PipelineCard(bridge: CommandBridge, open_base: Option<String>) -> impl IntoView {
    let status_class = match bridge.status.as_str() {
        "ok" => "status-ok",
        "missing_handler" => "status-missing",
        "unused_handler" => "status-unused",
        "unregistered_handler" => "status-unreg",
        _ => "status-unknown",
    };

    let status_icon = match bridge.status.as_str() {
        "ok" => ICON_CHECK_CIRCLE,
        "missing_handler" => ICON_WARNING_CIRCLE,
        "unused_handler" => ICON_GHOST,
        "unregistered_handler" => ICON_WARNING_CIRCLE,
        _ => ICON_WARNING_CIRCLE,
    };

    let status_label = match bridge.status.as_str() {
        "ok" => "OK",
        "missing_handler" => "Missing Handler",
        "unused_handler" => "Unused",
        "unregistered_handler" => "Not Registered",
        _ => "Unknown",
    };

    let has_fe = !bridge.fe_locations.is_empty();
    let has_be = bridge.be_location.is_some();
    let fe_count = bridge.fe_locations.len();

    // Build FE locations HTML
    let fe_html = if bridge.fe_locations.is_empty() {
        "<p class=\"muted\">No invoke() calls found</p>".to_string()
    } else {
        let items: Vec<String> = bridge
            .fe_locations
            .iter()
            .map(|(file, line)| format!("<li>{}</li>", linkify(open_base.as_deref(), file, *line)))
            .collect();
        format!("<ul class=\"location-list\">{}</ul>", items.join(""))
    };

    // Build BE location HTML
    let be_html = match &bridge.be_location {
        Some((file, line, impl_name)) => {
            let link = linkify(open_base.as_deref(), file, *line);
            let impl_suffix = if impl_name != &bridge.name {
                format!(" <span class=\"impl-name\">(impl: {})</span>", impl_name)
            } else {
                String::new()
            };
            format!(
                "<ul class=\"location-list\"><li>{}{}</li></ul>",
                link, impl_suffix
            )
        }
        None => "<p class=\"muted warning\">⚠ No handler found in backend</p>".to_string(),
    };

    view! {
        <div
            class=format!("pipeline-card {}", status_class)
            data-pipeline-status=bridge.status.clone()
            data-pipeline-name=bridge.name.to_lowercase()
        >
            // Header with command name and status - clickable via JS
            <div class="card-header" data-pipeline-toggle="true">
                <div class="card-title">
                    <code class="command-name">{bridge.name.clone()}</code>
                    <span class=format!("status-badge {}", status_class)>
                        <Icon path=status_icon size="14" />
                        " "{status_label}
                    </span>
                    {(!bridge.emits_events.is_empty()).then(|| {
                        let event_count = bridge.emits_events.len();
                        let events_title = format!("Emits: {}", bridge.emits_events.join(", "));
                        view! {
                            <span class="comm-badge comm-emit" title=events_title>
                                <Icon path=ICON_LIGHTNING size="12" />
                                " "{event_count}" event"{if event_count == 1 { "" } else { "s" }}
                            </span>
                        }
                    })}
                </div>
                <span class="expand-icon">"▶"</span>
            </div>

            // Chain visualization (always visible)
            <div class="chain-viz">
                // Frontend node
                <div class=if has_fe { "chain-node fe active" } else { "chain-node fe inactive" }>
                    <div class="node-icon">"FE"</div>
                    <div class="node-label">
                        {if has_fe {
                            format!("{} call{}", fe_count, if fe_count == 1 { "" } else { "s" })
                        } else {
                            "No calls".to_string()
                        }}
                    </div>
                </div>

                // Arrow
                <div class=if has_fe && has_be { "chain-arrow active" } else { "chain-arrow" }>
                    <span class="arrow-line"></span>
                    <span class="arrow-head">"→"</span>
                </div>

                // Backend node
                <div class=if has_be { "chain-node be active" } else { "chain-node be inactive" }>
                    <div class="node-icon">"BE"</div>
                    <div class="node-label">
                        {if has_be { "Handler" } else { "Missing" }}
                    </div>
                </div>
            </div>

            // Expanded details - hidden by default, shown via JS toggle
            <div class="card-details" style="display: none;">
                // Frontend locations
                <div class="detail-section">
                    <h4>
                        <Icon path=ICON_LIGHTNING size="14" />
                        " Frontend Calls"
                    </h4>
                    <div inner_html=fe_html></div>
                </div>

                // Backend location
                <div class="detail-section">
                    <h4>
                        <Icon path=ICON_PLUG size="14" />
                        " Backend Handler"
                    </h4>
                    <div inner_html=be_html></div>
                </div>
            </div>
        </div>
    }
}
