//! Tauri command coverage component

use leptos::prelude::*;
use std::collections::BTreeMap;
use crate::types::{CommandBridge, CommandGap, Confidence};

/// Tauri frontend-backend command coverage display
#[component]
pub fn TauriCommandCoverage(
    missing: Vec<CommandGap>,
    unused: Vec<CommandGap>,
    unregistered: Vec<CommandGap>,
    bridges: Vec<CommandBridge>,
    counts: (usize, usize),
    open_base: Option<String>,
) -> impl IntoView {
    let (fe_count, be_count) = counts;
    let missing_count = missing.len();
    let unused_count = unused.len();
    let unregistered_count = unregistered.len();
    let ok_count = bridges.iter().filter(|b| b.status == "ok").count();

    // Split unused by confidence
    let unused_high: Vec<_> = unused.iter()
        .filter(|g| g.confidence == Some(Confidence::High))
        .cloned()
        .collect();
    let unused_low: Vec<_> = unused.iter()
        .filter(|g| g.confidence == Some(Confidence::Low))
        .cloned()
        .collect();

    view! {
        <h3>"Tauri command coverage"</h3>
        {if counts == (0, 0) {
            view! {
                <p class="muted">"No Tauri commands detected in this root."</p>
            }.into_any()
        } else {
            view! {
                <div class="coverage-summary">
                    <span>"Frontend calls: " <strong>{fe_count}</strong></span>
                    <span>" • Backend handlers: " <strong>{be_count}</strong></span>
                    <span>" • Matched: " <strong class="text-success">{ok_count}</strong></span>
                    <span>" • Missing: " <strong class={if missing_count > 0 { "text-warning" } else { "" }}>{missing_count}</strong></span>
                    <span>" • Unused: " <strong class={if unused_count > 0 { "text-muted" } else { "" }}>{unused_count}</strong></span>
                    {(unregistered_count > 0).then(|| view! {
                        <span>" • Unregistered: " <strong class="text-warning">{unregistered_count}</strong></span>
                    })}
                </div>

                // Full FE↔BE comparison table
                <CommandBridgeTable bridges=bridges.clone() open_base=open_base.clone() />

                // Legacy gap table (for detailed view)
                {if !missing.is_empty() || !unused.is_empty() || !unregistered.is_empty() {
                    view! {
                        <details class="gap-details">
                            <summary>"Gap details (legacy view)"</summary>
                            <table class="command-table">
                                <tr>
                                    <th>"Missing handlers (FE→BE)"</th>
                                    <th>"Handlers unused by FE (HIGH confidence)"</th>
                                    <th>"Handlers unused by FE (LOW confidence)"</th>
                                    <th>"Handlers not registered"</th>
                                </tr>
                                <tr>
                                    <td>
                                        <CommandGapGroup gaps=missing open_base=open_base.clone() />
                                    </td>
                                    <td>
                                        <CommandGapGroup gaps=unused_high open_base=open_base.clone() />
                                    </td>
                                    <td>
                                        <CommandGapGroup gaps=unused_low open_base=open_base.clone() />
                                    </td>
                                    <td>
                                        <CommandGapGroup gaps=unregistered open_base=open_base />
                                    </td>
                                </tr>
                            </table>
                        </details>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            }.into_any()
        }}
    }
}

/// Full FE↔BE command comparison table
#[component]
fn CommandBridgeTable(bridges: Vec<CommandBridge>, open_base: Option<String>) -> impl IntoView {
    if bridges.is_empty() {
        return view! { <p class="muted">"No commands found."</p> }.into_any();
    }

    // Group by status for better visual organization
    let ok_bridges: Vec<_> = bridges.iter().filter(|b| b.status == "ok").cloned().collect();
    let missing_bridges: Vec<_> = bridges.iter().filter(|b| b.status == "missing_handler").cloned().collect();
    let unused_bridges: Vec<_> = bridges.iter().filter(|b| b.status == "unused_handler").cloned().collect();
    let unregistered_bridges: Vec<_> = bridges.iter().filter(|b| b.status == "unregistered_handler").cloned().collect();

    view! {
        <table class="bridge-table">
            <thead>
                <tr>
                    <th>"Command"</th>
                    <th>"Status"</th>
                    <th>"Frontend calls"</th>
                    <th>"Backend handler"</th>
                </tr>
            </thead>
            <tbody>
                // OK bridges first
                {ok_bridges.into_iter().map(|b| {
                    render_bridge_row(b, open_base.clone())
                }).collect::<Vec<_>>()}
                // Missing handlers
                {missing_bridges.into_iter().map(|b| {
                    render_bridge_row(b, open_base.clone())
                }).collect::<Vec<_>>()}
                // Unused handlers
                {unused_bridges.into_iter().map(|b| {
                    render_bridge_row(b, open_base.clone())
                }).collect::<Vec<_>>()}
                // Unregistered handlers
                {unregistered_bridges.into_iter().map(|b| {
                    render_bridge_row(b, open_base.clone())
                }).collect::<Vec<_>>()}
            </tbody>
        </table>
    }.into_any()
}

/// Render a single bridge row
fn render_bridge_row(bridge: CommandBridge, open_base: Option<String>) -> impl IntoView {
    let status_class = match bridge.status.as_str() {
        "ok" => "status-ok",
        "missing_handler" => "status-missing",
        "unused_handler" => "status-unused",
        "unregistered_handler" => "status-unregistered",
        _ => "",
    };

    let status_label = match bridge.status.as_str() {
        "ok" => "✓ OK".to_string(),
        "missing_handler" => "⚠ Missing BE".to_string(),
        "unused_handler" => "○ Unused".to_string(),
        "unregistered_handler" => "⊘ Not registered".to_string(),
        _ => bridge.status.clone(),
    };

    let fe_locs: Vec<String> = bridge.fe_locations.iter()
        .map(|(f, l)| linkify(open_base.as_deref(), f, *l))
        .collect();

    let be_loc = bridge.be_location.as_ref()
        .map(|(f, l, sym)| {
            let link = linkify(open_base.as_deref(), f, *l);
            if *sym != bridge.name {
                format!("{} <span class=\"muted\">({})</span>", link, sym)
            } else {
                link
            }
        })
        .unwrap_or_else(|| "—".to_string());

    view! {
        <tr class=status_class>
            <td><code>{bridge.name}</code></td>
            <td class="status-cell">{status_label}</td>
            <td class="loc-cell" inner_html={
                if fe_locs.is_empty() { "—".to_string() } else { fe_locs.join(", ") }
            }></td>
            <td class="loc-cell" inner_html=be_loc></td>
        </tr>
    }
}

/// Grouped display of command gaps by module
#[component]
fn CommandGapGroup(gaps: Vec<CommandGap>, open_base: Option<String>) -> impl IntoView {
    if gaps.is_empty() {
        return view! { <span class="muted">"None"</span> }.into_any();
    }

    // Group by module (first two path segments)
    let mut groups: BTreeMap<String, Vec<(CommandGap, Vec<String>)>> = BTreeMap::new();

    for gap in gaps {
        let module = gap.locations.first()
            .map(|(p, _)| {
                let parts: Vec<&str> = p.split('/').collect();
                if parts.len() >= 2 {
                    format!("{}/{}", parts[0], parts[1])
                } else {
                    parts.first().unwrap_or(&"").to_string()
                }
            })
            .unwrap_or_default();

        let locs: Vec<String> = gap.locations.iter()
            .map(|(f, l)| linkify(open_base.as_deref(), f, *l))
            .collect();

        groups.entry(module).or_default().push((gap, locs));
    }

    view! {
        {groups.into_iter().map(|(module, items)| {
            let module_label = if module.is_empty() { "-".to_string() } else { module };
            view! {
                <div class="module-group">
                    <div class="module-header">{module_label}</div>
                    <div>
                        {items.into_iter().map(|(gap, locs)| {
                            let alias_info = gap.implementation_name
                                .filter(|impl_name| impl_name != &gap.name)
                                .map(|impl_name| format!(" (impl: {})", impl_name))
                                .unwrap_or_default();

                            view! {
                                <span class="command-pill">
                                    <code>{gap.name}</code>
                                    {(!alias_info.is_empty()).then(|| view! {
                                        <span class="muted">{alias_info}</span>
                                    })}
                                </span>
                                " "
                                <span class="muted" inner_html=locs.join(", ")></span>
                                <br/>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            }
        }).collect::<Vec<_>>()}
    }.into_any()
}

/// Create a link to open file at line (if open_base is set)
fn linkify(base: Option<&str>, file: &str, line: usize) -> String {
    if let Some(base) = base {
        let encoded_file = url_encode(file);
        format!("<a href=\"{}/open?f={}&l={}\">{file}:{line}</a>", base, encoded_file, line)
    } else {
        format!("{file}:{line}")
    }
}

/// Simple URL encoding for file paths
fn url_encode(s: &str) -> String {
    s.chars().map(|c| {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        }
    }).collect()
}
