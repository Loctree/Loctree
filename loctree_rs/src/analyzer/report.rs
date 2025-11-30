use serde::Serialize;

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
}

#[derive(Clone, Serialize)]
pub struct RankedDup {
    pub name: String,
    pub files: Vec<String>,
    pub score: usize,
    pub prod_count: usize,
    pub dev_count: usize,
    pub canonical: String,
    pub refactors: Vec<String>,
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

#[derive(Serialize)]
pub struct ReportSection {
    pub root: String,
    pub files_analyzed: usize,
    pub total_loc: usize,
    pub reexport_files_count: usize,
    pub dynamic_imports_count: usize,
    pub ranked_dups: Vec<RankedDup>,
    pub cascades: Vec<(String, String)>,
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
    pub graph: Option<GraphData>,
    pub graph_warning: Option<String>,
    pub insights: Vec<AiInsight>,
}
