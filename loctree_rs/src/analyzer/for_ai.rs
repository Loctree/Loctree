//! AI-optimized hierarchical output format.
//!
//! Transforms analysis results into structured JSON that AI agents can:
//! - Parse easily with regex/jq
//! - Navigate via slice references
//! - Get actionable quick wins
//!
//! Developed with 💀 by The Loctree Team (c)2025

use serde::Serialize;
use std::collections::{HashMap, HashSet};

use super::report::{Confidence, DupSeverity, RankedDup, ReportSection};
use super::root_scan::normalize_module_id;
use super::twins::{TwinCategory, categorize_twin};
use crate::types::{FileAnalysis, SignatureUse, SignatureUseKind};

/// Top-level AI summary - the entry point for agents
#[derive(Serialize)]
pub struct ForAiReport {
    /// Project root path
    pub project: String,
    /// ISO timestamp
    pub generated_at: String,
    /// High-level summary with priorities
    pub summary: ForAiSummary,
    /// Per-root section references (link to slices)
    pub sections: Vec<ForAiSectionRef>,
    /// Immediate actionable items
    pub quick_wins: Vec<QuickWin>,
    /// Files with most connections (good context anchors)
    pub hub_files: Vec<HubFile>,
    /// Agent-ready bundle with condensed lists (handlers, dupes, dead, dynamic, cycles)
    pub bundle: AgentBundle,
}

/// Summary with counts and priority guidance
#[derive(Serialize)]
pub struct ForAiSummary {
    pub files_analyzed: usize,
    pub total_loc: usize,
    pub dead_exports: usize,
    pub duplicate_exports: usize,
    pub circular_imports: usize,
    pub missing_handlers: usize,
    pub unregistered_handlers: usize,
    pub unused_handlers: usize,
    pub unused_high_confidence: usize,
    pub cascade_imports: usize,
    pub dynamic_imports: usize,
    /// Dead parrots from twins analysis (exports with 0 imports)
    pub twins_dead_parrots: usize,
    /// Same-language exact twins (likely real duplicates needing consolidation)
    pub twins_same_language: usize,
    /// Cross-language twins (FE/BE pairs, usually intentional)
    pub twins_cross_language: usize,
    /// Priority message for the AI
    pub priority: String,
    /// Health score 0-100
    pub health_score: u8,
}

/// Reference to a section with command to get details
#[derive(Serialize)]
pub struct ForAiSectionRef {
    pub id: String,
    pub root: String,
    pub files: usize,
    pub loc: usize,
    pub issues: usize,
    /// Command to drill down
    pub slice_cmd: String,
}

/// Immediate actionable item
#[derive(Serialize, Clone)]
pub struct QuickWin {
    pub priority: u8, // 1=highest
    /// Kind of issue: missing_handler, unregistered_handler, unused_handler, dead_export, circular_import, opaque_passthrough
    pub kind: String,
    pub action: String,
    pub target: String,
    pub location: String,
    pub impact: String,
    /// Why this is a problem
    pub why: String,
    /// Specific fix suggestion
    pub fix_hint: String,
    /// Estimated complexity: trivial, easy, medium
    pub complexity: String,
    /// Command to investigate further
    pub trace_cmd: Option<String>,
    /// IDE integration URL (loctree://open?f={file}&l={line})
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_url: Option<String>,
}

/// High-connectivity file that makes good context anchor
#[derive(Serialize)]
pub struct HubFile {
    pub path: String,
    pub loc: usize,
    pub imports_count: usize,
    pub exports_count: usize,
    pub importers_count: usize, // Files that import this
    pub commands_count: usize,
    /// Command to get full context
    pub slice_cmd: String,
}

/// Condensed agent bundle - one JSON instead of multiple artifacts.
#[derive(Serialize)]
pub struct AgentBundle {
    pub handlers: AgentHandlerGroups,
    pub duplicates: Vec<AgentDuplicate>,
    pub dead_exports: Vec<AgentDeadExport>,
    pub dynamic_imports: Vec<AgentDynamicImport>,
    pub largest_files: Vec<AgentFile>,
    pub cycles: Vec<AgentCycle>,
}

#[derive(Serialize, Default)]
pub struct AgentHandlerGroups {
    pub missing: Vec<AgentHandler>,
    pub unused: Vec<AgentHandler>,
    pub unregistered: Vec<AgentHandler>,
}

#[derive(Serialize, Clone)]
pub struct AgentHandler {
    pub name: String,
    pub status: String,
    pub frontend: Vec<AgentLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<AgentBackend>,
}

