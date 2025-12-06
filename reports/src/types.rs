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

/// Confidence level for detection results.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// High confidence - likely a real issue
    High,
    /// Low confidence - may be false positive due to dynamic usage
    Low,
}

/// A string literal that might indicate dynamic command usage.
///
/// Used to track potential false positives when detecting unused handlers.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StringLiteralMatch {
    /// File path where the literal was found
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Context type: "allowlist", "const", "object_key", "array_item"
    pub context: String,
}

/// Full command bridge for FE↔BE comparison table.
///
/// Represents a single Tauri command with all frontend call sites
/// and the corresponding backend handler location.
///
/// # Status Values
///
/// - `"ok"` - Command properly matched (FE calls + BE handler)
/// - `"missing_handler"` - FE calls exist but no BE handler
/// - `"unused_handler"` - BE handler exists but no FE calls
/// - `"unregistered_handler"` - BE handler exists but not in generate_handler![]
///
/// # Example
///
/// ```rust
/// use report_leptos::types::CommandBridge;
///
/// let bridge = CommandBridge {
///     name: "get_user".into(),
///     fe_locations: vec![
///         ("src/api.ts".into(), 42),
///         ("src/components/Profile.tsx".into(), 15),
///     ],
///     be_location: Some(("src-tauri/src/commands/user.rs".into(), 10, "get_user".into())),
///     status: "ok".into(),
///     language: "rs".into(),
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommandBridge {
    /// Command name (exposed_name from Tauri)
    pub name: String,
    /// Frontend call locations (file, line)
    #[serde(default)]
    pub fe_locations: Vec<(String, usize)>,
    /// Backend handler location (file, line, impl_symbol) - None if missing
    pub be_location: Option<(String, usize, String)>,
    /// Status: "ok", "missing_handler", "unused_handler", "unregistered_handler"
    pub status: String,
    /// Language (ts, rs, etc.)
    #[serde(default)]
    pub language: String,
}

/// Directory or file node used by the report tree view.
///
/// Contains relative path, aggregated LOC, and children.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TreeNode {
    /// Relative path of this file/directory.
    pub path: String,
    /// Lines of code aggregated for this node (file LOC + children).
    pub loc: usize,
    /// Child nodes.
    #[serde(default)]
    pub children: Vec<TreeNode>,
}

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
///     ..Default::default()
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
    /// Detection confidence level
    pub confidence: Option<Confidence>,
    /// String literal matches that may indicate dynamic usage
    #[serde(default)]
    pub string_literal_matches: Vec<StringLiteralMatch>,
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
/// to render an interactive dependency graph with Cytoscape.js or DOT/graphviz.
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
///
/// // Convert to DOT format for graphviz/dot_ix
/// let dot = graph.to_dot();
/// assert!(dot.contains("digraph"));
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

