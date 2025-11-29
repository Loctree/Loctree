//! Report data types for structuring analysis results.
//!
//! These types define the data model for reports. They're designed to be:
//!
//! - **Serializable** - Easy JSON import/export via serde
//! - **Clone-friendly** - Components can share data without borrowing issues
//! - **Default-able** - Create partial reports with `..Default::default()`
//!
//! # Example
//!
//! ```rust
//! use report_leptos::types::{ReportSection, AiInsight, RankedDup};
//!
//! let section = ReportSection {
//!     root: "my-project/src".into(),
//!     files_analyzed: 42,
//!     insights: vec![
//!         AiInsight {
//!             title: "Circular Import Detected".into(),
//!             severity: "high".into(),
//!             message: "Consider breaking the cycle...".into(),
//!         }
//!     ],
//!     ranked_dups: vec![
//!         RankedDup {
//!             name: "formatDate".into(),
//!             files: vec!["utils/date.ts".into(), "helpers/format.ts".into()],
//!             score: 5,
//!             ..Default::default()
//!         }
//!     ],
//!     ..Default::default()
//! };
//! ```

use serde::{Deserialize, Serialize};

/// A gap between frontend command invocations and backend handlers.
///
/// Used for Tauri applications to track:
/// - Commands called from frontend but missing in backend
/// - Backend handlers that exist but aren't registered
/// - Registered handlers never called from frontend
///
/// # Example
///
/// ```rust
/// use report_leptos::types::CommandGap;
///
/// let gap = CommandGap {
///     name: "get_user_data".into(),
///     implementation_name: Some("getUserData".into()),
///     locations: vec![
///         ("src/api.ts".into(), 42),
///         ("src/components/Profile.tsx".into(), 15),
///     ],
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommandGap {
    /// Command name as invoked
    pub name: String,
    /// Actual implementation name (if different due to case conversion)
    pub implementation_name: Option<String>,
    /// File paths and line numbers where this command appears
    pub locations: Vec<(String, usize)>,
}

/// AI-generated insight about code quality or potential issues.
///
/// Insights are suggestions generated during analysis that highlight
/// patterns, anti-patterns, or areas for improvement.
///
/// # Severity Levels
///
/// - `"high"` - Critical issues requiring immediate attention
/// - `"medium"` - Important but not urgent
/// - `"low"` - Suggestions and nice-to-haves
///
/// # Example
///
/// ```rust
/// use report_leptos::types::AiInsight;
///
/// let insight = AiInsight {
///     title: "Large Module Detected".into(),
///     severity: "medium".into(),
///     message: "Consider splitting utils.ts (1500 LOC) into smaller modules.".into(),
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AiInsight {
    /// Short title for the insight
    pub title: String,
    /// Severity: "high", "medium", or "low"
    pub severity: String,
    /// Detailed explanation and recommendations
    pub message: String,
}

/// A node in the import/dependency graph.
///
/// Each node represents a file in the codebase with positioning
/// data for graph visualization.
///
/// # Fields
///
/// - `x`, `y` - Pre-computed positions (0.0-1.0 normalized)
/// - `component` - ID of the connected component this node belongs to
/// - `degree` - Number of edges (imports + exports)
/// - `detached` - True if node has no connections
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique identifier (usually file path)
    pub id: String,
    /// Display label (usually filename)
    pub label: String,
    /// Lines of code in this file
    pub loc: usize,
    /// X position (0.0-1.0)
    pub x: f32,
    /// Y position (0.0-1.0)
    pub y: f32,
    /// Connected component ID
    pub component: usize,
    /// Edge count (in + out degree)
    pub degree: usize,
    /// True if isolated (no imports or exports)
    pub detached: bool,
}

/// A connected component in the import graph.
///
/// The graph is decomposed into connected components to identify
/// isolated module clusters. The main component (largest) typically
/// represents the core application, while smaller components may
/// indicate dead code or independent utilities.
///
/// # Tauri Integration
///
/// For Tauri apps, `tauri_frontend` and `tauri_backend` count how many
/// nodes in this component belong to frontend vs backend code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphComponent {
    /// Component ID (0 = main/largest)
    pub id: usize,
    /// Number of nodes in this component
    pub size: usize,
    /// Number of edges in this component
    #[serde(rename = "edges")]
    pub edge_count: usize,
    /// List of node IDs in this component
    pub nodes: Vec<String>,
    /// Count of isolated (degree-0) nodes
    pub isolated_count: usize,
    /// Sample node for preview
    pub sample: String,
    /// Total lines of code in component
    pub loc_sum: usize,
    /// True if entirely detached from main component
    pub detached: bool,
    /// Count of frontend files (Tauri apps)
    pub tauri_frontend: usize,
    /// Count of backend files (Tauri apps)
    pub tauri_backend: usize,
}

