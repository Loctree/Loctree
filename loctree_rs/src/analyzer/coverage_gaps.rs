//! Coverage gap detection - finds mismatches between production usage and test coverage.
//!
//! This module cross-references three data sources:
//! 1. **Production usage**: What FE actually calls (invoke(), emit())
//! 2. **Test imports**: What test files import
//! 3. **Handler definitions**: What exists in backend
//!
//! The result is actionable gaps like:
//! - Handlers used in production but not tested (HIGH RISK)
//! - Events emitted but no test coverage (MEDIUM RISK)
//! - Tested code that's not used in production (potential dead code)
//!
//! Developed with 💀 by The Loctree Team (c)2025

use super::classify::is_test_path;
use crate::snapshot::{CommandBridge, EventBridge, Snapshot};
use crate::types::FileAnalysis;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A gap in test coverage
#[derive(Debug, Clone, Serialize)]
pub struct CoverageGap {
    pub kind: GapKind,
    pub target: String,
    pub location: String,
    pub severity: Severity,
    pub recommendation: String,
    /// Additional context about the gap
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// File paths involved
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    /// Handler used in production but not tested
    HandlerWithoutTest,
    /// Event emitted in production but not tested
    EventWithoutTest,
    /// Export used in production but not tested
    ExportWithoutTest,
    /// Tested but not used in production (suspicious)
    TestedButUnused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical, // Handler without test (can break runtime)
    High,     // Event without test (data flow issues)
    Medium,   // Export without test (integration gaps)
    Low,      // Tested but unused (cleanup candidate)
}

/// Find all coverage gaps in a snapshot
pub fn find_coverage_gaps(snapshot: &Snapshot) -> Vec<CoverageGap> {
    let mut gaps = Vec::new();

    // Build test file detection
    let test_files = detect_test_files(&snapshot.files);

    // Build test import index: what symbols do test files import?
    let test_imports = build_test_import_index(&snapshot.files, &test_files);

    // Gap 1: Handlers without tests
    gaps.extend(find_handler_gaps(&snapshot.command_bridges, &test_imports));

    // Gap 2: Events without tests
    gaps.extend(find_event_gaps(&snapshot.event_bridges, &test_imports));

    // Gap 3: Exports without tests (from files with production usage)
    gaps.extend(find_export_gaps(
        &snapshot.files,
        &test_imports,
        &test_files,
    ));

    // Gap 4: Tested but unused (inverse analysis)
    gaps.extend(find_tested_but_unused(
        &snapshot.command_bridges,
        &snapshot.event_bridges,
        &test_imports,
    ));

    // Sort by severity (critical first)
    gaps.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.target.cmp(&b.target)));

    gaps
}

/// Detect which files are test files
fn detect_test_files(files: &[FileAnalysis]) -> HashSet<String> {
    files
        .iter()
        .filter(|f| is_test_path(&f.path))
        .map(|f| f.path.clone())
        .collect()
}

/// Build index of what test files import
/// Returns: Map<symbol_name, Vec<test_file_that_imports_it>>
fn build_test_import_index(
    files: &[FileAnalysis],
    test_files: &HashSet<String>,
) -> HashMap<String, Vec<String>> {
    let mut index: HashMap<String, Vec<String>> = HashMap::new();

    for file in files {
        if !test_files.contains(&file.path) {
            continue;
        }

        // Collect all imported symbols from this test file
        for import in &file.imports {
            for symbol in &import.symbols {
                let name = if symbol.is_default {
                    "default".to_string()
                } else {
                    symbol.name.clone()
                };

                index.entry(name).or_default().push(file.path.clone());
            }
        }

        // Also track command handlers if test files define mocks/fixtures
        for handler in &file.command_handlers {
            index
                .entry(handler.name.clone())
                .or_default()
                .push(file.path.clone());
        }
    }

    index
}

