//! Tauri Command Pipelines visualization component
//!
//! Interactive visualization of FE ↔ BE command bridges showing:
//! - Full pipeline chains (Frontend invoke → Backend handler → Emitted events)
//! - Status filtering (ok, missing handler, unused, unregistered)
//! - Expandable details with file locations

use crate::components::icons::{
    Icon, ICON_CHECK_CIRCLE, ICON_GHOST, ICON_LIGHTNING, ICON_PLUG, ICON_WARNING_CIRCLE,
};
use crate::types::CommandBridge;
use leptos::ev;
use leptos::prelude::*;

/// Status filter options
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    Ok,
    Missing,
    Unused,
    Unregistered,
}

impl StatusFilter {
    fn matches(&self, status: &str) -> bool {
        match self {
            StatusFilter::All => true,
            StatusFilter::Ok => status == "ok",
            StatusFilter::Missing => status == "missing_handler",
            StatusFilter::Unused => status == "unused_handler",
            StatusFilter::Unregistered => status == "unregistered_handler",
        }
    }
}

/// Main Pipelines panel - shows all command bridges with interactive filtering
#[component]
pub fn Pipelines(bridges: Vec<CommandBridge>) -> impl IntoView {
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

    // Reactive filter state
    let (filter, set_filter) = signal(StatusFilter::All);
    let (search_query, set_search_query) = signal(String::new());
    let (expanded_cmd, set_expanded_cmd) = signal::<Option<String>>(None);

    // Clone bridges for the closure
    let bridges_for_filter = bridges.clone();

    // Filtered list
    let filtered_bridges = move || {
        let current_filter = filter.get();
        let query = search_query.get().to_lowercase();

        bridges_for_filter
            .iter()
            .filter(|b| current_filter.matches(&b.status))
            .filter(|b| query.is_empty() || b.name.to_lowercase().contains(&query))
            .cloned()
            .collect::<Vec<_>>()
    };

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

            // Filters row
            <div class="pipelines-filters">
                <div class="filter-buttons">
                    <FilterButton
                        label="All"
                        count=total
                        active=move || filter.get() == StatusFilter::All
                        on_click=move |_| set_filter.set(StatusFilter::All)
                    />
                    <FilterButton
                        label="OK"
                        count=ok_count
                        active=move || filter.get() == StatusFilter::Ok
                        on_click=move |_| set_filter.set(StatusFilter::Ok)
                    />
                    <FilterButton
                        label="Missing"
                        count=missing_count
                        active=move || filter.get() == StatusFilter::Missing
                        on_click=move |_| set_filter.set(StatusFilter::Missing)
                    />
                    <FilterButton
                        label="Unused"
                        count=unused_count
                        active=move || filter.get() == StatusFilter::Unused
                        on_click=move |_| set_filter.set(StatusFilter::Unused)
                    />
                    <FilterButton
                        label="Unregistered"
                        count=unreg_count
                        active=move || filter.get() == StatusFilter::Unregistered
                        on_click=move |_| set_filter.set(StatusFilter::Unregistered)
                    />
                </div>

                <div class="search-box">
                    <input
                        type="text"
                        placeholder="Search commands..."
                        class="search-input"
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            set_search_query.set(value);
                        }
                    />
                </div>
            </div>

            // Pipeline cards
            <div class="pipeline-cards">
                {move || {
                    let bridges = filtered_bridges();
                    if bridges.is_empty() {
                        view! {
                            <div class="no-results">
                                <p class="muted">"No commands match the current filter"</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="cards-grid">
                                {bridges.into_iter().map(|bridge| {
                                    let name = bridge.name.clone();
                                    view! {
                                        <PipelineCard
                                            bridge=bridge
                                            cmd_name=name
                                            expanded_cmd=expanded_cmd
                                            set_expanded_cmd=set_expanded_cmd
                                        />
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
    .into_any()
}

/// Filter button component
#[component]
fn FilterButton<F, C>(label: &'static str, count: usize, active: F, on_click: C) -> impl IntoView
where
    F: Fn() -> bool + Send + Sync + 'static,
    C: Fn(ev::MouseEvent) + Send + Sync + 'static,
{
    view! {
        <button
            class=move || if active() { "filter-btn active" } else { "filter-btn" }
            on:click=on_click
        >
            {label}
            <span class="filter-count">{count}</span>
        </button>
    }
}

/// Individual pipeline card with chain visualization
#[component]
fn PipelineCard(
    bridge: CommandBridge,
    cmd_name: String,
    expanded_cmd: ReadSignal<Option<String>>,
    set_expanded_cmd: WriteSignal<Option<String>>,
) -> impl IntoView {
    let name_for_check1 = cmd_name.clone();
    let name_for_check2 = cmd_name.clone();
    let name_for_toggle = cmd_name;

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

    // Clone for closures
    let fe_locations = bridge.fe_locations.clone();
    let be_location = bridge.be_location.clone();
    let name = bridge.name.clone();

    view! {
        <div class=format!("pipeline-card {}", status_class)>
            // Header with command name and status
            <div class="card-header" on:click=move |_| {
                let current = expanded_cmd.get();
                if current.as_ref() == Some(&name_for_toggle) {
                    set_expanded_cmd.set(None);
                } else {
                    set_expanded_cmd.set(Some(name_for_toggle.clone()));
                }
            }>
                <div class="card-title">
                    <code class="command-name">{name}</code>
                    <span class=format!("status-badge {}", status_class)>
                        <Icon path=status_icon size="14" />
                        " "{status_label}
                    </span>
                </div>
                <span class="expand-icon">
                    {move || if expanded_cmd.get().as_ref() == Some(&name_for_check1) { "▼" } else { "▶" }}
                </span>
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

            // Expanded details
            {move || (expanded_cmd.get().as_ref() == Some(&name_for_check2)).then(|| {
                view! {
                    <div class="card-details">
                        // Frontend locations
                        <div class="detail-section">
                            <h4>
                                <Icon path=ICON_LIGHTNING size="14" />
                                " Frontend Calls"
                            </h4>
                            {if fe_locations.is_empty() {
                                view! {
                                    <p class="muted">"No invoke() calls found"</p>
                                }.into_any()
                            } else {
                                view! {
                                    <ul class="location-list">
                                        {fe_locations.iter().map(|(file, line)| {
                                            let file = file.clone();
                                            let line = *line;
                                            view! {
                                                <li>
                                                    <code class="file-path">{file}</code>
                                                    <span class="line-num">":"{line}</span>
                                                </li>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                }.into_any()
                            }}
                        </div>

                        // Backend location
                        <div class="detail-section">
                            <h4>
                                <Icon path=ICON_PLUG size="14" />
                                " Backend Handler"
                            </h4>
                            {match &be_location {
                                Some((file, line, impl_name)) => {
                                    let file = file.clone();
                                    let line = *line;
                                    let impl_name = impl_name.clone();
                                    let bridge_name = bridge.name.clone();
                                    view! {
                                        <ul class="location-list">
                                            <li>
                                                <code class="file-path">{file}</code>
                                                <span class="line-num">":"{line}</span>
                                                {(impl_name != bridge_name).then(|| view! {
                                                    <span class="impl-name">" (impl: "{impl_name.clone()}")"</span>
                                                })}
                                            </li>
                                        </ul>
                                    }.into_any()
                                }
                                None => {
                                    view! {
                                        <p class="muted warning">"⚠ No handler found in backend"</p>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
