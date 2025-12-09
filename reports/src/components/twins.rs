//! Twins analysis panel - Dead Parrots, Exact Twins, and Barrel Chaos
//!
//! Displays three types of code duplication and structural issues:
//! 1. Dead Parrots - symbols exported but never imported
//! 2. Exact Twins - symbols with identical names in different files
//! 3. Barrel Chaos - missing/deep/inconsistent barrel files

use crate::components::{Icon, ICON_COPY, ICON_FOLDER_OPEN, ICON_GHOST, ICON_TWINS};
use crate::types::{BarrelChaos, DeadParrot, ExactTwin, TwinsData};
use leptos::prelude::*;

/// Main twins analysis panel
#[component]
pub fn Twins(twins: Option<TwinsData>) -> impl IntoView {
    let twins_data = match twins {
        Some(data) => data,
        None => {
            return view! {
                <div class="panel">
                    <h3>
                        <Icon path=ICON_COPY />
                        "Twins Analysis"
                    </h3>
                    <div class="graph-empty">
                        <div style="text-align: center; padding: 32px;">
                            <Icon path=ICON_COPY size="48" color="var(--theme-text-tertiary)" />
                            <p style="margin-top: 16px; color: var(--theme-text-secondary);">
                                "Twins analysis not available"
                            </p>
                            <p class="muted" style="font-size: 12px; margin-top: 8px;">
                                "Run analysis with --twins flag to enable this feature"
                            </p>
                        </div>
                    </div>
                </div>
            }
            .into_any();
        }
    };

    let dead_parrots_count = twins_data.dead_parrots.len();
    let exact_twins_count = twins_data.exact_twins.len();
    let barrel_issues_count = twins_data.barrel_chaos.missing_barrels.len()
        + twins_data.barrel_chaos.deep_chains.len()
        + twins_data.barrel_chaos.inconsistent_paths.len();

    view! {
        <div class="panel">
            <h3>
                <Icon path=ICON_COPY />
                "Twins Analysis"
            </h3>

            <div class="twins-summary" style="margin-bottom: 24px;">
                <p class="muted">
                    {format!(
                        "{} dead parrots, {} exact twins, {} barrel issues",
                        dead_parrots_count,
                        exact_twins_count,
                        barrel_issues_count
                    )}
                </p>
            </div>

            <DeadParrotsSection dead_parrots=twins_data.dead_parrots.clone() />
            <ExactTwinsSection
                exact_twins=twins_data.exact_twins.clone()
                dead_parrots=twins_data.dead_parrots.clone()
            />
            <BarrelChaosSection barrel_chaos=twins_data.barrel_chaos.clone() />
        </div>
    }
    .into_any()
}

/// Dead Parrots section - symbols with 0 imports
/// SSR-friendly: pre-renders content hidden, JS handles toggle
#[component]
fn DeadParrotsSection(dead_parrots: Vec<DeadParrot>) -> impl IntoView {
    let count = dead_parrots.len();

    if count == 0 {
        view! {
            <div class="twins-section" data-twins-section="dead-parrots">
                <button class="twins-section-header" data-toggle="twins-dead-parrots-content">
                    <span class="twins-section-title">
                        <Icon path=ICON_GHOST class="icon-sm" />
                        {format!(" Dead Parrots ({} symbols)", count)}
                    </span>
                    <span class="twins-section-toggle">"▶"</span>
                </button>
                <div id="twins-dead-parrots-content" class="twins-section-content" style="display: none;">
                    <p class="muted">"No dead parrots found - all exports are imported!"</p>
                </div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="twins-section" data-twins-section="dead-parrots">
                <button class="twins-section-header" data-toggle="twins-dead-parrots-content">
                    <span class="twins-section-title">
                        <Icon path=ICON_GHOST class="icon-sm" />
                        {format!(" Dead Parrots ({} symbols)", count)}
                    </span>
                    <span class="twins-section-toggle">"▶"</span>
                </button>
                <div id="twins-dead-parrots-content" class="twins-section-content" style="display: none;">
                    <DeadParrotsTable dead_parrots=dead_parrots.clone() />
                </div>
            </div>
        }
        .into_any()
    }
}

