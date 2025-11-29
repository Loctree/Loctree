//! AI Summary components for Leptos reports.
//!
//! Renders the same data as `--for-ai` JSON but in a human-readable format.

use leptos::prelude::*;

use crate::types::ReportSection;

/// Quick win action item
#[derive(Clone)]
pub struct QuickWin {
    pub priority: u8,
    pub action: String,
    pub target: String,
    pub location: String,
    pub impact: String,
}

/// Extract quick wins from report sections
pub fn extract_quick_wins(sections: &[ReportSection]) -> Vec<QuickWin> {
    let mut wins = Vec::new();
    let mut priority = 1u8;

    // Missing handlers (highest priority)
    for section in sections {
        for gap in &section.missing_handlers {
            if priority > 10 {
                break;
            }
            let location = gap
                .locations
                .first()
                .map(|(f, l)| format!("{}:{}", f, l))
                .unwrap_or_else(|| "unknown".to_string());

            wins.push(QuickWin {
                priority,
                action: "Add missing handler".to_string(),
                target: gap.name.clone(),
                location,
                impact: "Fixes runtime error".to_string(),
            });
            priority += 1;
        }
    }

    // Unregistered handlers
    for section in sections {
        for gap in &section.unregistered_handlers {
            if priority > 15 {
                break;
            }
            let location = gap
                .locations
                .first()
                .map(|(f, l)| format!("{}:{}", f, l))
                .unwrap_or_else(|| "unknown".to_string());

            wins.push(QuickWin {
                priority,
                action: "Register in generate_handler![]".to_string(),
                target: gap.name.clone(),
                location,
                impact: "Handler not exposed".to_string(),
            });
            priority += 1;
        }
    }

    // Unused handlers
    for section in sections {
        for gap in &section.unused_handlers {
            if priority > 20 {
                break;
            }
            let location = gap
                .locations
                .first()
                .map(|(f, l)| format!("{}:{}", f, l))
                .unwrap_or_else(|| "unknown".to_string());

            wins.push(QuickWin {
                priority,
                action: "Remove unused handler".to_string(),
                target: gap.name.clone(),
                location,
                impact: "Dead code cleanup".to_string(),
            });
            priority += 1;
        }
    }

    wins
}

/// AI Summary Panel component
#[component]
pub fn AiSummaryPanel(sections: Vec<ReportSection>) -> impl IntoView {
    let total_files: usize = sections.iter().map(|s| s.files_analyzed).sum();
    let missing: usize = sections.iter().map(|s| s.missing_handlers.len()).sum();
    let unregistered: usize = sections.iter().map(|s| s.unregistered_handlers.len()).sum();
    let unused: usize = sections.iter().map(|s| s.unused_handlers.len()).sum();
    let duplicates: usize = sections.iter().map(|s| s.ranked_dups.len()).sum();

    let quick_wins = extract_quick_wins(&sections);
    let has_issues = missing > 0 || unregistered > 0 || unused > 0;

    let health_class = if missing > 0 {
        "health-critical"
    } else if unregistered > 0 {
        "health-warning"
    } else if unused > 0 || duplicates > 0 {
        "health-debt"
    } else {
        "health-good"
    };

    view! {
        <div class="ai-summary-panel">
            <h3>"AI Summary"</h3>

            <div class=format!("health-badge {}", health_class)>
                {if missing > 0 {
                    format!("🚨 {} CRITICAL issues", missing)
                } else if unregistered > 0 {
                    format!("⚠️ {} warnings", unregistered)
                } else if has_issues {
                    format!("📋 {} items to review", unused + duplicates)
                } else {
                    "✅ Healthy".to_string()
                }}
            </div>

            <table class="summary-table">
                <tbody>
                    <tr>
                        <td>"Files analyzed"</td>
                        <td><strong>{total_files}</strong></td>
                    </tr>
                    <tr class={if missing > 0 { "row-critical" } else { "" }}>
                        <td>"Missing handlers"</td>
                        <td><strong>{missing}</strong></td>
                    </tr>
                    <tr class={if unregistered > 0 { "row-warning" } else { "" }}>
                        <td>"Unregistered handlers"</td>
                        <td><strong>{unregistered}</strong></td>
                    </tr>
                    <tr>
                        <td>"Unused handlers"</td>
                        <td><strong>{unused}</strong></td>
                    </tr>
                    <tr>
                        <td>"Duplicate exports"</td>
                        <td><strong>{duplicates}</strong></td>
                    </tr>
                </tbody>
            </table>

            {if !quick_wins.is_empty() {
                view! {
                    <div class="quick-wins">
                        <h4>"Quick Wins"</h4>
                        <ul>
                            {quick_wins.into_iter().map(|win| {
                                view! {
                                    <li class=format!("priority-{}", win.priority.min(3))>
                                        <span class="action">{win.action}</span>
                                        <code>{win.target}</code>
                                        <span class="location">{win.location}</span>
                                        <span class="impact">{win.impact}</span>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                }.into_any()
            } else {
                view! { <p class="no-issues">"No immediate action items."</p> }.into_any()
            }}
        </div>
    }
}