#[derive(Serialize, Clone)]
pub struct AgentBackend {
    pub path: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct AgentLocation {
    pub path: String,
    pub line: usize,
}

#[derive(Serialize)]
pub struct AgentDuplicate {
    pub name: String,
    pub canonical: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_line: Option<usize>,
    pub score: usize,
    pub severity: String,
    pub files: usize,
}

#[derive(Serialize)]
pub struct AgentDeadExport {
    pub symbol: String,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub confidence: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct AgentDynamicImport {
    pub file: String,
    pub resolved: Vec<String>,
    pub unresolved: Vec<String>,
}

#[derive(Serialize)]
pub struct AgentFile {
    pub path: String,
    pub loc: usize,
}

#[derive(Serialize)]
pub struct AgentCycle {
    pub kind: String,
    pub members: Vec<String>,
}

/// Generate AI report from analysis results
pub fn generate_for_ai_report(
    project_root: &str,
    sections: &[ReportSection],
    analyses: &[FileAnalysis],
) -> ForAiReport {
    let now = time::OffsetDateTime::now_utc();
    let generated_at = now
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".to_string());

    let summary = compute_summary(sections, analyses);
    let section_refs = build_section_refs(sections);
    let quick_wins = extract_quick_wins(sections, analyses);
    let hub_files = find_hub_files(analyses);
    let bundle = build_agent_bundle(sections, analyses);

    ForAiReport {
        project: project_root.to_string(),
        generated_at,
        summary,
        sections: section_refs,
        quick_wins,
        hub_files,
        bundle,
    }
}

fn compute_summary(sections: &[ReportSection], analyses: &[FileAnalysis]) -> ForAiSummary {
    // When sections are empty but we have analyses (e.g., --full-scan), use analyses.len()
    let files_analyzed: usize = if sections.is_empty() {
        analyses.len()
    } else {
        sections.iter().map(|s| s.files_analyzed).sum()
    };
    let total_loc: usize = analyses.iter().map(|a| a.loc).sum();
    let dead_exports: usize = sections.iter().map(|s| s.dead_exports.len()).sum();
    let duplicate_exports: usize = sections.iter().map(|s| s.ranked_dups.len()).sum();
    let missing_handlers: usize = sections.iter().map(|s| s.missing_handlers.len()).sum();
    let unregistered_handlers: usize = sections.iter().map(|s| s.unregistered_handlers.len()).sum();
    let unused_handlers: usize = sections.iter().map(|s| s.unused_handlers.len()).sum();
    let unused_high_confidence: usize = sections
        .iter()
        .flat_map(|s| &s.unused_handlers)
        .filter(|h| h.confidence == Some(Confidence::High))
        .count();
    let cascade_imports: usize = sections.iter().map(|s| s.cascades.len()).sum();
    let dynamic_imports: usize = sections.iter().map(|s| s.dynamic.len()).sum();
    let circular_imports: usize = sections.iter().map(|s| s.circular_imports.len()).sum();

    // Collect twins data from sections
    let twins_dead_parrots: usize = sections
        .iter()
        .filter_map(|s| s.twins_data.as_ref())
        .map(|t| t.dead_parrots.len())
        .sum();

    let (twins_same_language, twins_cross_language): (usize, usize) = sections
        .iter()
        .filter_map(|s| s.twins_data.as_ref())
        .flat_map(|t| &t.exact_twins)
        .fold((0, 0), |(same, cross), twin| match categorize_twin(twin) {
            TwinCategory::SameLanguage(_) => (same + 1, cross),
            TwinCategory::CrossLanguage => (same, cross + 1),
        });

    // Generate priority message (now includes twins!)
    let priority = if missing_handlers > 0 {
        format!(
            "CRITICAL: Fix {} missing handlers first (runtime errors at invoke). Then {} unused handlers (tech debt).",
            missing_handlers, unused_handlers
        )
    } else if unregistered_handlers > 0 {
        format!(
            "WARNING: {} handlers defined but not registered in generate_handler![]. They won't work at runtime.",
            unregistered_handlers
        )
    } else if unused_high_confidence > 0 {
        format!(
            "CLEANUP: {} unused handlers (high confidence) can be safely removed. {} dead exports, {} duplicate exports.",
            unused_high_confidence, dead_exports, duplicate_exports
        )
    } else if twins_same_language > 0 {
        format!(
            "TECH DEBT: {} same-language twins (consolidate duplicates). {} dead parrots (0 imports). {} cross-lang pairs (likely OK).",
            twins_same_language, twins_dead_parrots, twins_cross_language
        )
    } else if twins_dead_parrots > 0 {
        format!(
            "TECH DEBT: {} dead parrots (exports with 0 imports). Consider removing unused code.",
            twins_dead_parrots
        )
    } else if dead_exports > 0 {
        format!(
            "TECH DEBT: {} dead exports (unused). {} duplicate exports across files.",
            dead_exports, duplicate_exports
        )
    } else if duplicate_exports > 0 {
        format!(
            "TECH DEBT: {} duplicate exports across files. Consider consolidating to reduce confusion.",
            duplicate_exports
        )
    } else if circular_imports > 0 {
        format!(
            "TECH DEBT: {} circular import cycles. Consider refactoring to break cycles.",
            circular_imports
        )
    } else {
        "HEALTHY: No critical issues found. Good job!".to_string()
    };

    // Health score (weighted heuristic):
    // - Missing handlers: 20 points each (critical runtime errors)
    // - Unregistered handlers: 15 points each (won't work but defined)
    // - Unused handlers (high confidence): 5 points each (dead code)
    // - Dead exports: 2 points each (unused exports)
    // - Duplicate exports: 1 point per 5 (naming confusion)
    // - Circular imports: 3 points each (architectural smell)
    // - Dead parrots (twins): 1 point each (unused but less severe than dead_exports)
    // - Same-language twins: 2 points each (real duplicates needing consolidation)
    let issue_penalty = missing_handlers * 20
        + unregistered_handlers * 15
        + unused_high_confidence * 5
        + dead_exports * 2
        + (duplicate_exports / 5).min(20)
        + circular_imports * 3
        + twins_dead_parrots
        + twins_same_language * 2;
    let health_score = 100u8.saturating_sub(issue_penalty.min(100) as u8);

    ForAiSummary {
        files_analyzed,
        total_loc,
        dead_exports,
        duplicate_exports,
        circular_imports,
        missing_handlers,
        unregistered_handlers,
        unused_handlers,
        unused_high_confidence,
        cascade_imports,
        dynamic_imports,
        twins_dead_parrots,
        twins_same_language,
        twins_cross_language,
        priority,
        health_score,
    }
}

fn build_section_refs(sections: &[ReportSection]) -> Vec<ForAiSectionRef> {
    sections
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            let issues = s.missing_handlers.len()
                + s.unregistered_handlers.len()
                + s.unused_handlers.len()
                + s.ranked_dups.len();

            let loc: usize = s
                .graph
                .as_ref()
                .map(|g| g.nodes.iter().map(|n| n.loc).sum())
                .unwrap_or(0);

            ForAiSectionRef {
                id: format!("section-{}", idx),
                root: s.root.clone(),
                files: s.files_analyzed,
                loc,
                issues,
                slice_cmd: format!("loct slice {} --json", s.root),
            }
        })
        .collect()
}

fn build_agent_bundle(sections: &[ReportSection], analyses: &[FileAnalysis]) -> AgentBundle {
    let handlers = build_handler_groups(sections);

    let mut all_dups: Vec<RankedDup> = sections
        .iter()
        .flat_map(|s| s.ranked_dups.clone())
        .collect();
    all_dups.sort_by(|a, b| b.score.cmp(&a.score).then(a.name.cmp(&b.name)));
    let mut seen_dup: HashSet<(String, String)> = HashSet::new();
    let duplicates = all_dups
        .into_iter()
        .filter(|d| seen_dup.insert((d.name.clone(), d.canonical.clone())))
        .take(20)
        .map(|d| AgentDuplicate {
            name: d.name,
            canonical: d.canonical,
            canonical_line: d.canonical_line,
            score: d.score,
            severity: severity_label(d.severity).to_string(),
            files: d.files.len(),
        })
        .collect();

    let mut seen_dead: HashSet<(String, String)> = HashSet::new();
    let dead_exports = sections
        .iter()
        .flat_map(|s| s.dead_exports.clone())
        .filter(|d| seen_dead.insert((d.file.clone(), d.symbol.clone())))
        .take(50)
        .map(|d| AgentDeadExport {
            symbol: d.symbol,
            file: d.file,
            line: d.line,
            confidence: d.confidence,
            reason: d.reason,
        })
        .collect();

    let dynamic_imports = sections
        .iter()
        .flat_map(|s| s.dynamic.clone())
        .map(|(file, sources)| {
            let mut resolved = Vec::new();
            let mut unresolved = Vec::new();
            for src in sources {
                if is_resolved_dynamic(&src) {
                    resolved.push(src);
                } else {
                    unresolved.push(src);
                }
            }
            AgentDynamicImport {
                file,
                resolved,
                unresolved,
            }
        })
        .collect();

    let mut largest_files: Vec<AgentFile> = analyses
        .iter()
        .map(|a| AgentFile {
            path: a.path.clone(),
            loc: a.loc,
        })
        .collect();
    largest_files.sort_by(|a, b| b.loc.cmp(&a.loc).then(a.path.cmp(&b.path)));
    largest_files.truncate(25);

    let cycles = build_agent_cycles(sections);

    AgentBundle {
        handlers,
        duplicates,
        dead_exports,
        dynamic_imports,
        largest_files,
        cycles,
    }
}

