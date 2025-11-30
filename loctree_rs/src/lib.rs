//! # loctree
//!
//! **AI-oriented Project Analyzer** - Static analysis tool designed for AI agents
//! and developers building production-ready software.
//!
//! loctree helps overcome the common AI tendency to generate excessive artifacts
//! that lead to re-export cascades, circular imports, and spaghetti dependencies.
//!
//! ## Features
//!
//! - **Holographic Slice** - Extract focused context (deps + consumers) for any file
//! - **Handler Trace** - Follow Tauri commands through the entire pipeline
//! - **Dead Export Detection** - Find unused exports and orphaned code
//! - **Circular Import Detection** - Catch runtime bombs before they explode
//! - **Auto-Detect Stack** - Automatically configure for Rust, TypeScript, Python, Tauri
//! - **HTML Reports** - Interactive reports with Cytoscape.js dependency graphs
//!
//! ## Quick Start (Library Usage)
//!
//! ```rust,no_run
//! use loctree::{detect, snapshot, slicer};
//! use std::path::PathBuf;
//!
//! // Detect project stack
//! let detected = detect::detect_stack(std::path::Path::new("."));
//! println!("Detected: {}", detected.description);
//! ```
//!
//! ## Running Import Analysis
//!
//! ```rust,no_run
//! use loctree::{analyzer, args};
//! use std::path::PathBuf;
//!
//! // Run the full import analyzer on a project
//! let mut parsed = args::ParsedArgs::default();
//! parsed.dead_exports = true;
//! parsed.circular = true;
//!
//! let roots = vec![PathBuf::from(".")];
//! analyzer::run_import_analyzer(&roots, &parsed).unwrap();
//! ```
//!
//! ## CLI Usage
//!
//! For command-line usage, install with `cargo install loctree` and run:
//!
//! ```bash
//! loctree                    # Scan and snapshot
//! loctree slice src/App.tsx  # Extract AI context
//! loctree trace get_user     # Trace Tauri handler
//! loctree -A --circular      # Find circular imports
//! loctree --for-ai           # AI-optimized JSON output
//! ```
//!
//! See the [README](https://github.com/LibraxisAI/Loctree) for full documentation.

#![doc(html_root_url = "https://docs.rs/loctree/0.5.2")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/LibraxisAI/loctree/main/assets/loctree-logo.svg"
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/LibraxisAI/loctree/main/assets/loctree-logo.svg"
)]

// ============================================================================
// Core Modules
// ============================================================================

/// Import/export analyzer supporting TypeScript, JavaScript, Python, Rust, and CSS.
///
/// # Submodules
///
/// - [`analyzer::js`] - TypeScript/JavaScript analysis
/// - [`analyzer::py`] - Python analysis
/// - [`analyzer::rust`] - Rust analysis (Tauri commands)
/// - [`analyzer::cycles`] - Circular import detection (Tarjan's SCC)
/// - [`analyzer::dead_parrots`] - Dead export detection
/// - [`analyzer::trace`] - Handler tracing for Tauri
/// - [`analyzer::coverage`] - Tauri command coverage
/// - [`analyzer::for_ai`] - AI-optimized output generation
/// - [`analyzer::html`] - HTML report generation
/// - [`analyzer::sarif`] - SARIF 2.1.0 output for CI
pub mod analyzer;

/// Command-line argument parsing.
///
/// Contains [`ParsedArgs`](args::ParsedArgs) struct and [`parse_args`](args::parse_args) function.
pub mod args;

/// Auto-detection of project stacks.
///
/// Detects Rust, TypeScript, Python, Tauri, Vite, and more based on marker files.
///
/// # Example
///
/// ```rust,no_run
/// use loctree::detect;
/// use std::path::Path;
///
/// let detected = detect::detect_stack(Path::new("."));
/// if !detected.is_empty() {
///     println!("Stack: {}", detected.description);
///     println!("Extensions: {:?}", detected.extensions);
/// }
/// ```
pub mod detect;

/// Filesystem utilities.
///
/// - Gitignore handling with [`GitIgnoreChecker`](fs_utils::GitIgnoreChecker)
/// - File gathering with extension/depth filters
/// - Line counting
/// - Pattern normalization
pub mod fs_utils;

/// String similarity using Levenshtein distance.
///
/// Used for fuzzy matching in `--check` mode to find similar component names.
pub mod similarity;

/// Holographic slice extraction.
///
/// Extracts a file's context in three layers:
/// - **Core** - The target file itself
/// - **Deps** - Files the target imports (transitive)
/// - **Consumers** - Files that import the target
///
/// # Example
///
/// ```rust,no_run
/// use loctree::slicer;
/// use loctree::args::ParsedArgs;
/// use std::path::Path;
///
/// let parsed = ParsedArgs::default();
/// let root = Path::new(".");
///
/// // Extract slice for src/App.tsx with consumers, as JSON
/// slicer::run_slice(root, "src/App.tsx", true, true, &parsed).unwrap();
/// ```
pub mod slicer;

/// Incremental snapshot persistence.
///
/// Saves analysis results to `.loctree/snapshot.json` for faster subsequent runs.
/// Uses file modification times to skip unchanged files.
///
/// # Key Types
///
/// - [`Snapshot`](snapshot::Snapshot) - The persisted analysis state
/// - [`SnapshotMetadata`](snapshot::SnapshotMetadata) - Version and timestamp info
/// - [`GraphEdge`](snapshot::GraphEdge) - Import relationship
/// - [`CommandBridge`](snapshot::CommandBridge) - FE→BE command mapping
pub mod snapshot;

/// Directory tree with LOC counts.
///
/// Fast tree view similar to Unix `tree` command but with:
/// - Line counts per file
/// - Large file highlighting
/// - Gitignore support
/// - Build artifact detection (`--find-artifacts`)
pub mod tree;

/// Common types used throughout the crate.
///
/// # Key Types
///
/// - [`Mode`] - CLI mode (Tree, Slice, Trace, AnalyzeImports, ForAi)
/// - [`Options`] - Analysis configuration
/// - [`FileAnalysis`] - Per-file analysis result
/// - `ImportEntry` - Import statement representation
/// - `ExportSymbol` - Export declaration
/// - `CommandRef` - Tauri command reference
pub mod types;

// ============================================================================
// Re-exports for convenience
// ============================================================================

/// CLI modes.
pub use types::Mode;

/// Analysis options.
pub use types::Options;

/// Output format (Text, Json, Jsonl).
pub use types::OutputMode;

/// Color mode (Auto, Always, Never).
pub use types::ColorMode;

/// Per-file analysis result with imports, exports, commands, etc.
pub use types::FileAnalysis;

/// Detected project stack with extensions and ignores.
pub use detect::DetectedStack;

/// Main stack detection function.
pub use detect::detect_stack;

/// Holographic slice result.
pub use slicer::HolographicSlice;

/// Slice configuration.
pub use slicer::SliceConfig;

/// Persisted analysis state.
pub use snapshot::Snapshot;

/// Run the import analyzer.
pub use analyzer::run_import_analyzer;

/// Report section for HTML output.
pub use analyzer::ReportSection;

/// Command gap (missing/unused handler).
pub use analyzer::CommandGap;

/// Ranked duplicate export.
pub use analyzer::RankedDup;
