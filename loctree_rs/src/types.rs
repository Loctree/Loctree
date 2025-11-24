use std::collections::{HashMap, HashSet};

pub const DEFAULT_LOC_THRESHOLD: usize = 1000;
pub const COLOR_RED: &str = "\u{001b}[31m";
pub const COLOR_RESET: &str = "\u{001b}[0m";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorMode {
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Tree,
    AnalyzeImports,
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
    pub loc_threshold: usize,
    pub analyze_limit: usize,
    pub report_path: Option<std::path::PathBuf>,
    pub serve: bool,
    #[allow(dead_code)]
    pub editor_cmd: Option<String>,
    pub max_graph_nodes: Option<usize>,
    pub max_graph_edges: Option<usize>,
    pub verbose: bool,
}

pub struct LineEntry {
    pub label: String,
    pub loc: Option<usize>,
    pub relative_path: String,
    pub is_dir: bool,
    pub is_large: bool,
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

#[derive(Clone)]
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

#[derive(Clone)]
pub enum ImportKind {
    Static,
    SideEffect,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportResolutionKind {
    Local,
    Stdlib,
    Dynamic,
    Unknown,
}

#[derive(Clone)]
pub struct ImportSymbol {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReexportEntry {
    pub source: String,
    pub kind: ReexportKind,
    pub resolved: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ReexportKind {
    Star,
    Named(Vec<String>),
}

#[derive(Clone)]
pub struct ExportSymbol {
    pub name: String,
    pub kind: String,
    pub export_type: String,
    pub line: Option<usize>,
}

#[derive(Clone)]
pub struct CommandRef {
    pub name: String,
    pub exposed_name: Option<String>,
    pub line: usize,
    pub generic_type: Option<String>,
    pub payload: Option<String>,
}

#[derive(Clone)]
pub struct EventRef {
    pub raw_name: Option<String>,
    pub name: String,
    pub line: usize,
    pub kind: String,
    pub awaited: bool,
    pub payload: Option<String>,
}

#[derive(Clone)]
pub struct FileAnalysis {
    pub path: String,
    pub loc: usize,
    pub language: String,
    pub kind: String,
    pub is_test: bool,
    pub is_generated: bool,
    pub imports: Vec<ImportEntry>,
    pub reexports: Vec<ReexportEntry>,
    pub dynamic_imports: Vec<String>,
    pub exports: Vec<ExportSymbol>,
    pub command_calls: Vec<CommandRef>,
    pub command_handlers: Vec<CommandRef>,
    pub event_emits: Vec<EventRef>,
    pub event_listens: Vec<EventRef>,
    pub event_consts: HashMap<String, String>,
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
        }
    }
}

// Convenience type aliases reused across modules
pub type ExportIndex = HashMap<String, Vec<String>>;