fn build_handler_groups(sections: &[ReportSection]) -> AgentHandlerGroups {
    let mut missing = Vec::new();
    let mut unused = Vec::new();
    let mut unregistered = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for bridge in sections.iter().flat_map(|s| s.command_bridges.iter()) {
        // De-duplicate by command name to avoid repetition across roots
        if !seen.insert(bridge.name.clone()) {
            continue;
        }

        let handler = AgentHandler {
            name: bridge.name.clone(),
            status: bridge.status.clone(),
            frontend: bridge
                .fe_locations
                .iter()
                .map(|(path, line)| AgentLocation {
                    path: path.clone(),
                    line: *line,
                })
                .collect(),
            backend: bridge
                .be_location
                .as_ref()
                .map(|(path, line, symbol)| AgentBackend {
                    path: path.clone(),
                    line: *line,
                    symbol: Some(symbol.clone()),
                }),
        };

        match bridge.status.as_str() {
            "missing_handler" => missing.push(handler),
            "unused_handler" => unused.push(handler),
            "unregistered_handler" => unregistered.push(handler),
            _ => {}
        }
    }

    AgentHandlerGroups {
        missing,
        unused,
        unregistered,
    }
}

fn build_agent_cycles(sections: &[ReportSection]) -> Vec<AgentCycle> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut cycles = Vec::new();

    for section in sections {
        for cycle in &section.circular_imports {
            let key = ("strict".to_string(), cycle.join("->"));
            if seen.insert(key.clone()) {
                cycles.push(AgentCycle {
                    kind: key.0,
                    members: cycle.clone(),
                });
            }
        }
        for cycle in &section.lazy_circular_imports {
            let key = ("lazy".to_string(), cycle.join("->"));
            if seen.insert(key.clone()) {
                cycles.push(AgentCycle {
                    kind: key.0,
                    members: cycle.clone(),
                });
            }
        }
    }

    cycles
}

fn is_resolved_dynamic(src: &str) -> bool {
    let has_extension = src.ends_with(".ts")
        || src.ends_with(".tsx")
        || src.ends_with(".js")
        || src.ends_with(".jsx")
        || src.ends_with(".mjs")
        || src.ends_with(".cjs")
        || src.ends_with(".rs")
        || src.ends_with(".py");
    let has_path = src.contains('/') || src.starts_with("./") || src.starts_with("../");
    has_extension || has_path
}

fn severity_label(severity: DupSeverity) -> &'static str {
    match severity {
        DupSeverity::CrossLangExpected => "cross_lang_expected",
        DupSeverity::ReExportOrGeneric => "reexport_or_generic",
        DupSeverity::SamePackage => "same_package",
        DupSeverity::CrossModule => "cross_module",
        DupSeverity::CrossCrate => "cross_crate",
    }
}