impl GraphData {
    /// Convert graph data to DOT format for graphviz/dot_ix rendering.
    ///
    /// Generates a DOT language representation with:
    /// - Node styling based on LOC (size), component (color), detached status
    /// - Edge styling based on kind (import vs reexport)
    /// - Subgraph clusters for connected components
    ///
    /// # Example
    ///
    /// ```rust
    /// use report_leptos::types::GraphData;
    ///
    /// let graph = GraphData::default();
    /// let dot = graph.to_dot();
    /// assert!(dot.starts_with("digraph loctree"));
    /// ```
    pub fn to_dot(&self) -> String {
        let mut dot = String::with_capacity(self.nodes.len() * 100 + self.edges.len() * 50);

        dot.push_str("digraph loctree {\n");
        dot.push_str("  // Graph attributes\n");
        dot.push_str(
            "  graph [rankdir=TB, splines=true, overlap=false, nodesep=0.5, ranksep=0.8];\n",
        );
        dot.push_str(
            "  node [shape=box, style=\"rounded,filled\", fontname=\"sans-serif\", fontsize=10];\n",
        );
        dot.push_str("  edge [arrowsize=0.7, fontsize=8];\n\n");

        // Group nodes by component for subgraph clustering
        let mut component_nodes: std::collections::HashMap<usize, Vec<&GraphNode>> =
            std::collections::HashMap::new();
        for node in &self.nodes {
            component_nodes
                .entry(node.component)
                .or_default()
                .push(node);
        }

        // Render each component as a subgraph cluster
        for (comp_id, nodes) in &component_nodes {
            let is_main = *comp_id == self.main_component_id;
            let cluster_style = if is_main {
                "style=invis" // Main component: no visible cluster border
            } else {
                "style=dashed, color=\"#888888\"" // Other components: dashed border
            };

            dot.push_str(&format!("  subgraph cluster_{} {{\n", comp_id));
            dot.push_str(&format!("    {};\n", cluster_style));
            dot.push_str(&format!("    label=\"Component {}\";\n", comp_id));

            for node in nodes {
                let escaped_id = escape_dot_string(&node.id);
                let escaped_label = escape_dot_string(&node.label);

                // Node color based on status
                let fill_color = if node.detached {
                    "#d1830f" // Orange for detached
                } else if *comp_id == self.main_component_id {
                    "#4f81e1" // Blue for main component
                } else {
                    "#6c757d" // Gray for other components
                };

                // Node size based on LOC (min 0.3, max 1.5)
                let size = 0.3 + (node.loc as f32 / 500.0).min(1.2);

                dot.push_str(&format!(
                    "    \"{}\" [label=\"{}\\n({} LOC)\", fillcolor=\"{}\", width={:.2}, height={:.2}];\n",
                    escaped_id, escaped_label, node.loc, fill_color, size, size * 0.6
                ));
            }

            dot.push_str("  }\n\n");
        }

        // Render edges
        dot.push_str("  // Edges\n");
        for (from, to, kind) in &self.edges {
            let escaped_from = escape_dot_string(from);
            let escaped_to = escape_dot_string(to);

            let edge_style = match kind.as_str() {
                "reexport" => "color=\"#e67e22\", style=bold",
                _ => "color=\"#888888\"",
            };

            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [{}];\n",
                escaped_from, escaped_to, edge_style
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Convert graph data to DOT format with dark theme colors.
    pub fn to_dot_dark(&self) -> String {
        // Same structure but with dark-theme-appropriate colors
        let mut dot = String::with_capacity(self.nodes.len() * 100 + self.edges.len() * 50);

        dot.push_str("digraph loctree {\n");
        dot.push_str("  graph [rankdir=TB, splines=true, overlap=false, nodesep=0.5, ranksep=0.8, bgcolor=\"#0f1115\"];\n");
        dot.push_str("  node [shape=box, style=\"rounded,filled\", fontname=\"sans-serif\", fontsize=10, fontcolor=\"#eef2ff\"];\n");
        dot.push_str("  edge [arrowsize=0.7, fontsize=8, fontcolor=\"#aaa\"];\n\n");

        // Simplified rendering for dark theme (same structure, different colors)
        for node in &self.nodes {
            let escaped_id = escape_dot_string(&node.id);
            let escaped_label = escape_dot_string(&node.label);

            let fill_color = if node.detached {
                "#d1830f"
            } else if node.component == self.main_component_id {
                "#4f81e1"
            } else {
                "#4a5568"
            };

            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\\n({} LOC)\", fillcolor=\"{}\"];\n",
                escaped_id, escaped_label, node.loc, fill_color
            ));
        }

        for (from, to, kind) in &self.edges {
            let escaped_from = escape_dot_string(from);
            let escaped_to = escape_dot_string(to);

            let edge_style = match kind.as_str() {
                "reexport" => "color=\"#e67e22\"",
                _ => "color=\"#666666\"",
            };

            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [{}];\n",
                escaped_from, escaped_to, edge_style
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

/// Escape special characters for DOT string literals.
fn escape_dot_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
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

/// Match reason for crowd membership (mirrors loctree_rs::analyzer::crowd::MatchReason).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MatchReason {
    /// File/export name matches pattern
    NameMatch {
        /// The matched string (filename, export name, etc.)
        matched: String,
    },
    /// High import similarity with other crowd members
    ImportSimilarity {
        /// Similarity score (0.0-1.0)
        similarity: f32,
    },
    /// Exports similar types/functions
    ExportSimilarity {
        /// File this one is similar to
        similar_to: String,
    },
}

/// Issue detected in a crowd (mirrors loctree_rs::analyzer::crowd::CrowdIssue).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrowdIssue {
    /// Multiple files with very similar names
    NameCollision {
        /// Files with colliding names
        files: Vec<String>,
    },
    /// Some files have much lower usage than others
    UsageAsymmetry {
        /// The primary/most-used file
        primary: String,
        /// Underused files that might be redundant
        underused: Vec<String>,
    },
    /// Files export similar things
    ExportOverlap {
        /// Files with overlapping exports
        files: Vec<String>,
        /// Overlapping export names
        overlap: Vec<String>,
    },
    /// Related functionality is scattered
    Fragmentation {
        /// Categories/themes found scattered across crowd
        categories: Vec<String>,
    },
}

/// A member of a crowd.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrowdMember {
    /// File path
    pub file: String,
    /// Why this file matched the crowd
    pub match_reason: MatchReason,
    /// Number of files importing this one
    pub importer_count: usize,
    /// Similarity scores with other members (file, score)
    #[serde(default)]
    pub similarity_scores: Vec<(String, f32)>,
}

