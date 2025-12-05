use serde::Serialize;

use super::crowd::types::Crowd;
use super::dead_parrots::DeadExport;

/// Confidence level for unused handler detection.
/// HIGH = no string literal matches found (likely truly unused)
/// LOW = string literal matches found (may be used dynamically)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Confidence {
    High,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "HIGH"),
            Confidence::Low => write!(f, "LOW"),
        }
    }
}

/// A string literal match in frontend code that might indicate dynamic usage.
#[derive(Clone, Debug, Serialize)]
pub struct StringLiteralMatch {
    pub file: String,
    pub line: usize,
    pub context: String, // "allowlist", "const", "object_key", "array_item"
}

#[derive(Clone, Serialize)]
pub struct CommandGap {
    pub name: String,
    pub implementation_name: Option<String>,
    pub locations: Vec<(String, usize)>,
    /// Confidence level (None for missing handlers, Some for unused handlers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    /// String literal matches that may indicate dynamic usage
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub string_literal_matches: Vec<StringLiteralMatch>,
}

#[derive(Clone, Serialize)]
pub struct AiInsight {
    pub title: String,
    pub severity: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub loc: usize,
    pub x: f32,
    pub y: f32,
    pub component: usize,
    pub degree: usize,
    pub detached: bool,
}

#[derive(Clone, Serialize)]
pub struct GraphComponent {
    pub id: usize,
    pub size: usize,
    #[serde(rename = "edges")]
    pub edge_count: usize,
    pub nodes: Vec<String>,
    pub isolated_count: usize,
    pub sample: String,
    pub loc_sum: usize,
    pub detached: bool,
    pub tauri_frontend: usize,
    pub tauri_backend: usize,
}

#[derive(Clone, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(String, String, String)>, // from, to, kind
    pub components: Vec<GraphComponent>,
    pub main_component_id: usize,
    /// Whether this graph was truncated due to size limits
    #[serde(default)]
    pub truncated: bool,
    /// Total number of nodes before truncation (same as nodes.len() if not truncated)
    #[serde(default)]
    pub total_nodes: usize,
    /// Total number of edges before truncation (same as edges.len() if not truncated)
    #[serde(default)]
    pub total_edges: usize,
    /// Reason for truncation, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_reason: Option<String>,
}

/// Location of a duplicate export with line number
#[derive(Clone, Serialize)]
pub struct DupLocation {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// Severity levels for duplicate exports
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DupSeverity {
    /// Cross-language expected (Rust↔TS DTOs) - noise
    CrossLangExpected = 0,
    /// Same-package TS duplicate - potential issue
    #[default]
    SamePackage = 1,
    /// Semantic conflict (different meanings) - needs attention
    SemanticConflict = 2,
}

#[derive(Clone, Serialize)]
pub struct RankedDup {
    pub name: String,
    pub files: Vec<String>,
    /// Locations with line numbers (file, line)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub locations: Vec<DupLocation>,
    pub score: usize,
    pub prod_count: usize,
    pub dev_count: usize,
    pub canonical: String,
    /// Line number in canonical file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_line: Option<usize>,
    pub refactors: Vec<String>,
    /// Severity level: 0=cross-lang expected, 1=same-package, 2=semantic conflict
    #[serde(default)]
    pub severity: DupSeverity,
    /// True if duplicate spans multiple languages (Rust↔TS)
    #[serde(default)]
    pub is_cross_lang: bool,
    /// Distinct packages/directories containing this symbol
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
    /// Explanation for the severity classification
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

/// Full command bridge for FE↔BE comparison table.
/// Represents a single command with all its frontend calls and backend handler.
#[derive(Clone, Serialize)]
pub struct CommandBridge {
    /// Command name (exposed_name from Tauri)
    pub name: String,
    /// Frontend call locations (file, line)
    pub fe_locations: Vec<(String, usize)>,
    /// Backend handler location (file, line, impl_symbol) - None if missing
    pub be_location: Option<(String, usize, String)>,
    /// Status: "ok", "missing_handler", "unused_handler", "unregistered_handler"
    pub status: String,
    /// Language (ts, rs, etc.)
    pub language: String,
}

#[derive(Clone, Default, Serialize)]
pub struct TreeNode {
    pub path: String,
    pub loc: usize,
    #[serde(default)]
    pub children: Vec<TreeNode>,
}

#[derive(Serialize)]
pub struct ReportSection {
    pub root: String,
    pub files_analyzed: usize,
    pub total_loc: usize,
    pub reexport_files_count: usize,
    pub dynamic_imports_count: usize,
    pub ranked_dups: Vec<RankedDup>,
    pub cascades: Vec<(String, String)>,
    /// Actual circular import components (normalized)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circular_imports: Vec<Vec<String>>,
    /// Lazy circular imports (broken by lazy imports inside functions)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lazy_circular_imports: Vec<Vec<String>>,
    pub dynamic: Vec<(String, Vec<String>)>,
    pub analyze_limit: usize,
    pub missing_handlers: Vec<CommandGap>,
    /// Backend handlers that exist (`#[tauri::command]`) but are never
    /// registered via `tauri::generate_handler![...]`.
    pub unregistered_handlers: Vec<CommandGap>,
    pub unused_handlers: Vec<CommandGap>,
    pub command_counts: (usize, usize),
    /// Full command bridges for FE↔BE comparison table
    pub command_bridges: Vec<CommandBridge>,
    pub open_base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<Vec<TreeNode>>,
    pub graph: Option<GraphData>,
    pub graph_warning: Option<String>,
    pub insights: Vec<AiInsight>,
    pub git_branch: Option<String>,
    pub git_commit: Option<String>,
    /// Crowd analysis results (naming collision detection)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crowds: Vec<Crowd>,
    /// Dead exports (exported but never imported)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dead_exports: Vec<DeadExport>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::CommandBridge;

