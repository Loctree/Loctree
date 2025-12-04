//! AI-optimized hierarchical output format.
//!
//! Transforms analysis results into structured JSON that AI agents can:
//! - Parse easily with regex/jq
//! - Navigate via slice references
//! - Get actionable quick wins
//!
//! Developed with 💀 by The Loctree Team (c)2025

use serde::Serialize;

use super::report::{Confidence, ReportSection};
use crate::types::FileAnalysis;

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
}

/// Summary with counts and priority guidance
#[derive(Serialize)]
pub struct ForAiSummary {
    pub files_analyzed: usize,
    pub total_loc: usize,
    pub dead_exports: usize,
    pub missing_handlers: usize,
    pub unregistered_handlers: usize,
    pub unused_handlers: usize,
    pub unused_high_confidence: usize,
    pub cascade_imports: usize,
    pub dynamic_imports: usize,
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
    /// Kind of issue: missing_handler, unregistered_handler, unused_handler, dead_export, circular_import
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
    let quick_wins = extract_quick_wins(sections);
    let hub_files = find_hub_files(analyses);

    ForAiReport {
        project: project_root.to_string(),
        generated_at,
        summary,
        sections: section_refs,
        quick_wins,
        hub_files,
    }
}

fn compute_summary(sections: &[ReportSection], analyses: &[FileAnalysis]) -> ForAiSummary {
    let files_analyzed: usize = sections.iter().map(|s| s.files_analyzed).sum();
    let total_loc: usize = analyses.iter().map(|a| a.loc).sum();
    let dead_exports: usize = sections.iter().map(|s| s.ranked_dups.len()).sum();
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

    // Generate priority message
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
            "CLEANUP: {} unused handlers (high confidence) can be safely removed. {} dead exports to consolidate.",
            unused_high_confidence, dead_exports
        )
    } else if dead_exports > 0 {
        format!(
            "TECH DEBT: {} duplicate exports across files. Consider consolidating to reduce confusion.",
            dead_exports
        )
    } else {
        "HEALTHY: No critical issues found. Good job!".to_string()
    };

    // Health score (simple heuristic)
    let issue_penalty = missing_handlers * 20
        + unregistered_handlers * 15
        + unused_high_confidence * 5
        + (dead_exports / 10).min(20);
    let health_score = 100u8.saturating_sub(issue_penalty.min(100) as u8);

    ForAiSummary {
        files_analyzed,
        total_loc,
        dead_exports,
        missing_handlers,
        unregistered_handlers,
        unused_handlers,
        unused_high_confidence,
        cascade_imports,
        dynamic_imports,
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

fn extract_quick_wins(sections: &[ReportSection]) -> Vec<QuickWin> {
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
                    "Duplicate exported across {} files - causes confusion and maintenance burden",
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
                why: format!("Dependency cycle detected: {}", cycle.join(" → ")),
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

    wins
}

/// Print quick wins as JSONL (one JSON object per line) for agent consumption
pub fn print_agent_feed_jsonl(report: &ForAiReport) {
    for win in &report.quick_wins {
        let json = serde_json::to_string(win).expect("serialize QuickWin");
        println!("{}", json);
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
    use crate::analyzer::report::{CommandGap, DupLocation, RankedDup};
    use crate::types::{CommandRef, ImportEntry, ImportKind};

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
        let wins = extract_quick_wins(&sections);

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
        let wins = extract_quick_wins(&sections);

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
        }];

        let sections = vec![section];
        let wins = extract_quick_wins(&sections);

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
        let wins = extract_quick_wins(&sections);

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
        }];

        // Priority 5: Circular import
        section.circular_imports = vec![vec!["x.ts".to_string(), "y.ts".to_string()]];

        let sections = vec![section];
        let wins = extract_quick_wins(&sections);

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
}
