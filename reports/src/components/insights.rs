//! AI insights panel component

use leptos::prelude::*;
use crate::types::AiInsight;
use crate::components::{Icon, ICON_WARNING_CIRCLE, ICON_ROBOT};

/// Panel displaying AI-generated insights
#[component]
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