/// Table showing dead parrots
#[component]
fn DeadParrotsTable(dead_parrots: Vec<DeadParrot>) -> impl IntoView {
    // Group by file for cleaner display
    let mut by_file: std::collections::HashMap<String, Vec<DeadParrot>> =
        std::collections::HashMap::new();
    for parrot in dead_parrots {
        by_file
            .entry(parrot.file_path.clone())
            .or_default()
            .push(parrot);
    }

    let mut files: Vec<_> = by_file.keys().cloned().collect();
    files.sort();

    view! {
        <table class="data-table twins-table">
            <thead>
                <tr>
                    <th>"File"</th>
                    <th>"Symbol"</th>
                    <th>"Kind"</th>
                    <th>"Line"</th>
                </tr>
            </thead>
            <tbody>
                {files.into_iter().flat_map(|file| {
                    let parrots = by_file.get(&file).unwrap().clone();
                    parrots.into_iter().map(move |parrot| {
                        view! {
                            <tr>
                                <td class="file-cell">
                                    <code>{parrot.file_path.clone()}</code>
                                </td>
                                <td class="symbol-cell">
                                    <code>{parrot.name.clone()}</code>
                                </td>
                                <td class="kind-cell">
                                    {parrot.kind.clone()}
                                </td>
                                <td class="line-cell">
                                    {parrot.line.to_string()}
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()
                }).collect::<Vec<_>>()}
            </tbody>
        </table>
    }
}

/// Exact Twins section - symbols with same name in different files
#[component]
fn ExactTwinsSection(exact_twins: Vec<ExactTwin>, dead_parrots: Vec<DeadParrot>) -> impl IntoView {
    let count = exact_twins.len();

    // Serialize twins data for JavaScript visualization
    let twins_json = serde_json::to_string(&exact_twins).unwrap_or_else(|_| "[]".to_string());
    let parrots_json = serde_json::to_string(&dead_parrots).unwrap_or_else(|_| "[]".to_string());

    // Build initialization script for Cytoscape graph
    // This runs when the twins tab is shown (handled by twins-graph-init in APP_SCRIPT)
    let graph_data_script = format!(
        r#"window.__TWINS_DATA__ = {{ exactTwins: {twins}, deadParrots: {parrots} }};"#,
        twins = twins_json,
        parrots = parrots_json
    );

    // For SSR, we pre-render the content (hidden by default via CSS)
    // JavaScript handles the toggle visibility
    if count == 0 {
        view! {
            <div class="twins-section" data-twins-section="exact">
                <button class="twins-section-header" data-toggle="twins-exact-content">
                    <span class="twins-section-title">
                        <Icon path=ICON_TWINS class="icon-sm" />
                        {format!(" Exact Twins ({} duplicates)", count)}
                    </span>
                    <span class="twins-section-toggle">"▶"</span>
                </button>
                <div id="twins-exact-content" class="twins-section-content" style="display: none;">
                    <p class="muted">"No exact twins found - all symbol names are unique!"</p>
                </div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="twins-section" data-twins-section="exact">
                <button class="twins-section-header" data-toggle="twins-exact-content">
                    <span class="twins-section-title">
                        <Icon path=ICON_TWINS class="icon-sm" />
                        {format!(" Exact Twins ({} duplicates)", count)}
                    </span>
                    <span class="twins-section-toggle">"▶"</span>
                </button>
                // Pre-render graph data script (always in HTML)
                <script>{graph_data_script}</script>
                <div id="twins-exact-content" class="twins-section-content" style="display: none;">
                    <div class="twins-graph-wrapper" style="margin-bottom: 24px;">
                        <div
                            id="twins-graph-container"
                            class="twins-graph-container"
                            style="width: 100%; height: 500px; border: 1px solid var(--theme-border); border-radius: 8px; background: var(--theme-bg-secondary);"
                        ></div>
                        <div class="twins-graph-legend" style="margin-top: 12px; display: flex; gap: 16px; justify-content: center; flex-wrap: wrap;">
                            <span style="display: flex; align-items: center; gap: 4px;">
                                <span style="width: 12px; height: 12px; border-radius: 50%; background: #4ade80;"></span>
                                "File (green = no dead parrots)"
                            </span>
                            <span style="display: flex; align-items: center; gap: 4px;">
                                <span style="width: 12px; height: 12px; border-radius: 50%; background: #f87171;"></span>
                                "File (red = many dead parrots)"
                            </span>
                            <span style="display: flex; align-items: center; gap: 4px;">
                                <span style="width: 12px; height: 3px; background: var(--theme-accent);"></span>
                                "Shared symbol connection"
                            </span>
                        </div>
                    </div>
                    <ExactTwinsTable exact_twins=exact_twins.clone() />
                </div>
            </div>
        }
        .into_any()
    }
}

/// Table showing exact twins
#[component]
fn ExactTwinsTable(exact_twins: Vec<ExactTwin>) -> impl IntoView {
    view! {
        <table class="data-table twins-table">
            <thead>
                <tr>
                    <th>"Symbol"</th>
                    <th>"Locations"</th>
                    <th>"Canonical"</th>
                </tr>
            </thead>
            <tbody>
                {exact_twins.into_iter().map(|twin| {
                    let canonical = twin.locations.iter()
                        .find(|loc| loc.is_canonical)
                        .map(|loc| loc.file_path.clone())
                        .unwrap_or_else(|| "None".to_string());

                    view! {
                        <tr>
                            <td class="symbol-cell">
                                <code>{twin.name.clone()}</code>
                            </td>
                            <td class="locations-cell">
                                <ul class="location-list">
                                    {twin.locations.into_iter().map(|loc| {
                                        let marker = if loc.is_canonical { " ⭐" } else { "" };
                                        view! {
                                            <li>
                                                <code>{format!("{}:{}", loc.file_path, loc.line)}</code>
                                                {format!(" ({}) - {} imports{}", loc.kind, loc.import_count, marker)}
                                            </li>
                                        }
                                    }).collect::<Vec<_>>()}
                                </ul>
                            </td>
                            <td class="canonical-cell">
                                <code>{canonical}</code>
                            </td>
                        </tr>
                    }
                }).collect::<Vec<_>>()}
            </tbody>
        </table>
    }
}

/// Barrel Chaos section - missing/deep/inconsistent barrels
/// SSR-friendly: pre-renders content hidden, JS handles toggle
#[component]
fn BarrelChaosSection(barrel_chaos: BarrelChaos) -> impl IntoView {
    let total_issues = barrel_chaos.missing_barrels.len()
        + barrel_chaos.deep_chains.len()
        + barrel_chaos.inconsistent_paths.len();

    if total_issues == 0 {
        view! {
            <div class="twins-section" data-twins-section="barrel">
                <button class="twins-section-header" data-toggle="twins-barrel-content">
                    <span class="twins-section-title">
                        <Icon path=ICON_FOLDER_OPEN class="icon-sm" />
                        {format!(" Barrel Chaos ({} issues)", total_issues)}
                    </span>
                    <span class="twins-section-toggle">"▶"</span>
                </button>
                <div id="twins-barrel-content" class="twins-section-content" style="display: none;">
                    <p class="muted">"No barrel chaos detected - clean module structure!"</p>
                </div>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="twins-section" data-twins-section="barrel">
                <button class="twins-section-header" data-toggle="twins-barrel-content">
                    <span class="twins-section-title">
                        <Icon path=ICON_FOLDER_OPEN class="icon-sm" />
                        {format!(" Barrel Chaos ({} issues)", total_issues)}
                    </span>
                    <span class="twins-section-toggle">"▶"</span>
                </button>
                <div id="twins-barrel-content" class="twins-section-content" style="display: none;">
                    <BarrelChaosDetails barrel_chaos=barrel_chaos.clone() />
                </div>
            </div>
        }
        .into_any()
    }
}

/// Detailed breakdown of barrel chaos issues
#[component]
fn BarrelChaosDetails(barrel_chaos: BarrelChaos) -> impl IntoView {
    view! {
        <div class="barrel-chaos-details">
            {if !barrel_chaos.missing_barrels.is_empty() {
                view! {
                    <div class="barrel-issue-group">
                        <h4>{format!("Missing Barrels ({})", barrel_chaos.missing_barrels.len())}</h4>
                        <table class="data-table twins-table">
                            <thead>
                                <tr>
                                    <th>"Directory"</th>
                                    <th>"Files"</th>
                                    <th>"External Imports"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {barrel_chaos.missing_barrels.into_iter().map(|barrel| {
                                    view! {
                                        <tr>
                                            <td class="file-cell">
                                                <code>{barrel.directory}</code>
                                            </td>
                                            <td class="count-cell">{barrel.file_count.to_string()}</td>
                                            <td class="count-cell">{barrel.external_import_count.to_string()}</td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

            {if !barrel_chaos.deep_chains.is_empty() {
                view! {
                    <div class="barrel-issue-group">
                        <h4>{format!("Deep Re-export Chains ({})", barrel_chaos.deep_chains.len())}</h4>
                        <table class="data-table twins-table">
                            <thead>
                                <tr>
                                    <th>"Symbol"</th>
                                    <th>"Chain"</th>
                                    <th>"Depth"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {barrel_chaos.deep_chains.into_iter().map(|chain| {
                                    let chain_str = chain.chain.join(" → ");
                                    view! {
                                        <tr>
                                            <td class="symbol-cell">
                                                <code>{chain.symbol}</code>
                                            </td>
                                            <td class="chain-cell">
                                                <code>{chain_str}</code>
                                            </td>
                                            <td class="count-cell">{chain.depth.to_string()}</td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}

            {if !barrel_chaos.inconsistent_paths.is_empty() {
                view! {
                    <div class="barrel-issue-group">
                        <h4>{format!("Inconsistent Import Paths ({})", barrel_chaos.inconsistent_paths.len())}</h4>
                        <table class="data-table twins-table">
                            <thead>
                                <tr>
                                    <th>"Symbol"</th>
                                    <th>"Canonical Path"</th>
                                    <th>"Alternative Paths"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {barrel_chaos.inconsistent_paths.into_iter().map(|inconsistent| {
                                    let alternatives = inconsistent.alternative_paths.iter()
                                        .map(|(path, count)| format!("{} ({}×)", path, count))
                                        .collect::<Vec<_>>()
                                        .join(", ");

                                    view! {
                                        <tr>
                                            <td class="symbol-cell">
                                                <code>{inconsistent.symbol}</code>
                                            </td>
                                            <td class="file-cell">
                                                <code>{inconsistent.canonical_path}</code>
                                            </td>
                                            <td class="alternatives-cell">
                                                <code>{alternatives}</code>
                                            </td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}