fn extract_quick_wins(sections: &[ReportSection], analyses: &[FileAnalysis]) -> Vec<QuickWin> {
    let mut wins = Vec::new();
    let mut priority = 1u8;

    // Priority 1: Missing handlers (runtime errors!)
    for section in sections {
        for gap in &section.missing_handlers {
            if priority > 10 {
                break;
            }
            let (location, open_url) = gap
                .locations
                .first()
                .map(|(f, l)| {
                    let loc = format!("{}:{}", f, l);
                    let url = super::build_open_url(f, Some(*l), section.open_base.as_deref());
                    (loc, Some(url))
                })
                .unwrap_or_else(|| ("unknown".to_string(), None));

            wins.push(QuickWin {
                priority,
                kind: "missing_handler".to_string(),
                action: "Add missing backend handler".to_string(),
                target: gap.name.clone(),
                location,
                impact: "Fixes runtime error when frontend calls invoke()".to_string(),
                why: "Frontend calls invoke() but no #[tauri::command] handler exists".to_string(),
                fix_hint: format!(
                    "Add #[tauri::command] pub async fn {}(...) in src-tauri/src/commands/",
                    gap.name
                ),
                complexity: "medium".to_string(),
                trace_cmd: Some(format!("loct trace {}", gap.name)),
                open_url,
            });
            priority += 1;
        }
    }

    // Priority 2: Unregistered handlers
    for section in sections {
        for gap in &section.unregistered_handlers {
            if priority > 15 {
                break;
            }
            let (location, open_url) = gap
                .locations
                .first()
                .map(|(f, l)| {
                    let loc = format!("{}:{}", f, l);
                    let url = super::build_open_url(f, Some(*l), section.open_base.as_deref());
                    (loc, Some(url))
                })
                .unwrap_or_else(|| ("unknown".to_string(), None));

            wins.push(QuickWin {
                priority,
                kind: "unregistered_handler".to_string(),
                action: "Register handler in generate_handler![]".to_string(),
                target: gap.name.clone(),
                location,
                impact: "Handler exists but isn't exposed to frontend".to_string(),
                why: "Handler has #[tauri::command] but missing from generate_handler![] macro"
                    .to_string(),
                fix_hint: format!(
                    "Add {} to generate_handler![...] in lib.rs or main.rs",
                    gap.name
                ),
                complexity: "trivial".to_string(),
                trace_cmd: Some(format!("loct trace {}", gap.name)),
                open_url,
            });
            priority += 1;
        }
    }

    // Priority 3: Unused handlers (high confidence only)
    for section in sections {
        for gap in section
            .unused_handlers
            .iter()
            .filter(|h| h.confidence == Some(Confidence::High))
        {
            if priority > 20 {
                break;
            }
            let (location, open_url) = gap
                .locations
                .first()
                .map(|(f, l)| {
                    let loc = format!("{}:{}", f, l);
                    let url = super::build_open_url(f, Some(*l), section.open_base.as_deref());
                    (loc, Some(url))
                })
                .unwrap_or_else(|| ("unknown".to_string(), None));

            wins.push(QuickWin {
                priority,
                kind: "unused_handler".to_string(),
                action: "Remove unused handler".to_string(),
                target: gap.name.clone(),
                location,
                impact: "Dead code - handler defined but never invoked".to_string(),
                why: "No invoke() calls found in frontend for this handler".to_string(),
                fix_hint: format!(
                    "Delete the {} function and remove from generate_handler![]",
                    gap.name
                ),
                complexity: "easy".to_string(),
                trace_cmd: Some(format!("loct trace {}", gap.name)),
                open_url,
            });
            priority += 1;
        }
    }

    // Priority 4: Dead exports (duplicate exports across files)
    for section in sections {
        for dup in section
            .ranked_dups
            .iter()
            .filter(|d| d.score > 10) // Only high-score duplicates
            .take(10)
        // Limit to top 10
        {
            if priority > 30 {
                break;
            }

            // Get primary location from canonical file
            let (location, open_url) = if let Some(canon_line) = dup.canonical_line {
                let loc = format!("{}:{}", dup.canonical, canon_line);
                let url = super::build_open_url(
                    &dup.canonical,
                    Some(canon_line),
                    section.open_base.as_deref(),
                );
                (loc, Some(url))
            } else {
                (dup.canonical.clone(), None)
            };

            let refactor_hint = if !dup.refactors.is_empty() {
                dup.refactors.join(", ")
            } else {
                format!(
                    "Consolidate {} into canonical file {}",
                    dup.name, dup.canonical
                )
            };

            wins.push(QuickWin {
                priority,
                kind: "dead_export".to_string(),
                action: "Consolidate duplicate exports".to_string(),
                target: dup.name.clone(),
                location,
                impact: format!(
                    "Duplicate export across {} files - causes confusion and maintenance burden",
                    dup.files.len()
                ),
                why: format!(
                    "Export '{}' is defined in {} files, creating ambiguity for importers",
                    dup.name,
                    dup.files.len()
                ),
                fix_hint: refactor_hint,
                complexity: "easy".to_string(),
                trace_cmd: Some(format!("loct trace {}", dup.name)),
                open_url,
            });
            priority += 1;
        }
    }

    // Priority 5: Circular imports (import cycles)
    for section in sections {
        let mut seen_cycles = std::collections::HashSet::new();

        for cycle in section.circular_imports.iter().take(5) {
            if priority > 35 {
                break;
            }
            if cycle.is_empty() {
                continue;
            }

            let mut key_nodes = cycle.clone();
            key_nodes.sort();
            if !seen_cycles.insert(key_nodes) {
                continue;
            }

            let mut path = cycle.clone();
            if path.len() > 1 {
                path.push(path[0].clone());
            }

            let why_path = path.join(" → ");

            let target = if path.len() > 8 {
                let head = path[..3].join(" -> ");
                let tail = path[path.len() - 3..].join(" -> ");
                format!("{} -> ... -> {}", head, tail)
            } else {
                path.join(" -> ")
            };

            let location = cycle
                .first()
                .cloned()
                .unwrap_or_else(|| section.root.clone());

            wins.push(QuickWin {
                priority,
                kind: "circular_import".to_string(),
                action: "Break circular import".to_string(),
                target,
                location: location.clone(),
                impact:
                    "Circular imports can cause runtime errors and make code harder to understand"
                        .to_string(),
                why: format!("Dependency cycle detected: {}", why_path),
                fix_hint:
                    "Extract shared code into a third module, or make the dependency unidirectional"
                        .to_string(),
                complexity: "medium".to_string(),
                trace_cmd: None,
                open_url: super::build_open_url(&location, None, section.open_base.as_deref())
                    .into(),
            });
            priority += 1;
        }
    }

    // Priority 6: Opaque passthrough types (types only seen in signatures of used functions)
    let default_open_base = sections.iter().find_map(|s| s.open_base.as_deref());

    for opaque in detect_opaque_passthrough_types(analyses)
        .into_iter()
        .take(10)
    {
        if priority > 45 {
            break;
        }
        let location = if let Some(line) = opaque.line {
            format!("{}:{}", opaque.file, line)
        } else {
            opaque.file.clone()
        };
        let open_url = super::build_open_url(&opaque.file, opaque.line, default_open_base);
        let used_fns: Vec<String> = opaque
            .uses
            .iter()
            .map(|u| {
                let usage = match u.usage {
                    SignatureUseKind::Parameter => "param",
                    SignatureUseKind::Return => "return",
                };
                format!("{} ({usage})", u.function)
            })
            .take(4)
            .collect();
        let fix_hint = match opaque.severity.as_str() {
            "info" => "Document or re-export intentionally, or remove if unused".to_string(),
            "low" => {
                "Consider making the type private if it is only an internal carrier".to_string()
            }
            _ => "Either make the type private, or re-export it in the public API if intentional"
                .to_string(),
        };
        let impact = format!(
            "Severity: {}. Type is only flowing through function signatures; callers cannot import it directly",
            opaque.severity
        );
        wins.push(QuickWin {
            priority,
            kind: "opaque_passthrough".to_string(),
            action: "Harden opaque passthrough type".to_string(),
            target: opaque.symbol.clone(),
            location,
            impact,
            why: format!(
                "'{}' is never imported directly but is used in signatures of {}",
                opaque.symbol,
                used_fns.join(", ")
            ),
            fix_hint,
            complexity: "medium".to_string(),
            trace_cmd: Some(format!("loct trace {}", opaque.symbol)),
            open_url: Some(open_url),
        });
        priority += 1;
    }

    wins
}

#[derive(Clone)]
struct OpaquePassthroughFinding {
    symbol: String,
    file: String,
    line: Option<usize>,
    uses: Vec<SignatureUse>,
    severity: String,
}

fn build_used_exports(analyses: &[FileAnalysis]) -> HashSet<(String, String)> {
    let mut used_exports: HashSet<(String, String)> = HashSet::new();
    for analysis in analyses {
        for imp in &analysis.imports {
            let target_norm = if let Some(target) = &imp.resolved_path {
                normalize_module_id(target).as_key()
            } else {
                normalize_module_id(&imp.source).as_key()
            };
            if imp.symbols.is_empty() {
                continue;
            }
            for sym in &imp.symbols {
                let name = if sym.is_default {
                    "default".to_string()
                } else {
                    sym.name.clone()
                };
                used_exports.insert((target_norm.clone(), name.clone()));
                if sym.name == "*" {
                    used_exports.insert((target_norm.clone(), "*".to_string()));
                }
            }
        }
        for re in &analysis.reexports {
            let target_norm = re
                .resolved
                .as_ref()
                .map(|t| normalize_module_id(t).as_key())
                .unwrap_or_else(|| normalize_module_id(&re.source).as_key());
            match &re.kind {
                crate::types::ReexportKind::Star => {
                    used_exports.insert((target_norm, "*".to_string()));
                }
                crate::types::ReexportKind::Named(names) => {
                    for (original, _exported) in names {
                        used_exports.insert((target_norm.clone(), original.clone()));
                    }
                }
            }
        }
    }
    used_exports
}

