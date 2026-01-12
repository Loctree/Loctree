//! Audit panel - shows actionable health checklist
//!
//! Provides a visual summary of codebase health with actionable items
//! organized by severity (critical, warning, quick wins).

use leptos::prelude::*;

use crate::types::ReportSection;

/// Category of quick win suggestion
#[derive(Clone, Debug)]
pub struct QuickWin {
    /// Actionable text describing the quick win
    pub text: String,
    /// Category: "cleanup", "refactor", "optimize"
    pub category: String,
}

/// Audit panel showing health score and actionable items
///
/// Displays:
/// - Overall health score with visual badge
/// - Critical items (high-confidence dead exports)
/// - Warning items (cycles, twins)
/// - Quick win suggestions
#[component]
pub fn AuditPanel(section: ReportSection) -> impl IntoView {
    let health = section.health_score.unwrap_or(50);

    // Critical items: dead exports with high/very-high confidence
    let critical_items: Vec<_> = section
        .dead_exports
        .iter()
        .filter(|d| d.confidence == "high" || d.confidence == "very-high")
        .cloned()
        .collect();
    let critical_count = critical_items.len();

    // Warning items: cycles + twins
    let strict_cycle_count = section.circular_imports.len();
    let lazy_cycle_count = section.lazy_circular_imports.len();
    let twin_count = section
        .twins
        .as_ref()
        .map(|t| t.exact_twins.len())
        .unwrap_or(0);
    let warning_count = strict_cycle_count + lazy_cycle_count + twin_count;

    // Generate quick wins based on analysis
    let quick_wins = generate_quick_wins(&section);
    let quick_win_count = quick_wins.len();

    // Clone values for use in view closures
    let critical_items_for_view = critical_items.clone();
    let section_for_cycles = section.clone();
    let section_for_twins = section.clone();

    view! {
        <div class="audit-panel panel">
            <div class="audit-header">
                <h3>"Audit Summary"</h3>
                <div
                    class="health-badge"
                    class:critical=(health < 40)
                    class:warning=(40..70).contains(&health)
                    class:good=(health >= 70)
                >
                    <span class="health-value">{health}</span>
                    <span class="health-max">"/100"</span>
                </div>
            </div>

            // Critical section
            {(critical_count > 0).then(|| {
                let items = critical_items_for_view.clone();
                view! {
                    <section class="audit-section audit-critical">
                        <h4 class="audit-section-title">
                            <span class="audit-icon">"!!"</span>
                            "Critical ("{critical_count}")"
                        </h4>
                        <p class="audit-section-desc">
                            "High-confidence dead exports that should be removed"
                        </p>
                        <ul class="audit-list">
                            {items.into_iter().take(10).map(|dead| {
                                let file_short = shorten_path(&dead.file);
                                let line_str = dead.line.map(|l| format!(":{}", l)).unwrap_or_default();
                                view! {
                                    <li class="audit-item">
                                        <label class="audit-checkbox-label">
                                            <input type="checkbox" class="audit-checkbox" />
                                            <code class="audit-symbol">{dead.symbol.clone()}</code>
                                            <span class="audit-location">
                                                {file_short}{line_str}
                                            </span>
                                        </label>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                            {(critical_count > 10).then(|| view! {
                                <li class="audit-item audit-more">
                                    "... and "{critical_count - 10}" more"
                                </li>
                            })}
                        </ul>
                    </section>
                }
            })}

            // Warnings section
            {(warning_count > 0).then(|| {
                let strict_cycles = section_for_cycles.circular_imports.clone();
                let lazy_cycles = section_for_cycles.lazy_circular_imports.clone();
                let twins = section_for_twins.twins.clone();
                view! {
                    <section class="audit-section audit-warning">
                        <h4 class="audit-section-title">
                            <span class="audit-icon">"!"</span>
                            "Warnings ("{warning_count}")"
                        </h4>
                        <p class="audit-section-desc">
                            "Potential issues worth reviewing"
                        </p>
                        <ul class="audit-list">
                            // Strict cycles
                            {(!strict_cycles.is_empty()).then(|| view! {
                                <li class="audit-item audit-category">
                                    <span class="audit-category-icon">"Strict Cycles: "</span>
                                    <span class="audit-count">{strict_cycles.len()}</span>
                                </li>
                            })}
                            {strict_cycles.into_iter().take(3).map(|cycle| {
                                let cycle_str = cycle.join(" -> ");
                                view! {
                                    <li class="audit-item audit-sub-item">
                                        <label class="audit-checkbox-label">
                                            <input type="checkbox" class="audit-checkbox" />
                                            <code class="audit-cycle">{cycle_str}</code>
                                        </label>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}

                            // Lazy cycles
                            {(!lazy_cycles.is_empty()).then(|| view! {
                                <li class="audit-item audit-category">
                                    <span class="audit-category-icon">"Lazy Cycles: "</span>
                                    <span class="audit-count">{lazy_cycles.len()}</span>
                                </li>
                            })}
                            {lazy_cycles.into_iter().take(3).map(|cycle| {
                                let cycle_str = cycle.join(" -> ");
                                view! {
                                    <li class="audit-item audit-sub-item">
                                        <label class="audit-checkbox-label">
                                            <input type="checkbox" class="audit-checkbox" />
                                            <code class="audit-cycle">{cycle_str}</code>
                                        </label>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}

                            // Twins
                            {twins.as_ref().map(|t| {
                                let exact_twins = t.exact_twins.clone();
                                (!exact_twins.is_empty()).then(|| view! {
                                    <li class="audit-item audit-category">
                                        <span class="audit-category-icon">"Exact Twins: "</span>
                                        <span class="audit-count">{exact_twins.len()}</span>
                                    </li>
                                    {exact_twins.into_iter().take(5).map(|twin| {
                                        let loc_count = twin.locations.len();
                                        view! {
                                            <li class="audit-item audit-sub-item">
                                                <label class="audit-checkbox-label">
                                                    <input type="checkbox" class="audit-checkbox" />
                                                    <code class="audit-symbol">{twin.name}</code>
                                                    <span class="audit-location">
                                                        " in "{loc_count}" files"
                                                    </span>
                                                </label>
                                            </li>
                                        }
                                    }).collect::<Vec<_>>()}
                                })
                            })}
                        </ul>
                    </section>
                }
            })}

            // Quick wins section
            {(!quick_wins.is_empty()).then(|| {
                let wins = quick_wins.clone();
                view! {
                    <section class="audit-section audit-quick-wins">
                        <h4 class="audit-section-title">
                            <span class="audit-icon">"*"</span>
                            "Quick Wins ("{quick_win_count}")"
                        </h4>
                        <p class="audit-section-desc">
                            "Low-effort improvements to boost health score"
                        </p>
                        <ul class="audit-list">
                            {wins.into_iter().map(|win| {
                                let category_class = format!("audit-category-{}", win.category);
                                view! {
                                    <li class="audit-item">
                                        <label class="audit-checkbox-label">
                                            <input type="checkbox" class="audit-checkbox" />
                                            <span class=category_class>{win.text}</span>
                                        </label>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </section>
                }
            })}

            // Empty state
            {(critical_count == 0 && warning_count == 0 && quick_wins.is_empty()).then(|| view! {
                <div class="audit-empty">
                    <p>"Codebase is in good shape! No critical issues detected."</p>
                </div>
            })}

            // Footer with tips
            <div class="audit-footer">
                <p class="audit-tip">
                    "Tip: Check items as you fix them to track progress. "
                    "Run "<code>"loct health"</code>" to regenerate this report."
                </p>
            </div>
        </div>
    }
}

/// Generate quick win suggestions based on analysis
fn generate_quick_wins(section: &ReportSection) -> Vec<QuickWin> {
    let mut wins = Vec::new();

    // Suggest removing unused exports
    let low_confidence_dead = section
        .dead_exports
        .iter()
        .filter(|d| d.confidence != "high" && d.confidence != "very-high")
        .count();
    if low_confidence_dead > 0 {
        wins.push(QuickWin {
            text: format!(
                "Review {} medium-confidence unused exports",
                low_confidence_dead
            ),
            category: "cleanup".into(),
        });
    }

    // Suggest barrel file cleanup
    if let Some(twins) = &section.twins {
        if !twins.barrel_chaos.missing_barrels.is_empty() {
            wins.push(QuickWin {
                text: format!(
                    "Add barrel files to {} directories",
                    twins.barrel_chaos.missing_barrels.len()
                ),
                category: "refactor".into(),
            });
        }
        if !twins.barrel_chaos.deep_chains.is_empty() {
            wins.push(QuickWin {
                text: format!(
                    "Simplify {} deep re-export chains",
                    twins.barrel_chaos.deep_chains.len()
                ),
                category: "refactor".into(),
            });
        }
    }

    // Suggest reviewing crowds
    let high_score_crowds = section.crowds.iter().filter(|c| c.score >= 7.0).count();
    if high_score_crowds > 0 {
        wins.push(QuickWin {
            text: format!("Review {} high-severity naming crowds", high_score_crowds),
            category: "refactor".into(),
        });
    }

    // Suggest fixing coverage gaps
    let critical_gaps = section
        .coverage_gaps
        .iter()
        .filter(|g| g.severity == crate::types::Severity::Critical)
        .count();
    if critical_gaps > 0 {
        wins.push(QuickWin {
            text: format!("Add tests for {} untested handlers", critical_gaps),
            category: "test".into(),
        });
    }

    // Suggest cleaning up duplicates
    let high_score_dups = section.ranked_dups.iter().filter(|d| d.score >= 10).count();
    if high_score_dups > 0 {
        wins.push(QuickWin {
            text: format!(
                "Consolidate {} high-priority duplicate exports",
                high_score_dups
            ),
            category: "cleanup".into(),
        });
    }

    // Generic suggestion if nothing specific
    if wins.is_empty() && section.health_score.unwrap_or(100) < 100 {
        wins.push(QuickWin {
            text: "Run full analysis with `loct analyze` for more suggestions".into(),
            category: "optimize".into(),
        });
    }

    wins
}

/// Shorten a file path for display
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        path.to_string()
    } else {
        parts
            .iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("/")
    }
}