    #[test]
    fn confidence_display_high() {
        assert_eq!(format!("{}", Confidence::High), "HIGH");
    }

    #[test]
    fn confidence_display_low() {
        assert_eq!(format!("{}", Confidence::Low), "LOW");
    }

    #[test]
    fn confidence_equality() {
        assert_eq!(Confidence::High, Confidence::High);
        assert_eq!(Confidence::Low, Confidence::Low);
        assert_ne!(Confidence::High, Confidence::Low);
    }

    #[test]
    fn string_literal_match_creation() {
        let m = StringLiteralMatch {
            file: "test.ts".to_string(),
            line: 42,
            context: "allowlist".to_string(),
        };
        assert_eq!(m.file, "test.ts");
        assert_eq!(m.line, 42);
        assert_eq!(m.context, "allowlist");
    }

    #[test]
    fn command_gap_creation() {
        let gap = CommandGap {
            name: "test_cmd".to_string(),
            implementation_name: Some("testCmd".to_string()),
            locations: vec![("test.ts".to_string(), 10)],
            confidence: Some(Confidence::High),
            string_literal_matches: vec![],
        };
        assert_eq!(gap.name, "test_cmd");
        assert_eq!(gap.implementation_name, Some("testCmd".to_string()));
        assert_eq!(gap.locations.len(), 1);
        assert_eq!(gap.confidence, Some(Confidence::High));
    }

    #[test]
    fn ai_insight_creation() {
        let insight = AiInsight {
            title: "Test Insight".to_string(),
            severity: "warning".to_string(),
            message: "Some message".to_string(),
        };
        assert_eq!(insight.title, "Test Insight");
        assert_eq!(insight.severity, "warning");
    }

    #[test]
    fn graph_node_creation() {
        let node = GraphNode {
            id: "src/main.ts".to_string(),
            label: "main.ts".to_string(),
            loc: 100,
            x: 0.5,
            y: 0.5,
            component: 0,
            degree: 3,
            detached: false,
        };
        assert_eq!(node.id, "src/main.ts");
        assert_eq!(node.loc, 100);
        assert!(!node.detached);
    }

    #[test]
    fn command_bridge_creation() {
        let bridge = CommandBridge {
            name: "get_user".to_string(),
            frontend_calls: vec![("src/app.ts".to_string(), 10)],
            backend_handler: Some(("src-tauri/src/lib.rs".to_string(), 20)),
            has_handler: true,
            is_called: true,
        };
        assert_eq!(bridge.name, "get_user");
        assert!(bridge.has_handler);
        assert!(bridge.backend_handler.is_some());
    }
}