fn build_reexport_map(analyses: &[FileAnalysis]) -> HashMap<String, HashSet<String>> {
    let mut map: HashMap<String, HashSet<String>> = HashMap::new();
    for analysis in analyses {
        for re in &analysis.reexports {
            let target_norm = re
                .resolved
                .as_ref()
                .map(|t| normalize_module_id(t).as_key())
                .unwrap_or_else(|| normalize_module_id(&re.source).as_key());
            let entry = map.entry(target_norm).or_default();
            match &re.kind {
                crate::types::ReexportKind::Star => {
                    entry.insert("*".to_string());
                }
                crate::types::ReexportKind::Named(names) => {
                    for (original, _exported) in names {
                        entry.insert(original.clone());
                    }
                }
            }
        }
    }
    map
}

fn is_type_like_export(exp: &crate::types::ExportSymbol, path: &str) -> bool {
    match exp.kind.as_str() {
        "type" | "interface" | "enum" => true,
        "class" => true,
        _ if path.ends_with(".rs") => exp.name.chars().next().is_some_and(|c| c.is_uppercase()),
        _ => false,
    }
}

fn should_exclude_passthrough(exp: &crate::types::ExportSymbol) -> bool {
    let name = exp.name.as_str();
    // Common marker/ZST and doc-hidden-style prefixes
    matches!(
        name,
        "PhantomData" | "PhantomPinned" | "Never" | "Infallible"
    ) || name.starts_with('_')
}

fn detect_opaque_passthrough_types(analyses: &[FileAnalysis]) -> Vec<OpaquePassthroughFinding> {
    let used_exports = build_used_exports(analyses);
    let reexport_map = build_reexport_map(analyses);
    let mut findings = Vec::new();

    for analysis in analyses {
        let module_key = normalize_module_id(&analysis.path).as_key();
        let module_star = used_exports.contains(&(module_key.clone(), "*".to_string()));

        for exp in &analysis.exports {
            if !is_type_like_export(exp, &analysis.path) {
                continue;
            }
            if should_exclude_passthrough(exp) {
                continue;
            }
            if module_star || used_exports.contains(&(module_key.clone(), exp.name.clone())) {
                continue;
            }

            let sigs: Vec<SignatureUse> = analysis
                .signature_uses
                .iter()
                .filter(|s| s.type_name == exp.name)
                .cloned()
                .collect();
            if sigs.is_empty() {
                continue;
            }

            let mut used_sigs: Vec<SignatureUse> = Vec::new();
            for sig in sigs {
                if module_star || used_exports.contains(&(module_key.clone(), sig.function.clone()))
                {
                    used_sigs.push(sig);
                }
            }
            if used_sigs.is_empty() {
                continue;
            }

            let reexported = reexport_map
                .get(&module_key)
                .map(|names| names.contains("*") || names.contains(&exp.name))
                .unwrap_or(false);

            let severity = if reexported { "info" } else { "medium" }.to_string();

            findings.push(OpaquePassthroughFinding {
                symbol: exp.name.clone(),
                file: analysis.path.clone(),
                line: exp.line,
                uses: used_sigs,
                severity,
            });
        }
    }

    findings
}

/// Print quick wins as JSONL (one JSON object per line) for agent consumption
pub fn print_agent_feed_jsonl(report: &ForAiReport) {
    for win in &report.quick_wins {
        match serde_json::to_string(win) {
            Ok(json) => println!("{}", json),
            Err(err) => eprintln!("[loctree][warn] could not serialize quick win: {err}"),
        }
    }
}

fn find_hub_files(analyses: &[FileAnalysis]) -> Vec<HubFile> {
    use std::collections::HashMap;

    // Build reverse index: who imports what
    let mut importers: HashMap<String, Vec<String>> = HashMap::new();
    for analysis in analyses {
        for imp in &analysis.imports {
            if let Some(resolved) = &imp.resolved_path {
                importers
                    .entry(resolved.clone())
                    .or_default()
                    .push(analysis.path.clone());
            }
        }
    }

    // Score files by connectivity
    let mut scored: Vec<_> = analyses
        .iter()
        .map(|a| {
            let imports_count = a.imports.len();
            let exports_count = a.exports.len();
            let importers_count = importers.get(&a.path).map(|v| v.len()).unwrap_or(0);
            let commands_count = a.command_calls.len() + a.command_handlers.len();

            let score =
                imports_count + exports_count * 2 + importers_count * 3 + commands_count * 2;

            (
                a,
                imports_count,
                exports_count,
                importers_count,
                commands_count,
                score,
            )
        })
        .collect();

    scored.sort_by(|a, b| b.5.cmp(&a.5));

    scored
        .into_iter()
        .take(10)
        .filter(|(_, _, _, _, _, score)| *score > 5)
        .map(
            |(a, imports_count, exports_count, importers_count, commands_count, _)| HubFile {
                path: a.path.clone(),
                loc: a.loc,
                imports_count,
                exports_count,
                importers_count,
                commands_count,
                slice_cmd: format!("loct slice {} --json", a.path),
            },
        )
        .collect()
}

