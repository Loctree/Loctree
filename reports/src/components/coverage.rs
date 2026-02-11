//! Coverage gaps panel component - shows test coverage gaps

use crate::components::icons::{ICON_FLASK, Icon};
use crate::types::{CoverageGap, GapKind, Severity};
use leptos::prelude::*;

/// Panel displaying test coverage gaps
#[component]
pub fn Coverage(coverage_gaps: Vec<CoverageGap>) -> impl IntoView {
    let count = coverage_gaps.len();

    // Count by severity
    let critical_count = coverage_gaps
        .iter()
        .filter(|g| matches!(g.severity, Severity::Critical))
        .count();
    let high_count = coverage_gaps
        .iter()
        .filter(|g| matches!(g.severity, Severity::High))
        .count();
    let medium_count = coverage_gaps
        .iter()
        .filter(|g| matches!(g.severity, Severity::Medium))
        .count();
    let low_count = coverage_gaps
        .iter()
        .filter(|g| matches!(g.severity, Severity::Low))
        .count();

    view! {
        <div class="panel">
            <h3>
                <Icon path=ICON_FLASK />
                "Coverage Gaps"
            </h3>

            {if count == 0 {
                view! {
                    <div class="graph-empty">
                        <div style="text-align: center; padding: 32px;">
                            <Icon path=ICON_FLASK size="48" color="var(--theme-text-tertiary)" />
                            <p style="margin-top: 16px; color: var(--theme-text-secondary);">
                                "No coverage gaps detected"
                            </p>
                            <p class="muted" style="font-size: 12px; margin-top: 8px;">
                                "All production code has test coverage"
                            </p>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <CoverageTable
                        coverage_gaps=coverage_gaps
                        total_count=count
                        critical_count=critical_count
                        high_count=high_count
                        medium_count=medium_count
                        low_count=low_count
                    />
                }.into_any()
            }}
        </div>
    }
}

/// Table showing coverage gaps with filtering
#[component]
fn CoverageTable(
    coverage_gaps: Vec<CoverageGap>,
    total_count: usize,
    critical_count: usize,
    high_count: usize,
    medium_count: usize,
    low_count: usize,
) -> impl IntoView {
    view! {
        <div class="coverage-summary" data-coverage-default-filter="all">
            <p class="muted">
                {format!(
                    "{} coverage gap{} found: {} critical, {} high, {} medium, {} low",
                    total_count,
                    if total_count == 1 { "" } else { "s" },
                    critical_count,
                    high_count,
                    medium_count,
                    low_count
                )}
            </p>

            <div class="filter-buttons" style="margin-top: 12px; display: flex; gap: 8px; flex-wrap: wrap;">
                <button
                    class="filter-btn active"
                    data-coverage-filter="all"
                >
                    "All (" {total_count} ")"
                </button>
                {if critical_count > 0 {
                    view! {
                        <button
                            class="filter-btn severity-critical"
                            data-coverage-filter="critical"
                        >
                            "Critical (" {critical_count} ")"
                        </button>
                    }.into_any()
                } else {
                    view! { "" }.into_any()
                }}
                {if high_count > 0 {
                    view! {
                        <button
                            class="filter-btn severity-high"
                            data-coverage-filter="high"
                        >
                            "High (" {high_count} ")"
                        </button>
                    }.into_any()
                } else {
                    view! { "" }.into_any()
                }}
                {if medium_count > 0 {
                    view! {
                        <button
                            class="filter-btn severity-medium"
                            data-coverage-filter="medium"
                        >
                            "Medium (" {medium_count} ")"
                        </button>
                    }.into_any()
                } else {
                    view! { "" }.into_any()
                }}
                {if low_count > 0 {
                    view! {
                        <button
                            class="filter-btn severity-low"
                            data-coverage-filter="low"
                        >
                            "Low (" {low_count} ")"
                        </button>
                    }.into_any()
                } else {
                    view! { "" }.into_any()
                }}
            </div>
        </div>

        <table class="data-table coverage-table">
            <thead>
                <tr>
                    <th>"Target"</th>
                    <th>"Location"</th>
                    <th>"Kind"</th>
                    <th>"Severity"</th>
                    <th>"Recommendation"</th>
                </tr>
            </thead>
            <tbody>
                {coverage_gaps.into_iter().map(|gap| {
                    let severity_class = match gap.severity {
                        Severity::Critical => "severity-critical",
                        Severity::High => "severity-high",
                        Severity::Medium => "severity-medium",
                        Severity::Low => "severity-low",
                    };
                    let severity_slug = match gap.severity {
                        Severity::Critical => "critical",
                        Severity::High => "high",
                        Severity::Medium => "medium",
                        Severity::Low => "low",
                    };

                    let severity_text = match gap.severity {
                        Severity::Critical => "Critical",
                        Severity::High => "High",
                        Severity::Medium => "Medium",
                        Severity::Low => "Low",
                    };

                    let kind_text = match gap.kind {
                        GapKind::HandlerWithoutTest => "Handler without test",
                        GapKind::EventWithoutTest => "Event without test",
                        GapKind::ExportWithoutTest => "Export without test",
                        GapKind::TestedButUnused => "Tested but unused",
                    };

                    view! {
                        <tr class=severity_class data-coverage-severity=severity_slug>
                            <td class="symbol-cell">
                                <code>{gap.target.clone()}</code>
                            </td>
                            <td class="file-cell">
                                <code>{gap.location.clone()}</code>
                            </td>
                            <td class="kind-cell">
                                {kind_text}
                            </td>
                            <td class="severity-cell">
                                <span class=format!("severity-badge {}", severity_class)>
                                    {severity_text}
                                </span>
                            </td>
                            <td class="recommendation-cell">
                                {gap.recommendation.clone()}
                                {gap.context.as_ref().map(|ctx| view! {
                                    <div class="muted" style="font-size: 11px; margin-top: 4px;">
                                        {ctx.clone()}
                                    </div>
                                })}
                            </td>
                        </tr>
                    }
                }).collect::<Vec<_>>()}
            </tbody>
        </table>
    }
}
