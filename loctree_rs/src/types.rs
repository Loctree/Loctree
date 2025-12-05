//! Core types for loctree analysis.
//!
//! This module defines the fundamental data structures used throughout loctree:
//! - [`FileAnalysis`] - Per-file analysis result (imports, exports, commands)
//! - [`ImportEntry`] / [`ExportSymbol`] - Import/export representations
//! - [`CommandRef`] / [`EventRef`] - Tauri command and event tracking
//! - [`Mode`] - CLI operation modes
//! - [`Options`] - Analysis configuration

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Default LOC threshold for "large file" warnings.
pub const DEFAULT_LOC_THRESHOLD: usize = 1000;

/// ANSI escape code for red text.
pub const COLOR_RED: &str = "\u{001b}[31m";

/// ANSI escape code to reset text color.
pub const COLOR_RESET: &str = "\u{001b}[0m";

/// Terminal color mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ColorMode {
    /// Detect TTY and colorize if interactive.
    #[default]
    Auto,
    /// Always use ANSI colors.
    Always,
    /// Never use colors (for piping/CI).
    Never,
}

/// Output format for analysis results.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputMode {
    /// Human-readable text with colors and formatting.
    Human,
    /// Pretty-printed JSON object.
    Json,
    /// Newline-delimited JSON (one object per line).
    Jsonl,
}

/// CLI operation mode - determines what loctree does.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Display directory tree with LOC counts (default without -A).
    Tree,
    /// Full import/export analysis (-A flag).
    AnalyzeImports,
    /// Initialize/update snapshot (scan once).
    Init,
    /// Holographic Slice - extract file + deps + consumers for AI context.
    Slice,
    /// Trace a handler - show full investigation path and WHY it's unused/missing.
    Trace,
    /// AI-optimized JSON output with quick wins and slice references.
    ForAi,
    /// Git awareness - temporal knowledge from repository history.
    Git(GitSubcommand),
    /// Unified search - returns symbol matches, semantic matches, dead status.
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

/// Analysis configuration options.
///
/// Controls file filtering, output format, and analysis behavior.
#[derive(Clone)]
pub struct Options {
    /// File extensions to include (None = all supported).
    pub extensions: Option<HashSet<String>>,
    /// Paths to exclude from analysis.
    pub ignore_paths: Vec<std::path::PathBuf>,
    /// Respect .gitignore rules.
    pub use_gitignore: bool,
    /// Maximum directory depth for tree view.
    pub max_depth: Option<usize>,
    /// Terminal color mode.
    pub color: ColorMode,
    /// Output format (Human, Json, Jsonl).
    pub output: OutputMode,
    /// Show summary statistics.
    pub summary: bool,
    /// Max items in summary lists.
    pub summary_limit: usize,
    /// Include dotfiles/directories.
    pub show_hidden: bool,
    /// Include gitignored files.
    pub show_ignored: bool,
    /// LOC threshold for "large file" warnings.
    pub loc_threshold: usize,
    /// Max files to analyze (0 = unlimited).
    pub analyze_limit: usize,
    /// Path for HTML report output.
    pub report_path: Option<std::path::PathBuf>,
    /// Start local server for HTML report.
    pub serve: bool,
    #[allow(dead_code)]
    /// Editor command for click-to-open (e.g., "code -g").
    pub editor_cmd: Option<String>,
    /// Max nodes in dependency graph.
    pub max_graph_nodes: Option<usize>,
    /// Max edges in dependency graph.
    pub max_graph_edges: Option<usize>,
    /// Enable verbose logging.
    pub verbose: bool,
    /// Scan all files (ignore incremental cache).
    pub scan_all: bool,
    /// Symbol to search for (--symbol flag).
    pub symbol: Option<String>,
    /// File for impact analysis (--impact flag).
    pub impact: Option<String>,
    /// Detect build artifacts (node_modules, target, etc.).
    pub find_artifacts: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            extensions: None,
            ignore_paths: Vec::new(),
            use_gitignore: true,
            max_depth: None,
            color: ColorMode::Auto,
            output: OutputMode::Human,
            summary: false,
            summary_limit: 50,
            show_hidden: false,
            show_ignored: false,
            loc_threshold: 500,
            analyze_limit: 100,
            report_path: None,
            serve: false,
            editor_cmd: None,
            max_graph_nodes: None,
            max_graph_edges: None,
            verbose: false,
            scan_all: false,
            symbol: None,
            impact: None,
            find_artifacts: false,
        }
    }
}

/// A single line in the tree output (file or directory).
pub struct LineEntry {
    /// Display label (filename with tree prefix).
    pub label: String,
    /// Lines of code (None for directories without aggregation).
    pub loc: Option<usize>,
    /// Path relative to scan root.
    pub relative_path: String,
    /// True if this is a directory.
    pub is_dir: bool,
    /// True if LOC exceeds threshold.
    pub is_large: bool,
}

