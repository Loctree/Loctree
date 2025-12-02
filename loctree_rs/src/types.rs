use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const DEFAULT_LOC_THRESHOLD: usize = 1000;
pub const COLOR_RED: &str = "\u{001b}[31m";
pub const COLOR_RESET: &str = "\u{001b}[0m";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputMode {
    Human,
    Json,
    Jsonl,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    Tree,
    AnalyzeImports,
    /// Initialize/update snapshot (scan once)
    Init,
    /// VS2 Holographic Slice - extract context for a file
    Slice,
    /// Trace a handler - show full investigation path and WHY it's unused/missing
    Trace,
    /// AI-optimized hierarchical output with quick wins and slice references
    ForAi,
    /// Git awareness - temporal knowledge from repository history
    Git(GitSubcommand),
    /// Unified search - returns symbol matches, semantic matches, dead status in one call
    Search,
}

/// Git subcommands for temporal awareness - semantic analysis only (no passthrough)
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GitSubcommand {
    /// Semantic diff between two commits (snapshot comparison)
    /// Shows: files changed, graph delta, exports delta, dead code delta, impact analysis
    Compare {
        /// Starting commit (e.g., "HEAD~1", "abc123")
        from: String,
        /// Ending commit, defaults to current working tree if None
        to: Option<String>,
    },
    /// Symbol-level blame: which commit introduced each symbol/import
    Blame {
        /// File to analyze
        file: String,
    },
    /// Track evolution of a symbol or file's structure over time
    History {
        /// Symbol name to track (e.g., "processUser")
        symbol: Option<String>,
        /// File path to track
        file: Option<String>,
        /// Maximum number of commits to show
        limit: usize,
    },
    /// Find when a pattern was introduced (circular import, dead code, etc.)
    WhenIntroduced {
        /// Circular import pattern (e.g., "src/a.rs <-> src/b.rs")
        circular: Option<String>,
        /// Dead code symbol (e.g., "src/utils.rs::unused_fn")
        dead: Option<String>,
        /// Import source (e.g., "lodash")
        import: Option<String>,
    },
}

#[derive(Clone)]
pub struct Options {
    pub extensions: Option<HashSet<String>>,
    pub ignore_paths: Vec<std::path::PathBuf>,
    pub use_gitignore: bool,
    pub max_depth: Option<usize>,
    pub color: ColorMode,
    pub output: OutputMode,
    pub summary: bool,
    pub summary_limit: usize,
    pub show_hidden: bool,
    pub show_ignored: bool,
    pub loc_threshold: usize,
    pub analyze_limit: usize,
    pub report_path: Option<std::path::PathBuf>,
    pub serve: bool,
    #[allow(dead_code)]
    pub editor_cmd: Option<String>,
    pub max_graph_nodes: Option<usize>,
    pub max_graph_edges: Option<usize>,
    pub verbose: bool,
    pub scan_all: bool,
    pub symbol: Option<String>,
    pub impact: Option<String>,
    pub find_artifacts: bool,
}