/// Complete graph data for visualization.
///
/// Contains all nodes, edges, and component metadata needed
/// to render an interactive dependency graph with Cytoscape.js.
///
/// # Example
///
/// ```rust
/// use report_leptos::types::{GraphData, GraphNode, GraphComponent};
///
/// let graph = GraphData {
///     nodes: vec![
///         GraphNode {
///             id: "src/main.ts".into(),
///             label: "main.ts".into(),
///             loc: 150,
///             x: 0.5,
///             y: 0.5,
///             component: 0,
///             degree: 5,
///             detached: false,
///         }
///     ],
///     edges: vec![
///         ("src/main.ts".into(), "src/utils.ts".into(), "import".into()),
///     ],
///     components: vec![],
///     main_component_id: 0,
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphData {
    /// All nodes (files) in the graph
    pub nodes: Vec<GraphNode>,
    /// Edges as (from, to, kind) tuples
    pub edges: Vec<(String, String, String)>,
    /// Connected components
    pub components: Vec<GraphComponent>,
    /// ID of the main (largest) component
    pub main_component_id: usize,
}

/// A duplicate export found across multiple files.
///
/// Identifies symbols (functions, classes, types) that are exported
/// from multiple locations, which may indicate copy-paste code or
/// naming collisions.
///
/// # Scoring
///
/// The `score` combines frequency and context:
/// - Higher score = more problematic
/// - `prod_count` vs `dev_count` helps prioritize production code
///
/// # Example
///
/// ```rust
/// use report_leptos::types::RankedDup;
///
/// let dup = RankedDup {
///     name: "formatDate".into(),
///     files: vec![
///         "src/utils/date.ts".into(),
///         "src/helpers/format.ts".into(),
///         "src/legacy/utils.ts".into(),
///     ],
///     score: 15,
///     prod_count: 2,
///     dev_count: 1,
///     canonical: "src/utils/date.ts".into(),
///     refactors: vec![
///         "src/helpers/format.ts".into(),
///         "src/legacy/utils.ts".into(),
///     ],
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RankedDup {
    /// Export name
    pub name: String,
    /// All files exporting this symbol
    pub files: Vec<String>,
    /// Priority score (higher = more important to fix)
    pub score: usize,
    /// Count in production code paths
    pub prod_count: usize,
    /// Count in dev/test code paths
    pub dev_count: usize,
    /// Recommended canonical location
    pub canonical: String,
    /// Files that should import from canonical instead
    pub refactors: Vec<String>,
}

/// A complete report section for one analyzed directory.
///
/// This is the main data structure passed to [`crate::render_report`].
/// Each section represents analysis results for one source root.
///
/// # Example
///
/// ```rust
/// use report_leptos::types::ReportSection;
///
/// let section = ReportSection {
///     root: "packages/my-app/src".into(),
///     files_analyzed: 234,
///     analyze_limit: 500,
///     command_counts: (15, 18), // 15 frontend calls, 18 backend handlers
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReportSection {
    /// Root directory that was analyzed
    pub root: String,
    /// Number of files analyzed
    pub files_analyzed: usize,
    /// Duplicate exports ranked by priority
    pub ranked_dups: Vec<RankedDup>,
    /// Cascade import pairs (source, target)
    pub cascades: Vec<(String, String)>,
    /// Dynamic imports per file
    pub dynamic: Vec<(String, Vec<String>)>,
    /// Maximum files to analyze (0 = unlimited)
    pub analyze_limit: usize,
    /// Frontend commands missing backend handlers
    pub missing_handlers: Vec<CommandGap>,
    /// Backend handlers not registered in generate_handler![]
    pub unregistered_handlers: Vec<CommandGap>,
    /// Registered handlers never called from frontend
    pub unused_handlers: Vec<CommandGap>,
    /// (frontend_command_count, backend_handler_count)
    pub command_counts: (usize, usize),
    /// Base URL for opening files in editor
    pub open_base: Option<String>,
    /// Dependency graph data (if generated)
    pub graph: Option<GraphData>,
    /// Warning if graph was skipped (too large, etc.)
    pub graph_warning: Option<String>,
    /// AI-generated insights
    pub insights: Vec<AiInsight>,
}