/// A symbol match from search/grep operations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymbolMatch {
    /// 1-based line number.
    pub line: usize,
    /// Line content with match highlighted.
    pub context: String,
}

/// A file exceeding the LOC threshold.
pub struct LargeEntry {
    /// Relative path to file.
    pub path: String,
    /// Lines of code.
    pub loc: usize,
}

/// Aggregated scan statistics.
#[derive(Default)]
pub struct Stats {
    /// Total directories scanned.
    pub directories: usize,
    /// Total files scanned.
    pub files: usize,
    /// Files with countable LOC.
    pub files_with_loc: usize,
    /// Sum of all LOC.
    pub total_loc: usize,
}

/// Mutable collectors passed through tree traversal.
pub struct Collectors<'a> {
    /// Tree entries for display.
    pub entries: &'a mut Vec<LineEntry>,
    /// Files exceeding LOC threshold.
    pub large_entries: &'a mut Vec<LargeEntry>,
    /// Running statistics.
    pub stats: &'a mut Stats,
}

/// An import statement (JS/TS/Python).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportEntry {
    /// Resolved/normalized source path.
    pub source: String,
    /// Original source as written in code.
    pub source_raw: String,
    /// Import type (static, side-effect, dynamic).
    pub kind: ImportKind,
    /// Absolute resolved path (if local file).
    pub resolved_path: Option<String>,
    /// True if bare specifier (npm package, not relative).
    pub is_bare: bool,
    /// Imported symbols (named, default, namespace).
    pub symbols: Vec<ImportSymbol>,
    /// Resolution result (local, stdlib, dynamic, unknown).
    pub resolution: ImportResolutionKind,
    /// True if inside TYPE_CHECKING block (Python).
    pub is_type_checking: bool,
    /// True if placed inside a function/method (lazy import to break cycles).
    #[serde(default)]
    pub is_lazy: bool,
}

/// Type of import statement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ImportKind {
    /// `import X from 'y'` or `from x import y`
    Static,
    /// `import type { X } from 'y'` (TypeScript-only, still a real dependency)
    Type,
    /// `import 'styles.css'` (no bindings)
    SideEffect,
    /// `import('module')` or `React.lazy(() => import(...))`
    Dynamic,
}

/// How an import source was resolved.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportResolutionKind {
    /// Local file (relative or absolute path).
    Local,
    /// Standard library module.
    Stdlib,
    /// Dynamic import (path unknown at parse time).
    Dynamic,
    /// Could not resolve.
    Unknown,
}

/// A single symbol from an import statement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportSymbol {
    /// Original name in source module.
    pub name: String,
    /// Local alias (e.g., `import { foo as bar }`).
    pub alias: Option<String>,
    /// True if default import (`import Foo from './bar'`).
    #[serde(default)]
    pub is_default: bool,
}

/// A re-export statement (`export { x } from './y'` or `export * from './z'`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReexportEntry {
    /// Source module path.
    pub source: String,
    /// Star or named re-export.
    pub kind: ReexportKind,
    /// Resolved absolute path (if local).
    pub resolved: Option<String>,
}

/// Type of re-export.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReexportKind {
    /// `export * from './module'`
    Star,
    /// `export { a, b } from './module'`
    Named(Vec<String>),
}

/// An exported symbol from a module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportSymbol {
    /// Exported name (may differ from internal name).
    pub name: String,
    /// Symbol kind: "function", "class", "const", "type", etc.
    pub kind: String,
    /// Export type: "named", "default", "reexport".
    pub export_type: String,
    /// 1-based line number of declaration.
    pub line: Option<usize>,
}

/// A Tauri command reference (handler or invocation).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandRef {
    /// Command function name (Rust side).
    pub name: String,
    /// Exposed name if different (e.g., via `#[tauri::command(rename_all = ...)]`).
    pub exposed_name: Option<String>,
    /// 1-based line number.
    pub line: usize,
    /// Generic type parameter (e.g., `State<AppState>`).
    pub generic_type: Option<String>,
    /// Payload type/shape if detected.
    pub payload: Option<String>,
}

/// Casing inconsistency in command payload keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandPayloadCasing {
    /// Command name.
    pub command: String,
    /// Key with inconsistent casing.
    pub key: String,
    /// File path.
    pub path: String,
    /// 1-based line number.
    pub line: usize,
}

/// JS/TS string literal captured for dynamic/registry awareness
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StringLiteral {
    pub value: String,
    pub line: usize,
}

/// A Tauri event reference (emit or listen).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventRef {
    /// Original event name as written (may be const reference).
    pub raw_name: Option<String>,
    /// Resolved event name.
    pub name: String,
    /// 1-based line number.
    pub line: usize,
    /// "emit" or "listen".
    pub kind: String,
    /// True if awaited (`await emit(...)`).
    pub awaited: bool,
    /// Payload type/shape if detected.
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

