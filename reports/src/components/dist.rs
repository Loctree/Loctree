//! Bundle distribution panel component.

use crate::components::icons::{ICON_PACKAGE, Icon};
use crate::types::{DistAnalysisLevel, DistCandidateClass, DistConfidence, DistReport};
use leptos::prelude::*;

fn candidate_class_label(class_name: DistCandidateClass) -> &'static str {
    match class_name {
        DistCandidateClass::DeadInAllChunks => "dead_in_all_chunks",
        DistCandidateClass::BootPathOnly => "boot_path_only",
        DistCandidateClass::FeatureLocal => "feature_local",
        DistCandidateClass::FakeLazy => "fake_lazy",
        DistCandidateClass::VerifyFirst => "verify_first",
    }
}

fn confidence_label(confidence: DistConfidence) -> &'static str {
    match confidence {
        DistConfidence::Low => "low",
        DistConfidence::Medium => "medium",
        DistConfidence::High => "high",
    }
}

/// Panel displaying bundle distribution / tree-shaking results.
#[component]
pub fn DistPanel(dist: Option<DistReport>) -> impl IntoView {
    view! {
        <div class="panel">
            <h3>
                <Icon path=ICON_PACKAGE />
                "Bundle distribution"
            </h3>

            {match dist {
                Some(dist) => view! { <DistContent dist=dist /> }.into_any(),
                None => view! {
                    <div class="graph-empty">
                        <div style="text-align: center; padding: 32px;">
                            <Icon path=ICON_PACKAGE size="48" color="var(--theme-text-tertiary)" />
                            <p style="margin-top: 16px; color: var(--theme-text-secondary);">
                                "No bundle distribution data available"
                            </p>
                            <p class="muted" style="font-size: 12px; margin-top: 8px;">
                                "Run loct dist with one or more production source maps to populate this panel."
                            </p>
                        </div>
                    </div>
                }
                .into_any(),
            }}
        </div>
    }
}

#[component]
fn DistContent(dist: DistReport) -> impl IntoView {
    let source_map_paths = dist.source_map_paths.clone();
    let impacted_files = dist.impacted_files.clone();
    let dead_exports = dist.dead_exports.clone();
    let candidate_counts = dist.candidate_counts.clone();
    let candidates = dist.candidates.clone();
    let analysis_level = match dist.analysis_level {
        DistAnalysisLevel::File => "file",
        DistAnalysisLevel::Line => "line",
        DistAnalysisLevel::Symbol => "symbol",
        DistAnalysisLevel::Mixed => "mixed",
    };

    view! {
        <div class="dist-summary" style="display: grid; gap: 12px;">
            <div class="stats-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px;">
                <StatCard label="Source maps" value=dist.source_maps.to_string() />
                <StatCard label="Source exports" value=dist.source_exports.to_string() />
                <StatCard label="Bundled exports" value=dist.bundled_exports.to_string() />
                <StatCard label="Tree-shaken" value=format!("{} ({}%)", dist.tree_shaken_exports, dist.tree_shaken_pct) />
                <StatCard label="Coverage" value=format!("{}%", dist.coverage_pct) />
                <StatCard label="Analysis level" value=analysis_level.to_string() />
            </div>

            {if !dist.src_dir.is_empty() {
                view! {
                    <p class="muted">
                        <strong>"Source scope:"</strong>
                        " "
                        <code>{dist.src_dir.clone()}</code>
                    </p>
                }.into_any()
            } else {
                view! { "" }.into_any()
            }}

            {if !source_map_paths.is_empty() {
                view! {
                    <div>
                        <h4>"Source maps"</h4>
                        <ul class="clean-list">
                            {source_map_paths.into_iter().map(|path| {
                                view! {
                                    <li><code>{path}</code></li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                }.into_any()
            } else {
                view! { "" }.into_any()
            }}

            {if !candidate_counts.is_empty() {
                view! {
                    <div>
                        <h4>"Runtime candidate classes"</h4>
                        <div class="summary-grid">
                            {candidate_counts.into_iter().map(|(class_name, count)| {
                                view! {
                                    <div class="summary-stat">
                                        <span class="stat-value">{count}</span>
                                        <span class="stat-label">{class_name}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { "" }.into_any()
            }}

            {if !impacted_files.is_empty() {
                view! {
                    <div>
                        <h4>"Impacted files"</h4>
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"File"</th>
                                    <th>"Source exports"</th>
                                    <th>"Bundled exports"</th>
                                    <th>"Tree-shaken"</th>
                                    <th>"Status"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {impacted_files.into_iter().map(|file| {
                                    view! {
                                        <tr>
                                            <td><code>{file.file}</code></td>
                                            <td>{file.source_exports}</td>
                                            <td>{file.bundled_exports}</td>
                                            <td>{file.tree_shaken_exports}</td>
                                            <td>{file.status}</td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            } else {
                view! { "" }.into_any()
            }}

            {if !dead_exports.is_empty() {
                view! {
                    <div>
                        <h4>"Exports removed from all analyzed bundles"</h4>
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"Symbol"</th>
                                    <th>"Kind"</th>
                                    <th>"File"</th>
                                    <th>"Line"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {dead_exports.into_iter().take(25).map(|export| {
                                    view! {
                                        <tr>
                                            <td><code>{export.name}</code></td>
                                            <td>{export.kind}</td>
                                            <td><code>{export.file}</code></td>
                                            <td>{export.line}</td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            } else {
                view! {
                    <p class="muted">
                        "No exports were removed from all analyzed bundles."
                    </p>
                }.into_any()
            }}

            {if !candidates.is_empty() {
                view! {
                    <div>
                        <h4>"Ranked runtime candidates"</h4>
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"Class"</th>
                                    <th>"Symbol"</th>
                                    <th>"Confidence"</th>
                                    <th>"Chunks"</th>
                                    <th>"File"</th>
                                    <th>"Notes"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {candidates.into_iter().take(25).map(|candidate| {
                                    let notes = if candidate.notes.is_empty() {
                                        "-".to_string()
                                    } else {
                                        candidate.notes.join("; ")
                                    };
                                    let chunks = if candidate.chunk_names.is_empty() {
                                        "-".to_string()
                                    } else {
                                        candidate.chunk_names.join(", ")
                                    };

                                    view! {
                                        <tr>
                                            <td><code>{candidate_class_label(candidate.class_name)}</code></td>
                                            <td><code>{candidate.name}</code></td>
                                            <td>{confidence_label(candidate.confidence)}</td>
                                            <td>{chunks}</td>
                                            <td><code>{format!("{}:{}", candidate.file, candidate.line)}</code></td>
                                            <td>{notes}</td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                }.into_any()
            } else {
                view! { "" }.into_any()
            }}
        </div>
    }
}

#[component]
fn StatCard(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="card" style="padding: 12px;">
            <div class="muted" style="font-size: 12px; text-transform: uppercase;">{label}</div>
            <div style="font-size: 24px; font-weight: 700; margin-top: 4px;">{value}</div>
        </div>
    }
}
