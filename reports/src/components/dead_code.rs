//! Dead code panel component - shows dead exports (Dead Parrots)

use crate::components::{Icon, ICON_GHOST};
use crate::types::DeadExport;
use leptos::prelude::*;

/// Panel displaying dead exports (exported symbols that are never imported)
#[component]
pub fn DeadCode(dead_exports: Vec<DeadExport>) -> impl IntoView {
    let count = dead_exports.len();

    // Split by confidence for filtering
    let high_confidence: Vec<_> = dead_exports
        .iter()
        .filter(|e| e.confidence == "very-high")
        .cloned()
        .collect();

    let high_confidence_count = high_confidence.len();

    view! {
        <div class="panel">
            <h3>
                <Icon path=ICON_GHOST />
                "Dead Exports"
            </h3>

            {if count == 0 {
                view! {
                    <div class="graph-empty">
                        <div style="text-align: center; padding: 32px;">
                            <Icon path=ICON_GHOST size="48" color="var(--theme-text-tertiary)" />
                            <p style="margin-top: 16px; color: var(--theme-text-secondary);">
                                "No dead exports detected"
                            </p>
                            <p class="muted" style="font-size: 12px; margin-top: 8px;">
                                "All exports are being imported somewhere in the codebase"
                            </p>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <DeadCodeTable
                        dead_exports=dead_exports
                        total_count=count
                        high_confidence_count=high_confidence_count
                    />
                }.into_any()
            }}
        </div>
    }
}

/// Table showing dead exports with filtering
#[component]
fn DeadCodeTable(
    dead_exports: Vec<DeadExport>,
    total_count: usize,
    high_confidence_count: usize,
) -> impl IntoView {
    // State for filter toggle
    let (show_high_only, set_show_high_only) = signal(false);

    // Filtered list based on toggle
    let filtered = move || {
        if show_high_only.get() {
            dead_exports
                .iter()
                .filter(|e| e.confidence == "very-high")
                .cloned()
                .collect::<Vec<_>>()
        } else {
            dead_exports.clone()
        }
    };

    view! {
        <div class="dead-code-summary">
            <p class="muted">
                {format!(
                    "{} dead export{} found ({} very high confidence)",
                    total_count,
                    if total_count == 1 { "" } else { "s" },
                    high_confidence_count
                )}
            </p>

            <label class="filter-toggle">
                <input
                    type="checkbox"
                    checked=show_high_only
                    on:change=move |_| set_show_high_only.update(|v| *v = !*v)
                />
                "Show very high confidence only"
            </label>
        </div>

        <table class="data-table dead-exports-table">
            <thead>
                <tr>
                    <th>"File"</th>
                    <th>"Symbol"</th>
                    <th>"Line"</th>
                    <th>"Confidence"</th>
                    <th>"Reason"</th>
                </tr>
            </thead>
            <tbody>
                {move || filtered().into_iter().map(|export| {
                    let confidence_class = match export.confidence.as_str() {
                        "very-high" => "confidence-very-high",
                        "high" => "confidence-high",
                        _ => "confidence-medium",
                    };

                    let confidence_text = match export.confidence.as_str() {
                        "very-high" => "Very High",
                        "high" => "High",
                        _ => "Medium",
                    };

                    let is_test_attr = if export.is_test { "true" } else { "false" };

                    view! {
                        <tr class=confidence_class data-is-test=is_test_attr>
                            <td class="file-cell">
                                {if let Some(url) = &export.open_url {
                                    view! {
                                        <a href=url.clone() title="Open in editor">
                                            <code>{export.file.clone()}</code>
                                        </a>
                                    }.into_any()
                                } else {
                                    view! {
                                        <code>{export.file.clone()}</code>
                                    }.into_any()
                                }}
                            </td>
                            <td class="symbol-cell">
                                <code>{export.symbol.clone()}</code>
                            </td>
                            <td class="line-cell">
                                {export.line.map(|l| l.to_string()).unwrap_or_else(|| "-".to_string())}
                            </td>
                            <td class="confidence-cell">
                                <span class=format!("confidence-badge {}", confidence_class)>
                                    {confidence_text}
                                </span>
                            </td>
                            <td class="reason-cell">
                                {export.reason.clone()}
                            </td>
                        </tr>
                    }
                }).collect::<Vec<_>>()}
            </tbody>
        </table>
    }
}
