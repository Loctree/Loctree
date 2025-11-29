//! AI insights panel component

use leptos::prelude::*;
use crate::types::AiInsight;

/// Panel displaying AI-generated insights
#[component]
pub fn AiInsightsPanel(insights: Vec<AiInsight>) -> impl IntoView {
    if insights.is_empty() {
        return view! {}.into_any();
    }

    view! {
        <h3>"AI Insights"</h3>
        <ul class="command-list">
            {insights.into_iter().map(|insight| {
                let color = match insight.severity.as_str() {
                    "high" => "#e74c3c",
                    "medium" => "#e67e22",
                    _ => "#3498db",
                };
                view! {
                    <li>
                        <strong style=format!("color:{}", color)>
                            {insight.title}
                        </strong>
                        ": "
                        {insight.message}
                    </li>
                }
            }).collect::<Vec<_>>()}
        </ul>
    }.into_any()
}
