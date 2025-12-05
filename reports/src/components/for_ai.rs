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

/// Determine health class based on issues
pub fn determine_health_class(
    missing: usize,
    unregistered: usize,
    unused: usize,
    duplicates: usize,
) -> &'static str {
    if missing > 0 {
        "health-critical"
    } else if unregistered > 0 {
        "health-warning"
    } else if unused > 0 || duplicates > 0 {
        "health-debt"
    } else {
        "health-good"
    }
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
    let health_class = determine_health_class(missing, unregistered, unused, duplicates);

    view! {
        <div class="ai-summary-panel">
            <h3>"AI Summary"</h3>

            <div class=format!("health-badge {}", health_class)>
                {if missing > 0 {
                    format!("{} CRITICAL issues", missing)
                } else if unregistered > 0 {
                    format!("{} warnings", unregistered)
                } else if has_issues {
                    format!("{} items to review", unused + duplicates)
                } else {
                    "Healthy".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CommandGap;

    #[test]
    fn health_class_critical_when_missing_handlers() {
        assert_eq!(determine_health_class(1, 0, 0, 0), "health-critical");
        assert_eq!(determine_health_class(5, 3, 2, 1), "health-critical");
    }

    #[test]
    fn health_class_warning_when_unregistered() {
        assert_eq!(determine_health_class(0, 1, 0, 0), "health-warning");
        assert_eq!(determine_health_class(0, 3, 2, 1), "health-warning");
    }

    #[test]
    fn health_class_debt_when_unused_or_duplicates() {
        assert_eq!(determine_health_class(0, 0, 1, 0), "health-debt");
        assert_eq!(determine_health_class(0, 0, 0, 1), "health-debt");
        assert_eq!(determine_health_class(0, 0, 5, 3), "health-debt");
    }

    #[test]
    fn health_class_good_when_no_issues() {
        assert_eq!(determine_health_class(0, 0, 0, 0), "health-good");
    }

    #[test]
    fn extract_quick_wins_empty_sections() {
        let sections: Vec<ReportSection> = vec![];
        let wins = extract_quick_wins(&sections);
        assert!(wins.is_empty());
    }

    #[test]
    fn extract_quick_wins_no_issues() {
        let section = ReportSection {
            root: "test".into(),
            files_analyzed: 10,
            ..Default::default()
        };
        let wins = extract_quick_wins(&[section]);
        assert!(wins.is_empty());
    }

    #[test]
    fn extract_quick_wins_missing_handlers() {
        let section = ReportSection {
            root: "test".into(),
            missing_handlers: vec![
                CommandGap {
                    name: "get_user".into(),
                    locations: vec![("api.ts".into(), 42)],
                    ..Default::default()
                },
                CommandGap {
                    name: "save_data".into(),
                    locations: vec![("data.ts".into(), 10)],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let wins = extract_quick_wins(&[section]);

        assert_eq!(wins.len(), 2);
        assert_eq!(wins[0].priority, 1);
        assert_eq!(wins[0].target, "get_user");
        assert_eq!(wins[0].location, "api.ts:42");
        assert_eq!(wins[0].action, "Add missing handler");
        assert_eq!(wins[1].priority, 2);
        assert_eq!(wins[1].target, "save_data");
    }

    #[test]
    fn extract_quick_wins_unregistered_handlers() {
        let section = ReportSection {
            root: "test".into(),
            unregistered_handlers: vec![CommandGap {
                name: "internal_fn".into(),
                locations: vec![("lib.rs".into(), 100)],
                ..Default::default()
            }],
            ..Default::default()
        };
        let wins = extract_quick_wins(&[section]);

        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].action, "Register in generate_handler![]");
        assert_eq!(wins[0].impact, "Handler not exposed");
    }

    #[test]
    fn extract_quick_wins_unused_handlers() {
        let section = ReportSection {
            root: "test".into(),
            unused_handlers: vec![CommandGap {
                name: "old_handler".into(),
                locations: vec![("old.rs".into(), 5)],
                ..Default::default()
            }],
            ..Default::default()
        };
        let wins = extract_quick_wins(&[section]);

        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].action, "Remove unused handler");
        assert_eq!(wins[0].impact, "Dead code cleanup");
    }

    #[test]
    fn extract_quick_wins_priority_order() {
        let section = ReportSection {
            root: "test".into(),
            missing_handlers: vec![CommandGap {
                name: "missing".into(),
                locations: vec![("a.ts".into(), 1)],
                ..Default::default()
            }],
            unregistered_handlers: vec![CommandGap {
                name: "unregistered".into(),
                locations: vec![("b.rs".into(), 2)],
                ..Default::default()
            }],
            unused_handlers: vec![CommandGap {
                name: "unused".into(),
                locations: vec![("c.rs".into(), 3)],
                ..Default::default()
            }],
            ..Default::default()
        };
        let wins = extract_quick_wins(&[section]);

        assert_eq!(wins.len(), 3);
        // Missing first (priority 1)
        assert_eq!(wins[0].target, "missing");
        assert_eq!(wins[0].priority, 1);
        // Unregistered second (priority 2)
        assert_eq!(wins[1].target, "unregistered");
        assert_eq!(wins[1].priority, 2);
        // Unused third (priority 3)
        assert_eq!(wins[2].target, "unused");
        assert_eq!(wins[2].priority, 3);
    }

    #[test]
    fn extract_quick_wins_respects_limits() {
        // Create 15 missing handlers - should stop at 10
        let section = ReportSection {
            root: "test".into(),
            missing_handlers: (0..15)
                .map(|i| CommandGap {
                    name: format!("handler_{}", i),
                    locations: vec![(format!("file_{}.ts", i), i)],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let wins = extract_quick_wins(&[section]);

        // Should have exactly 10 (limit for missing handlers)
        assert_eq!(wins.len(), 10);
        assert_eq!(wins[9].priority, 10);
    }

    #[test]
    fn extract_quick_wins_unknown_location() {
        let section = ReportSection {
            root: "test".into(),
            missing_handlers: vec![CommandGap {
                name: "no_location".into(),
                locations: vec![], // Empty locations
                ..Default::default()
            }],
            ..Default::default()
        };
        let wins = extract_quick_wins(&[section]);

        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].location, "unknown");
    }
}