/// Find handlers used in production but not tested
fn find_handler_gaps(
    command_bridges: &[CommandBridge],
    test_imports: &HashMap<String, Vec<String>>,
) -> Vec<CoverageGap> {
    let mut gaps = Vec::new();

    for bridge in command_bridges {
        // Only care about handlers that:
        // 1. Have a backend implementation
        // 2. Are called from frontend (production usage)
        if !bridge.has_handler || !bridge.is_called {
            continue;
        }

        // Check if this handler is imported by any test file
        let is_tested = test_imports.contains_key(&bridge.name);

        if !is_tested {
            let location = bridge
                .backend_handler
                .as_ref()
                .map(|(path, line)| format!("{}:{}", path, line))
                .unwrap_or_else(|| "unknown".to_string());

            let frontend_files: Vec<String> = bridge
                .frontend_calls
                .iter()
                .map(|(path, _)| path.clone())
                .collect();

            gaps.push(CoverageGap {
                kind: GapKind::HandlerWithoutTest,
                target: bridge.name.clone(),
                location,
                severity: Severity::Critical,
                recommendation: format!(
                    "Add test coverage for handler '{}' - it's called from {} production location(s) but has no tests",
                    bridge.name,
                    bridge.frontend_calls.len()
                ),
                context: Some(format!(
                    "Called from: {}",
                    frontend_files.join(", ")
                )),
                files: frontend_files,
            });
        }
    }

    gaps
}

