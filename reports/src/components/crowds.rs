//! Crowds component - displays naming collision analysis
//!
//! Shows groups of files with similar names/patterns that may indicate
//! fragmentation, naming collisions, or copy-paste duplication.

use crate::types::{Crowd, CrowdIssue, CrowdMember, MatchReason};
use leptos::prelude::*;

/// Score color classification for crowd severity
fn score_color(score: f32) -> &'static str {
    if score < 4.0 {
        "#27ae60" // Green - low severity
    } else if score < 7.0 {
        "#e67e22" // Orange - medium severity
    } else {
        "#c0392b" // Red - high severity
    }
}

/// Format issue badge text
fn issue_badge_text(issue: &CrowdIssue) -> String {
    match issue {
        CrowdIssue::NameCollision { files } => {
            format!("Name Collision ({} files)", files.len())
        }
        CrowdIssue::UsageAsymmetry {
            primary: _,
            underused,
        } => {
            format!("Usage Asymmetry ({}+{} files)", 1, underused.len())
        }
        CrowdIssue::ExportOverlap { files: _, overlap } => {
            format!("Export Overlap ({} symbols)", overlap.len())
        }
        CrowdIssue::Fragmentation { categories } => {
            format!("Fragmented ({} categories)", categories.len())
        }
    }
}

/// Issue severity class for styling
fn issue_severity_class(issue: &CrowdIssue) -> &'static str {
    match issue {
        CrowdIssue::NameCollision { .. } => "issue-critical",
        CrowdIssue::UsageAsymmetry { .. } => "issue-warning",
        CrowdIssue::ExportOverlap { .. } => "issue-warning",
        CrowdIssue::Fragmentation { .. } => "issue-info",
    }
}

/// Format match reason for display
fn format_match_reason(reason: &MatchReason) -> String {
    match reason {
        MatchReason::NameMatch { matched } => {
            format!("Name: {}", matched)
        }
        MatchReason::ImportSimilarity { similarity } => {
            format!("Import similarity ({:.0}%)", similarity * 100.0)
        }
        MatchReason::ExportSimilarity { similar_to } => {
            format!("Similar exports to {}", similar_to)
        }
    }
}

/// Main crowds panel component
#[component]
pub fn Crowds(crowds: Vec<Crowd>) -> impl IntoView {
    let count = crowds.len();

    view! {
        <div class="content-container">
            <div class="panel">
                <h3>"Crowds Analysis"</h3>
                {if crowds.is_empty() {
                    view! {
                        <p class="muted">"No crowds detected. Your codebase has well-distributed naming patterns."</p>
                    }.into_any()
                } else {
                    view! {
                        <p class="muted">
                            {format!("{} crowd patterns found - files with similar names that may indicate fragmentation or duplication", count)}
                        </p>
                        <div class="crowds-list">
                            {crowds.into_iter().map(|crowd| {
                                view! { <CrowdCard crowd=crowd /> }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

/// Individual crowd card component
#[component]
fn CrowdCard(crowd: Crowd) -> impl IntoView {
    let score = crowd.score;
    let color = score_color(score);
    let pattern = crowd.pattern.clone();
    let member_count = crowd.members.len();

    view! {
        <div class="crowd-card">
            <div class="crowd-header">
                <div class="crowd-pattern">
                    <code>{pattern}</code>
                    <span class="crowd-member-count muted">
                        {format!("{} files", member_count)}
                    </span>
                </div>
                <div class="crowd-score" style=format!("--score-color: {}", color)>
                    <span class="score-value">{format!("{:.1}", score)}</span>
                    <span class="score-label">"severity"</span>
                </div>
            </div>

            {(!crowd.issues.is_empty()).then(|| view! {
                <div class="crowd-issues">
                    {crowd.issues.into_iter().map(|issue| {
                        let badge_text = issue_badge_text(&issue);
                        let severity_class = issue_severity_class(&issue);
                        view! {
                            <span class=format!("issue-badge {}", severity_class)>
                                {badge_text}
                            </span>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            })}

            <div class="crowd-members">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>"File"</th>
                            <th>"Match Reason"</th>
                            <th>"Importers"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {crowd.members.into_iter().map(|member| {
                            view! { <CrowdMemberRow member=member /> }
                        }).collect::<Vec<_>>()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// Individual crowd member row
#[component]
fn CrowdMemberRow(member: CrowdMember) -> impl IntoView {
    let match_reason_text = format_match_reason(&member.match_reason);

    view! {
        <tr>
            <td><code class="file-path">{member.file}</code></td>
            <td class="muted">{match_reason_text}</td>
            <td>{member.importer_count}</td>
        </tr>
    }
}
