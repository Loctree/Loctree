//! AI insights panel component

use leptos::prelude::*;
use crate::types::AiInsight;
use crate::components::{Icon, ICON_WARNING_CIRCLE, ICON_ROBOT, ICON_SQUARES_FOUR};

/// Summary statistics panel
#[component]
pub fn AnalysisSummary(
    files_analyzed: usize,
    total_loc: usize,
    duplicate_exports: usize,
    reexport_files: usize,
    dynamic_imports: usize,
) -> impl IntoView {
    view! {
        <div class="analysis-summary">
            <h3>
                <Icon path=ICON_SQUARES_FOUR />
                "Analysis Summary"
            </h3>
            <div class="summary-grid">
                <div class="summary-stat">
                    <span class="stat-value">{files_analyzed.to_string()}</span>
                    <span class="stat-label">"Files analyzed"</span>
                </div>
                <div class="summary-stat">
                    <span class="stat-value">{format_loc(total_loc)}</span>
                    <span class="stat-label">"Total LOC"</span>
                </div>
                <div class="summary-stat">
                    <span class="stat-value">{duplicate_exports.to_string()}</span>
                    <span class="stat-label">"Duplicate exports"</span>
                </div>
                <div class="summary-stat">
                    <span class="stat-value">{reexport_files.to_string()}</span>
                    <span class="stat-label">"Re-export files"</span>
                </div>
                <div class="summary-stat">
                    <span class="stat-value">{dynamic_imports.to_string()}</span>
                    <span class="stat-label">"Dynamic imports"</span>
                </div>
            </div>
        </div>
    }
}

fn format_loc(loc: usize) -> String {
    if loc >= 1_000_000 {
        format!("{:.1}M", loc as f64 / 1_000_000.0)
    } else if loc >= 1_000 {
        format!("{:.1}K", loc as f64 / 1_000.0)
    } else {
        loc.to_string()
    }
}

/// Panel displaying AI-generated insights
#[component]
#[allow(clippy::unit_arg, clippy::unused_unit)]
pub fn AiInsightsPanel(insights: Vec<AiInsight>) -> impl IntoView {
    if insights.is_empty() {
        return view! {}.into_any();
    }

    view! {
        <h3>
            <Icon path=ICON_ROBOT />
            "AI Insights"
        </h3>
        <ul class="insight-list">
            {insights.into_iter().map(|insight| {
                let color = match insight.severity.as_str() {
                    "high" => "#e74c3c",   // Red
                    "medium" => "#e67e22", // Orange
                    _ => "#3498db",        // Blue
                };
                view! {
                    <li class="insight-item">
                        <div class="insight-icon">
                            <Icon path=ICON_WARNING_CIRCLE color=color />
                        </div>
                        <div class="insight-content">
                            <strong style=format!("color:{}", color)>
                                {insight.title}
                            </strong>
                            <p>{insight.message}</p>
                        </div>
                    </li>
                }
            }).collect::<Vec<_>>()}
        </ul>
    }.into_any()
}
