use serde::Serialize;

#[derive(Clone)]
pub struct CommandGap {
    pub name: String,
    pub implementation_name: Option<String>,
    pub locations: Vec<(String, usize)>,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct RankedDup {
    pub name: String,
    pub files: Vec<String>,
    pub score: usize,
    pub prod_count: usize,
    pub dev_count: usize,
    pub canonical: String,
    pub refactors: Vec<String>,
}

pub struct ReportSection {
    pub root: String,
    pub files_analyzed: usize,
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
    pub open_base: Option<String>,
    pub graph: Option<GraphData>,
    pub graph_warning: Option<String>,
    pub insights: Vec<AiInsight>,
}