/// Find events emitted in production but not tested
fn find_event_gaps(
    event_bridges: &[EventBridge],
    test_imports: &HashMap<String, Vec<String>>,
) -> Vec<CoverageGap> {
    let mut gaps = Vec::new();

    for bridge in event_bridges {
        // Only care about events that are actually emitted
        if bridge.emits.is_empty() {
            continue;
        }

        // Check if event name appears in test imports (rough heuristic)
        let is_tested = test_imports.contains_key(&bridge.name);

        if !is_tested {
            let location = bridge
                .emits
                .first()
                .map(|(path, line, _)| format!("{}:{}", path, line))
                .unwrap_or_else(|| "unknown".to_string());

            let emit_files: Vec<String> = bridge
                .emits
                .iter()
                .map(|(path, _, _)| path.clone())
                .collect();

            gaps.push(CoverageGap {
                kind: GapKind::EventWithoutTest,
                target: bridge.name.clone(),
                location,
                severity: Severity::High,
                recommendation: format!(
                    "Add test coverage for event '{}' - emitted from {} location(s) but not tested",
                    bridge.name,
                    bridge.emits.len()
                ),
                context: Some(format!(
                    "Emitted from: {}",
                    emit_files
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                files: emit_files,
            });
        }
    }

    gaps
}

/// Find exports used in production but not tested
fn find_export_gaps(
    files: &[FileAnalysis],
    test_imports: &HashMap<String, Vec<String>>,
    test_files: &HashSet<String>,
) -> Vec<CoverageGap> {
    let mut gaps = Vec::new();

    // Build usage map: which exports are actually imported in production code?
    let mut production_usage: HashMap<String, Vec<String>> = HashMap::new();

    for file in files {
        if test_files.contains(&file.path) {
            continue; // Skip test files
        }

        for import in &file.imports {
            for symbol in &import.symbols {
                let name = if symbol.is_default {
                    "default".to_string()
                } else {
                    symbol.name.clone()
                };

                production_usage
                    .entry(name)
                    .or_default()
                    .push(file.path.clone());
            }
        }
    }

    // Find exports that are used in production but not tested
    for (symbol, usage_locations) in production_usage {
        if symbol == "*" {
            continue; // Skip wildcard imports
        }

        let is_tested = test_imports.contains_key(&symbol);

        if !is_tested && usage_locations.len() >= 2 {
            // Only flag if used in multiple places (more important)
            gaps.push(CoverageGap {
                kind: GapKind::ExportWithoutTest,
                target: symbol.clone(),
                location: usage_locations.first().cloned().unwrap_or_default(),
                severity: Severity::Medium,
                recommendation: format!(
                    "Add test for export '{}' - used in {} production files but not tested",
                    symbol,
                    usage_locations.len()
                ),
                context: Some(format!(
                    "Used in: {}",
                    usage_locations
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                files: usage_locations,
            });
        }
    }

    gaps
}

/// Find handlers/events that are tested but not used in production
fn find_tested_but_unused(
    command_bridges: &[CommandBridge],
    event_bridges: &[EventBridge],
    test_imports: &HashMap<String, Vec<String>>,
) -> Vec<CoverageGap> {
    let mut gaps = Vec::new();

    // Build set of production-used handlers and events
    let mut production_handlers: HashSet<String> = HashSet::new();
    let mut production_events: HashSet<String> = HashSet::new();

    for bridge in command_bridges {
        if bridge.is_called {
            production_handlers.insert(bridge.name.clone());
        }
    }

    for bridge in event_bridges {
        if !bridge.emits.is_empty() {
            production_events.insert(bridge.name.clone());
        }
    }

    // Check test imports for handlers/events not in production
    for (symbol, test_files) in test_imports {
        // Check if it looks like a handler (common naming patterns)
        let looks_like_handler = symbol.contains("Handler")
            || symbol.contains("Command")
            || symbol.starts_with("handle_")
            || symbol.starts_with("cmd_");

        let looks_like_event =
            symbol.contains("Event") || symbol.contains("event") || symbol.ends_with("_event");

        if looks_like_handler && !production_handlers.contains(symbol) {
            gaps.push(CoverageGap {
                kind: GapKind::TestedButUnused,
                target: symbol.clone(),
                location: test_files.first().cloned().unwrap_or_default(),
                severity: Severity::Low,
                recommendation: format!(
                    "Handler '{}' has tests but is not called in production - consider removing if truly unused",
                    symbol
                ),
                context: Some(format!(
                    "Tested in: {}",
                    test_files.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )),
                files: test_files.clone(),
            });
        } else if looks_like_event && !production_events.contains(symbol) {
            gaps.push(CoverageGap {
                kind: GapKind::TestedButUnused,
                target: symbol.clone(),
                location: test_files.first().cloned().unwrap_or_default(),
                severity: Severity::Low,
                recommendation: format!(
                    "Event '{}' has tests but is not emitted in production - consider removing if truly unused",
                    symbol
                ),
                context: Some(format!(
                    "Tested in: {}",
                    test_files.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )),
                files: test_files.clone(),
            });
        }
    }

    gaps
}

/// Generate actionable quick wins from gaps
pub fn gaps_to_quick_wins(gaps: &[CoverageGap]) -> Vec<crate::analyzer::for_ai::QuickWin> {
    gaps.iter()
        .enumerate()
        .map(|(idx, gap)| {
            let (action, why, fix_hint, complexity) = match gap.kind {
                GapKind::HandlerWithoutTest => (
                    "Add test coverage for production handler",
                    format!("Handler '{}' is called in production but has no tests - runtime failures won't be caught", gap.target),
                    format!("Create test file that imports and tests '{}' handler", gap.target),
                    "medium",
                ),
                GapKind::EventWithoutTest => (
                    "Add test coverage for event emission",
                    format!("Event '{}' is emitted in production but has no tests - event handlers may break silently", gap.target),
                    format!("Add test that verifies '{}' event is emitted with correct payload", gap.target),
                    "medium",
                ),
                GapKind::ExportWithoutTest => (
                    "Add test for production export",
                    format!("Export '{}' is used across multiple production files but has no tests", gap.target),
                    format!("Create unit tests for '{}' to ensure behavior is documented", gap.target),
                    "easy",
                ),
                GapKind::TestedButUnused => (
                    "Remove unused test or restore production usage",
                    format!("'{}' has tests but is not used in production - likely dead code", gap.target),
                    format!("Either remove tests for '{}' or restore production usage if intentional", gap.target),
                    "easy",
                ),
            };

            let priority = match gap.severity {
                Severity::Critical => 5 + idx as u8,
                Severity::High => 15 + idx as u8,
                Severity::Medium => 25 + idx as u8,
                Severity::Low => 35 + idx as u8,
            };

            crate::analyzer::for_ai::QuickWin {
                priority: priority.min(100),
                kind: format!("{:?}", gap.kind).to_lowercase(),
                action: action.to_string(),
                target: gap.target.clone(),
                location: gap.location.clone(),
                impact: gap.recommendation.clone(),
                why,
                fix_hint: fix_hint.to_string(),
                complexity: complexity.to_string(),
                trace_cmd: Some(format!("loct trace {}", gap.target)),
                open_url: None,
            }
        })
        .collect()
}