/// Per-file analysis result.
///
/// Contains all extracted information from a single source file:
/// imports, exports, Tauri commands/events, and metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FileAnalysis {
    /// Relative path from project root.
    #[serde(default)]
    pub path: String,
    /// Lines of code (excluding blanks/comments).
    #[serde(default)]
    pub loc: usize,
    /// Detected language: "typescript", "javascript", "python", "rust", "css".
    #[serde(default)]
    pub language: String,
    /// File kind: "code", "test", "config", "style".
    #[serde(default)]
    pub kind: String,
    /// True if test file (based on path/name patterns).
    #[serde(default)]
    pub is_test: bool,
    /// True if generated file (has generation marker).
    #[serde(default)]
    pub is_generated: bool,
    /// Import statements found in file.
    #[serde(default)]
    pub imports: Vec<ImportEntry>,
    /// Re-export statements (`export { x } from './y'`).
    #[serde(default)]
    pub reexports: Vec<ReexportEntry>,
    /// Dynamic import paths (resolved where possible).
    #[serde(default)]
    pub dynamic_imports: Vec<String>,
    /// Exported symbols (functions, classes, consts, types).
    #[serde(default)]
    pub exports: Vec<ExportSymbol>,
    /// Tauri command invocations (frontend `invoke()`).
    #[serde(default)]
    pub command_calls: Vec<CommandRef>,
    /// Tauri command handlers (backend `#[tauri::command]`).
    #[serde(default)]
    pub command_handlers: Vec<CommandRef>,
    /// Detected casing inconsistencies in command payloads.
    #[serde(default)]
    pub command_payload_casing: Vec<CommandPayloadCasing>,
    /// String literals for dynamic/registry awareness.
    #[serde(default)]
    pub string_literals: Vec<StringLiteral>,
    /// Tauri event emissions.
    #[serde(default)]
    pub event_emits: Vec<EventRef>,
    /// Tauri event listeners.
    #[serde(default)]
    pub event_listens: Vec<EventRef>,
    /// Event name constants (`const EVENT_X = "event-x"`).
    #[serde(default)]
    pub event_consts: HashMap<String, String>,
    /// Symbol search matches.
    #[serde(default)]
    pub matches: Vec<SymbolMatch>,
    /// Detected entry points (main, index, App).
    #[serde(default)]
    pub entry_points: Vec<String>,
    /// Rust handlers registered via `tauri::generate_handler![...]`.
    #[serde(default)]
    pub tauri_registered_handlers: Vec<String>,
    /// File mtime (Unix timestamp) for incremental scanning.
    #[serde(default)]
    pub mtime: u64,
    /// File size in bytes for incremental cache validation.
    #[serde(default)]
    pub size: u64,
    /// Python concurrency race indicators.
    #[serde(default)]
    pub py_race_indicators: Vec<PyRaceIndicator>,
    /// Python: True if package has py.typed marker (PEP 561).
    #[serde(default)]
    pub is_typed_package: bool,
    /// Python: True if namespace package (PEP 420).
    #[serde(default)]
    pub is_namespace_package: bool,
    /// Locally-referenced symbols (for dead-code suppression).
    #[serde(default)]
    pub local_uses: Vec<String>,
    /// Type usages that appear in function signatures (parameters/returns).
    #[serde(default)]
    pub signature_uses: Vec<SignatureUse>,
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
            is_lazy: false,
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
            command_payload_casing: Vec::new(),
            string_literals: Vec::new(),
            event_emits: Vec::new(),
            event_listens: Vec::new(),
            event_consts: HashMap::new(),
            matches: Vec::new(),
            entry_points: Vec::new(),
            tauri_registered_handlers: Vec::new(),
            py_race_indicators: Vec::new(),
            mtime: 0,
            size: 0,
            is_typed_package: false,
            is_namespace_package: false,
            local_uses: Vec::new(),
            signature_uses: Vec::new(),
        }
    }
}

/// How a type is used in a function signature.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignatureUseKind {
    Parameter,
    Return,
}

/// A single mention of a type in a function signature.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignatureUse {
    /// Function or method name where the type appears.
    pub function: String,
    /// Kind of usage: parameter or return type.
    pub usage: SignatureUseKind,
    /// The referenced type name (as parsed).
    pub type_name: String,
    /// Line number for traceability.
    #[serde(default)]
    pub line: Option<usize>,
}

// Convenience type aliases reused across modules
pub type ExportIndex = HashMap<String, Vec<String>>;
pub type PayloadEntry = (String, usize, Option<String>);
pub type PayloadMap = HashMap<String, Vec<PayloadEntry>>;