/// A group of files with similar names/patterns.
///
/// Crowds indicate potential naming collisions, fragmentation,
/// or copy-paste duplication across the codebase.
///
/// # Scoring
///
/// - 0-4: Low severity (acceptable naming patterns)
/// - 4-7: Medium severity (worth reviewing)
/// - 7-10: High severity (likely problematic)
///
/// # Example
///
/// ```rust
/// use report_leptos::types::{Crowd, CrowdMember, MatchReason, CrowdIssue};
///
/// let crowd = Crowd {
///     pattern: "message".into(),
///     members: vec![
///         CrowdMember {
///             file: "src/message.ts".into(),
///             match_reason: MatchReason::NameMatch {
///                 matched: "message".into(),
///             },
///             importer_count: 15,
///             similarity_scores: vec![],
///         },
///         CrowdMember {
///             file: "src/components/Message.tsx".into(),
///             match_reason: MatchReason::NameMatch {
///                 matched: "Message".into(),
///             },
///             importer_count: 8,
///             similarity_scores: vec![],
///         },
///     ],
///     score: 6.5,
///     issues: vec![
///         CrowdIssue::NameCollision {
///             files: vec!["src/message.ts".into(), "src/components/Message.tsx".into()],
///         },
///     ],
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Crowd {
    /// Pattern name (e.g., "message", "chat")
    pub pattern: String,
    /// Files matching this pattern
    pub members: Vec<CrowdMember>,
    /// Severity score (0-10, higher = worse)
    pub score: f32,
    /// Issues detected in this crowd
    #[serde(default)]
    pub issues: Vec<CrowdIssue>,
}

/// A dead export (symbol exported but never imported).
///
/// Represents code that appears to be unused - exported from a file
/// but never imported anywhere in the analyzed codebase.
///
/// # Example
///
/// ```rust
/// use report_leptos::types::DeadExport;
///
/// let dead = DeadExport {
///     file: "src/utils/legacy.ts".into(),
///     symbol: "formatOldDate".into(),
///     line: Some(42),
///     confidence: "very-high".into(),
///     reason: "No imports found in codebase".into(),
///     open_url: Some("loctree://open?f=src/utils/legacy.ts&l=42".into()),
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeadExport {
    /// File path containing the dead export
    pub file: String,
    /// Symbol name (function, class, type, etc.)
    pub symbol: String,
    /// Line number where symbol is defined (1-indexed)
    pub line: Option<usize>,
    /// Confidence level: "high", "very-high"
    pub confidence: String,
    /// Human-readable reason why this is considered dead
    pub reason: String,
    /// Optional URL for opening in editor (loctree://open protocol)
    pub open_url: Option<String>,
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
    /// Total lines of code across all analyzed files
    pub total_loc: usize,
    /// Number of files with re-exports (barrel files)
    pub reexport_files_count: usize,
    /// Number of files with dynamic imports
    pub dynamic_imports_count: usize,
    /// Duplicate exports ranked by priority
    pub ranked_dups: Vec<RankedDup>,
    /// Cascade import pairs (source, target)
    pub cascades: Vec<(String, String)>,
    /// Circular import components (strict cycles)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub circular_imports: Vec<Vec<String>>,
    /// Lazy circular imports (cycles only via dynamic/lazy imports)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lazy_circular_imports: Vec<Vec<String>>,
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
    /// Full command bridges for FE↔BE comparison table
    #[serde(default)]
    pub command_bridges: Vec<CommandBridge>,
    /// Base URL for opening files in editor
    pub open_base: Option<String>,
    /// Directory tree with LOC per node
    #[serde(default)]
    pub tree: Option<Vec<TreeNode>>,
    /// Dependency graph data (if generated)
    pub graph: Option<GraphData>,
    /// Warning if graph was skipped (too large, etc.)
    pub graph_warning: Option<String>,
    /// AI-generated insights
    pub insights: Vec<AiInsight>,
    /// Git branch name (if available)
    pub git_branch: Option<String>,
    /// Git commit hash (if available)
    pub git_commit: Option<String>,
    /// Crowd analysis results (naming collision detection)
    #[serde(default)]
    pub crowds: Vec<Crowd>,
    /// Dead exports (exported but never imported)
    #[serde(default)]
    pub dead_exports: Vec<DeadExport>,
}