/// Print the report as JSON
pub fn print_for_ai_json(report: &ForAiReport) {
    let json = serde_json::to_string_pretty(report).expect("serialize for-ai report");
    println!("{}", json);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::report::{CommandGap, DupLocation, DupSeverity, RankedDup};
    use crate::types::{
        CommandRef, ExportSymbol, ImportEntry, ImportKind, ImportResolutionKind, ImportSymbol,
        SignatureUse, SignatureUseKind,
    };

    fn mock_file(path: &str, loc: usize) -> FileAnalysis {
        FileAnalysis {
            path: path.to_string(),
            loc,
            ..Default::default()
        }
    }

    fn mock_section(root: &str, files: usize) -> ReportSection {
        ReportSection {
            root: root.to_string(),
            files_analyzed: files,
            total_loc: 0,
            reexport_files_count: 0,
            dynamic_imports_count: 0,
            ranked_dups: vec![],
            cascades: vec![],
            circular_imports: vec![],
            lazy_circular_imports: vec![],
            dynamic: vec![],
            analyze_limit: 50,
            missing_handlers: vec![],
            unregistered_handlers: vec![],
            unused_handlers: vec![],
            command_counts: (0, 0),
            command_bridges: vec![],
            open_base: None,
            tree: None,
            insights: vec![],
            graph: None,
            graph_warning: None,
            git_branch: None,
            git_commit: None,
            crowds: vec![],
            dead_exports: vec![],
            twins_data: None,
            coverage_gaps: vec![],
        }
    }

    #[test]
    fn test_compute_summary_empty() {
        let sections: Vec<ReportSection> = vec![];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        assert_eq!(summary.files_analyzed, 0);
        assert_eq!(summary.total_loc, 0);
        assert_eq!(summary.health_score, 100);
        assert!(summary.priority.contains("HEALTHY"));
    }

    #[test]
    fn test_compute_summary_empty_sections_with_analyses() {
        // Bug fix test: when sections are empty but analyses exist (e.g., --full-scan),
        // files_analyzed should be populated from analyses.len()
        let sections: Vec<ReportSection> = vec![];
        let analyses = vec![
            mock_file("src/a.ts", 100),
            mock_file("src/b.ts", 200),
            mock_file("src/c.rs", 50),
        ];

        let summary = compute_summary(&sections, &analyses);

        assert_eq!(summary.files_analyzed, 3);
        assert_eq!(summary.total_loc, 350);
        assert_eq!(summary.health_score, 100);
        assert!(summary.priority.contains("HEALTHY"));
    }

    #[test]
    fn test_compute_summary_with_missing_handlers() {
        let mut section = mock_section("src", 10);
        section.missing_handlers = vec![
            CommandGap {
                name: "cmd1".to_string(),
                implementation_name: None,
                locations: vec![("src/a.ts".to_string(), 1)],
                confidence: None,
                string_literal_matches: vec![],
            },
            CommandGap {
                name: "cmd2".to_string(),
                implementation_name: None,
                locations: vec![("src/b.ts".to_string(), 2)],
                confidence: None,
                string_literal_matches: vec![],
            },
        ];

        let sections = vec![section];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        assert_eq!(summary.missing_handlers, 2);
        assert!(summary.priority.contains("CRITICAL"));
        // Missing handlers penalty: 2 * 20 = 40
        assert!(summary.health_score < 100);
    }

    #[test]
    fn test_compute_summary_with_unregistered_handlers() {
        let mut section = mock_section("src", 10);
        section.unregistered_handlers = vec![CommandGap {
            name: "unreg_cmd".to_string(),
            implementation_name: Some("unregisteredHandler".to_string()),
            locations: vec![("src-tauri/src/main.rs".to_string(), 50)],
            confidence: None,
            string_literal_matches: vec![],
        }];

        let sections = vec![section];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        assert_eq!(summary.unregistered_handlers, 1);
        assert!(summary.priority.contains("WARNING"));
    }

    #[test]
    fn test_compute_summary_with_unused_high_confidence() {
        let mut section = mock_section("src", 10);
        section.unused_handlers = vec![CommandGap {
            name: "unused_cmd".to_string(),
            implementation_name: Some("unusedHandler".to_string()),
            locations: vec![("src-tauri/src/main.rs".to_string(), 100)],
            confidence: Some(Confidence::High),
            string_literal_matches: vec![],
        }];

        let sections = vec![section];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        assert_eq!(summary.unused_high_confidence, 1);
        assert!(summary.priority.contains("CLEANUP"));
    }

    #[test]
    fn test_build_section_refs() {
        let sections = vec![mock_section("src", 10), mock_section("lib", 5)];

        let refs = build_section_refs(&sections);

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].root, "src");
        assert_eq!(refs[0].files, 10);
        assert!(refs[0].slice_cmd.contains("loct slice"));
        assert_eq!(refs[1].root, "lib");
    }

    #[test]
    fn test_extract_quick_wins_missing_handlers() {
        let mut section = mock_section("src", 10);
        section.missing_handlers = vec![CommandGap {
            name: "missing_cmd".to_string(),
            implementation_name: None,
            locations: vec![("src/app.ts".to_string(), 42)],
            confidence: None,
            string_literal_matches: vec![],
        }];

        let sections = vec![section];
        let wins = extract_quick_wins(&sections, &[]);

        assert!(!wins.is_empty());
        assert_eq!(wins[0].priority, 1);
        assert!(wins[0].action.contains("missing backend handler"));
        assert_eq!(wins[0].target, "missing_cmd");
        assert!(wins[0].location.contains("src/app.ts:42"));
    }

    #[test]
    fn test_extract_quick_wins_priority_order() {
        let mut section = mock_section("src", 10);
        section.missing_handlers = vec![CommandGap {
            name: "missing".to_string(),
            implementation_name: None,
            locations: vec![("a.ts".to_string(), 1)],
            confidence: None,
            string_literal_matches: vec![],
        }];
        section.unregistered_handlers = vec![CommandGap {
            name: "unreg".to_string(),
            implementation_name: Some("unregHandler".to_string()),
            locations: vec![("b.rs".to_string(), 2)],
            confidence: None,
            string_literal_matches: vec![],
        }];
        section.unused_handlers = vec![CommandGap {
            name: "unused".to_string(),
            implementation_name: Some("unusedHandler".to_string()),
            locations: vec![("c.rs".to_string(), 3)],
            confidence: Some(Confidence::High),
            string_literal_matches: vec![],
        }];

        let sections = vec![section];
        let wins = extract_quick_wins(&sections, &[]);

        // Should have all 3 with priority order: missing < unregistered < unused
        assert!(wins.len() >= 3);
        let missing_win = wins.iter().find(|w| w.target == "missing").unwrap();
        let unreg_win = wins.iter().find(|w| w.target == "unreg").unwrap();
        let unused_win = wins.iter().find(|w| w.target == "unused").unwrap();

        assert!(missing_win.priority < unreg_win.priority);
        assert!(unreg_win.priority < unused_win.priority);
    }

    #[test]
    fn test_find_hub_files_empty() {
        let analyses: Vec<FileAnalysis> = vec![];
        let hubs = find_hub_files(&analyses);
        assert!(hubs.is_empty());
    }

    #[test]
    fn test_find_hub_files_scores_by_connectivity() {
        let mut high_connectivity = mock_file("hub.ts", 200);
        high_connectivity.imports = vec![
            ImportEntry::new("./a".to_string(), ImportKind::Static),
            ImportEntry::new("./b".to_string(), ImportKind::Static),
            ImportEntry::new("./c".to_string(), ImportKind::Static),
        ];
        high_connectivity.command_handlers = vec![
            CommandRef {
                name: "cmd1".to_string(),
                exposed_name: None,
                line: 10,
                generic_type: None,
                payload: None,
            },
            CommandRef {
                name: "cmd2".to_string(),
                exposed_name: None,
                line: 20,
                generic_type: None,
                payload: None,
            },
        ];

        let low_connectivity = mock_file("leaf.ts", 50);

        let analyses = vec![high_connectivity, low_connectivity];
        let hubs = find_hub_files(&analyses);

        // High connectivity file should appear first (if any)
        if !hubs.is_empty() {
            assert_eq!(hubs[0].path, "hub.ts");
        }
    }

    #[test]
    fn test_generate_for_ai_report() {
        let sections = vec![mock_section("src", 5)];
        let analyses = vec![mock_file("src/a.ts", 100), mock_file("src/b.ts", 50)];

        let report = generate_for_ai_report("/project", &sections, &analyses);

        assert_eq!(report.project, "/project");
        assert!(!report.generated_at.is_empty());
        assert_eq!(report.summary.files_analyzed, 5);
        assert_eq!(report.summary.total_loc, 150);
        assert_eq!(report.sections.len(), 1);
    }

    #[test]
    fn test_health_score_bounds() {
        // Health score should be 0-100
        let mut section = mock_section("src", 10);
        // Add lots of issues to test lower bound
        section.missing_handlers = (0..10)
            .map(|i| CommandGap {
                name: format!("cmd{}", i),
                implementation_name: None,
                locations: vec![("a.ts".to_string(), i)],
                confidence: None,
                string_literal_matches: vec![],
            })
            .collect();

        let sections = vec![section];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        // Should not go below 0
        assert!(summary.health_score <= 100);
        // With 10 missing handlers (20 points each = 200), should be 0
        assert_eq!(summary.health_score, 0);
    }

    #[test]
    fn test_extract_quick_wins_dead_exports() {
        let mut section = mock_section("src", 10);
        section.ranked_dups = vec![RankedDup {
            name: "UserType".to_string(),
            files: vec![
                "src/types/user.ts".to_string(),
                "src/models/user.ts".to_string(),
                "src/api/user.ts".to_string(),
            ],
            locations: vec![
                DupLocation {
                    file: "src/types/user.ts".to_string(),
                    line: Some(10),
                },
                DupLocation {
                    file: "src/models/user.ts".to_string(),
                    line: Some(20),
                },
            ],
            score: 50,
            prod_count: 3,
            dev_count: 0,
            canonical: "src/types/user.ts".to_string(),
            canonical_line: Some(10),
            refactors: vec!["Move all imports to src/types/user.ts".to_string()],
            severity: DupSeverity::CrossCrate,
            is_cross_lang: false,
            packages: vec!["types".to_string(), "models".to_string(), "api".to_string()],
            reason: "Symbol in 3 different packages".to_string(),
        }];

        let sections = vec![section];
        let wins = extract_quick_wins(&sections, &[]);

        // Should include dead export quick win
        let dead_export_wins: Vec<_> = wins.iter().filter(|w| w.kind == "dead_export").collect();
        assert!(!dead_export_wins.is_empty());

        let win = dead_export_wins[0];
        assert_eq!(win.target, "UserType");
        assert_eq!(win.kind, "dead_export");
        assert_eq!(win.complexity, "easy");
        assert!(win.location.contains("src/types/user.ts:10"));
        assert!(win.why.contains("defined in 3 files"));
        assert!(
            win.fix_hint
                .contains("Move all imports to src/types/user.ts")
        );
    }

    #[test]
    fn test_extract_quick_wins_circular_imports() {
        let mut section = mock_section("src", 10);
        section.circular_imports = vec![
            vec!["src/a.ts".to_string(), "src/b.ts".to_string()],
            vec!["src/b.ts".to_string(), "src/a.ts".to_string()],
            vec![
                "src/c.ts".to_string(),
                "src/d.ts".to_string(),
                "src/e.ts".to_string(),
            ],
        ];

        let sections = vec![section];
        let wins = extract_quick_wins(&sections, &[]);

        // Should include circular import quick wins
        let cycle_wins: Vec<_> = wins
            .iter()
            .filter(|w| w.kind == "circular_import")
            .collect();
        assert!(!cycle_wins.is_empty());

        // Should deduplicate bidirectional cycles (a↔b)
        assert!(
            cycle_wins
                .iter()
                .any(|w| w.target.contains("src/a.ts") && w.target.contains("src/b.ts"))
        );

        let win = &cycle_wins[0];
        assert_eq!(win.kind, "circular_import");
        assert_eq!(win.complexity, "medium");
        assert!(win.why.contains("Dependency cycle"));
        assert!(
            win.fix_hint
                .contains("Extract shared code into a third module")
        );
    }

    #[test]
    fn test_extract_quick_wins_all_priorities() {
        let mut section = mock_section("src", 10);

        // Priority 1: Missing handler
        section.missing_handlers = vec![CommandGap {
            name: "missing_cmd".to_string(),
            implementation_name: None,
            locations: vec![("src/app.ts".to_string(), 1)],
            confidence: None,
            string_literal_matches: vec![],
        }];

        // Priority 2: Unregistered handler
        section.unregistered_handlers = vec![CommandGap {
            name: "unreg_cmd".to_string(),
            implementation_name: Some("unregHandler".to_string()),
            locations: vec![("src-tauri/src/main.rs".to_string(), 2)],
            confidence: None,
            string_literal_matches: vec![],
        }];

        // Priority 3: Unused handler
        section.unused_handlers = vec![CommandGap {
            name: "unused_cmd".to_string(),
            implementation_name: Some("unusedHandler".to_string()),
            locations: vec![("src-tauri/src/commands.rs".to_string(), 3)],
            confidence: Some(Confidence::High),
            string_literal_matches: vec![],
        }];

        // Priority 4: Dead export
        section.ranked_dups = vec![RankedDup {
            name: "DupType".to_string(),
            files: vec!["a.ts".to_string(), "b.ts".to_string()],
            locations: vec![],
            score: 20,
            prod_count: 2,
            dev_count: 0,
            canonical: "a.ts".to_string(),
            canonical_line: Some(10),
            refactors: vec![],
            severity: DupSeverity::SamePackage,
            is_cross_lang: false,
            packages: vec![],
            reason: String::new(),
        }];

        // Priority 5: Circular import
        section.circular_imports = vec![vec!["x.ts".to_string(), "y.ts".to_string()]];

        let sections = vec![section];
        let wins = extract_quick_wins(&sections, &[]);

        // Should have all 5 priorities represented
        assert!(wins.len() >= 5);

        // Verify we have each kind
        assert!(wins.iter().any(|w| w.kind == "missing_handler"));
        assert!(wins.iter().any(|w| w.kind == "unregistered_handler"));
        assert!(wins.iter().any(|w| w.kind == "unused_handler"));
        assert!(wins.iter().any(|w| w.kind == "dead_export"));
        assert!(wins.iter().any(|w| w.kind == "circular_import"));

        // Verify priority ordering
        let priorities: Vec<u8> = wins.iter().map(|w| w.priority).collect();
        let mut sorted_priorities = priorities.clone();
        sorted_priorities.sort();
        assert_eq!(priorities, sorted_priorities, "Priorities should be sorted");
    }

    #[test]
    fn test_detects_opaque_passthrough_quick_win() {
        // Producer with a type and a function that uses it in signature
        let mut producer = FileAnalysis {
            path: "src/tray.rs".to_string(),
            exports: vec![
                ExportSymbol::new("LoadedIcon".to_string(), "decl", "named", Some(18)),
                ExportSymbol::new("spawn_tray".to_string(), "decl", "named", Some(24)),
            ],
            ..Default::default()
        };
        producer.signature_uses.push(SignatureUse {
            function: "spawn_tray".to_string(),
            usage: SignatureUseKind::Parameter,
            type_name: "LoadedIcon".to_string(),
            line: Some(24),
        });

        // Consumer imports the function (not the type)
        let mut consumer = FileAnalysis {
            path: "src/main.rs".to_string(),
            ..Default::default()
        };
        consumer.imports.push(ImportEntry {
            source: "src/tray.rs".to_string(),
            source_raw: "src/tray.rs".to_string(),
            kind: ImportKind::Static,
            resolved_path: Some("src/tray.rs".to_string()),
            is_bare: false,
            symbols: vec![ImportSymbol {
                name: "spawn_tray".to_string(),
                alias: None,
                is_default: false,
            }],
            resolution: ImportResolutionKind::Local,
            is_type_checking: false,
            is_lazy: false,
            is_crate_relative: false,
            is_super_relative: false,
            is_self_relative: false,
            raw_path: String::new(),
        });

        let findings = detect_opaque_passthrough_types(&[producer.clone(), consumer.clone()]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].symbol, "LoadedIcon");

        // Quick win emitted when analyses are provided
        let wins = extract_quick_wins(&[mock_section("root", 2)], &[producer, consumer]);
        assert!(
            wins.iter().any(|w| w.kind == "opaque_passthrough"),
            "Opaque passthrough quick win should be emitted"
        );
    }

    #[test]
    fn test_compute_summary_with_twins_same_language() {
        use crate::analyzer::report::TwinsData;
        use crate::analyzer::twins::{ExactTwin, SymbolEntry, TwinLocation};

        let mut section = mock_section("src", 10);
        section.twins_data = Some(TwinsData {
            dead_parrots: vec![SymbolEntry {
                name: "unusedUtil".to_string(),
                kind: "function".to_string(),
                file_path: "src/utils.ts".to_string(),
                line: 10,
                import_count: 0,
            }],
            exact_twins: vec![ExactTwin {
                name: "UserType".to_string(),
                locations: vec![
                    TwinLocation {
                        file_path: "src/types/user.ts".to_string(),
                        line: 5,
                        kind: "type".to_string(),
                        import_count: 10,
                        is_canonical: true,
                        signature_fingerprint: None,
                    },
                    TwinLocation {
                        file_path: "src/models/user.ts".to_string(),
                        line: 8,
                        kind: "type".to_string(),
                        import_count: 2,
                        is_canonical: false,
                        signature_fingerprint: None,
                    },
                ],
                signature_similarity: None,
            }],
            barrel_chaos: Default::default(),
        });

        let sections = vec![section];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        // Check twins are counted
        assert_eq!(summary.twins_dead_parrots, 1);
        assert_eq!(summary.twins_same_language, 1);
        assert_eq!(summary.twins_cross_language, 0);

        // Health score should be reduced
        // Penalty: 1 dead_parrot * 1 + 1 same_lang * 2 = 3
        assert_eq!(summary.health_score, 97);

        // Priority should mention twins
        assert!(summary.priority.contains("same-language twins"));
    }

    #[test]
    fn test_compute_summary_with_twins_cross_language() {
        use crate::analyzer::report::TwinsData;
        use crate::analyzer::twins::{ExactTwin, TwinLocation};

        let mut section = mock_section("src", 10);
        section.twins_data = Some(TwinsData {
            dead_parrots: vec![],
            exact_twins: vec![ExactTwin {
                name: "Message".to_string(),
                locations: vec![
                    TwinLocation {
                        file_path: "src/types/message.ts".to_string(),
                        line: 5,
                        kind: "interface".to_string(),
                        import_count: 10,
                        is_canonical: true,
                        signature_fingerprint: None,
                    },
                    TwinLocation {
                        file_path: "src-tauri/src/types.rs".to_string(),
                        line: 20,
                        kind: "struct".to_string(),
                        import_count: 5,
                        is_canonical: false,
                        signature_fingerprint: None,
                    },
                ],
                signature_similarity: None,
            }],
            barrel_chaos: Default::default(),
        });

        let sections = vec![section];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        // Cross-language twins should NOT add to penalty
        assert_eq!(summary.twins_same_language, 0);
        assert_eq!(summary.twins_cross_language, 1);

        // Health score should be 100 (cross-lang twins don't penalize)
        assert_eq!(summary.health_score, 100);
        assert!(summary.priority.contains("HEALTHY"));
    }

    #[test]
    fn test_compute_summary_twins_dead_parrots_penalty() {
        use crate::analyzer::report::TwinsData;
        use crate::analyzer::twins::SymbolEntry;

        let mut section = mock_section("src", 10);
        section.twins_data = Some(TwinsData {
            dead_parrots: (0..10)
                .map(|i| SymbolEntry {
                    name: format!("unused{}", i),
                    kind: "function".to_string(),
                    file_path: format!("src/util{}.ts", i),
                    line: i,
                    import_count: 0,
                })
                .collect(),
            exact_twins: vec![],
            barrel_chaos: Default::default(),
        });

        let sections = vec![section];
        let analyses: Vec<FileAnalysis> = vec![];

        let summary = compute_summary(&sections, &analyses);

        // 10 dead parrots * 1 point each = 10 penalty
        assert_eq!(summary.twins_dead_parrots, 10);
        assert_eq!(summary.health_score, 90);
        assert!(summary.priority.contains("dead parrots"));
    }
}