pub struct LineEntry {
    pub label: String,
    pub loc: Option<usize>,
    pub relative_path: String,
    pub is_dir: bool,
    pub is_large: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymbolMatch {
    pub line: usize,
    pub context: String,
}

pub struct LargeEntry {
    pub path: String,
    pub loc: usize,
}

#[derive(Default)]
pub struct Stats {
    pub directories: usize,
    pub files: usize,
    pub files_with_loc: usize,
    pub total_loc: usize,
}

pub struct Collectors<'a> {
    pub entries: &'a mut Vec<LineEntry>,
    pub large_entries: &'a mut Vec<LargeEntry>,
    pub stats: &'a mut Stats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportEntry {
    pub source: String,
    pub source_raw: String,
    pub kind: ImportKind,
    pub resolved_path: Option<String>,
    pub is_bare: bool,
    pub symbols: Vec<ImportSymbol>,
    pub resolution: ImportResolutionKind,
    pub is_type_checking: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ImportKind {
    Static,
    SideEffect,
    Dynamic,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportResolutionKind {
    Local,
    Stdlib,
    Dynamic,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportSymbol {
    pub name: String,
    pub alias: Option<String>,
    /// True if this is a default import (import Foo from './bar')
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReexportEntry {
    pub source: String,
    pub kind: ReexportKind,
    pub resolved: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReexportKind {
    Star,
    Named(Vec<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportSymbol {
    pub name: String,
    pub kind: String,
    pub export_type: String,
    pub line: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandRef {
    pub name: String,
    pub exposed_name: Option<String>,
    pub line: usize,
    pub generic_type: Option<String>,
    pub payload: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventRef {
    pub raw_name: Option<String>,
    pub name: String,
    pub line: usize,
    pub kind: String,
    pub awaited: bool,
    pub payload: Option<String>,
}

/// Python concurrency race indicator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PyRaceIndicator {
    /// Line number where the pattern was found
    pub line: usize,
    /// Type of concurrency pattern: "threading", "asyncio", "multiprocessing"
    pub concurrency_type: String,
    /// Specific pattern: "Thread", "Lock", "gather", "create_task", "Pool", etc.
    pub pattern: String,
    /// Risk level: "info", "warning", "high"
    pub risk: String,
    /// Description of the potential issue
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FileAnalysis {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub loc: usize,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub is_test: bool,
    #[serde(default)]
    pub is_generated: bool,
    #[serde(default)]
    pub imports: Vec<ImportEntry>,
    #[serde(default)]
    pub reexports: Vec<ReexportEntry>,
    #[serde(default)]
    pub dynamic_imports: Vec<String>,
    #[serde(default)]
    pub exports: Vec<ExportSymbol>,
    #[serde(default)]
    pub command_calls: Vec<CommandRef>,
    #[serde(default)]
    pub command_handlers: Vec<CommandRef>,
    #[serde(default)]
    pub event_emits: Vec<EventRef>,
    #[serde(default)]
    pub event_listens: Vec<EventRef>,
    #[serde(default)]
    pub event_consts: HashMap<String, String>,
    #[serde(default)]
    pub matches: Vec<SymbolMatch>,
    #[serde(default)]
    pub entry_points: Vec<String>,
    /// Names of Rust functions registered via `tauri::generate_handler![...]` in this file
    #[serde(default)]
    pub tauri_registered_handlers: Vec<String>,
    /// File modification time (Unix timestamp) for incremental scanning
    #[serde(default)]
    pub mtime: u64,
    /// File size in bytes for incremental scanning (combined with mtime for accuracy)
    #[serde(default)]
    pub size: u64,
    /// Python concurrency race indicators (threading/asyncio/multiprocessing patterns)
    #[serde(default)]
    pub py_race_indicators: Vec<PyRaceIndicator>,
}

impl ImportEntry {
    pub fn new(source: String, kind: ImportKind) -> Self {
        let is_bare = !source.starts_with('.') && !source.starts_with('/');
        Self {
            source_raw: source.clone(),
            source,
            kind,
            resolved_path: None,
            is_bare,
            symbols: Vec::new(),
            resolution: ImportResolutionKind::Unknown,
            is_type_checking: false,
        }
    }
}

impl ExportSymbol {
    pub fn new(name: String, kind: &str, export_type: &str, line: Option<usize>) -> Self {
        Self {
            name,
            kind: kind.to_string(),
            export_type: export_type.to_string(),
            line,
        }
    }
}

impl FileAnalysis {
    pub fn new(path: String) -> Self {
        Self {
            path,
            loc: 0,
            language: String::new(),
            kind: "code".to_string(),
            is_test: false,
            is_generated: false,
            imports: Vec::new(),
            reexports: Vec::new(),
            dynamic_imports: Vec::new(),
            exports: Vec::new(),
            command_calls: Vec::new(),
            command_handlers: Vec::new(),
            event_emits: Vec::new(),
            event_listens: Vec::new(),
            event_consts: HashMap::new(),
            matches: Vec::new(),
            entry_points: Vec::new(),
            tauri_registered_handlers: Vec::new(),
            py_race_indicators: Vec::new(),
            mtime: 0,
            size: 0,
        }
    }
}

// Convenience type aliases reused across modules
pub type ExportIndex = HashMap<String, Vec<String>>;
pub type PayloadEntry = (String, usize, Option<String>);
pub type PayloadMap = HashMap<String, Vec<PayloadEntry>>;
