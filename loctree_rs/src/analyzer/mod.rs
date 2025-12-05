//! Static analysis engine for loctree.
//!
//! This module contains language-specific analyzers and cross-cutting analysis features:
//!
//! ## Language Analyzers
//! - [`ast_js`] / [`js`] - TypeScript/JavaScript AST analysis
//! - [`py`] - Python import/export analysis
//! - [`rust`] - Rust analysis (Tauri commands, mod statements)
//! - [`css`] - CSS/SCSS dependency tracking
//!
//! ## Analysis Features
//! - [`cycles`] - Circular import detection (Tarjan's SCC algorithm)
//! - [`dead_parrots`] - Dead/unused export detection
//! - [`coverage`] - Tauri command coverage (FE→BE matching)
//! - [`trace`] - Handler tracing through the call graph
//! - [`pipelines`] - Data flow pipeline analysis
//!
//! ## Output Formats
//! - [`for_ai`] - AI-optimized JSON with quick wins
//! - [`html`] - Interactive HTML reports
//! - [`sarif`] - SARIF 2.1.0 for CI integration
//! - [`report`] - Report data structures

pub mod assets;
pub mod ast_js;
pub mod classify;
pub mod coverage;
mod css;
pub mod cycles;
pub mod dead_parrots;
pub mod entrypoints;
pub mod for_ai;
mod graph;
pub mod html;
mod insights;
pub mod js;
pub mod open_server;
pub mod output;
pub mod pipelines;
pub mod py;
pub mod regexes;
pub mod report;
pub mod resolvers;
pub mod root_scan;
pub mod runner;
pub mod rust;
pub mod sarif;
pub mod scan;
pub mod search;
pub mod trace;
mod tsconfig;

pub(super) fn offset_to_line(content: &str, offset: usize) -> usize {
    content[..offset].bytes().filter(|b| *b == b'\n').count() + 1
}

/// Build an open URL for IDE integration
/// Format: loctree://open?f={file}&l={line}
/// Or if open_base is provided (e.g., "http://127.0.0.1:7777"):
/// Format: {open_base}/open?f={file}&l={line}
pub fn build_open_url(file: &str, line: Option<usize>, open_base: Option<&str>) -> String {
    let base = open_base.unwrap_or("loctree://");
    let path = if base.ends_with('/') {
        format!("{}open", base)
    } else if base.contains("://") {
        format!("{}/open", base)
    } else {
        format!("{}://open", base)
    };

    match line {
        Some(l) => format!("{}?f={}&l={}", path, urlencoding::encode(file), l),
        None => format!("{}?f={}", path, urlencoding::encode(file)),
    }
}

#[allow(unused_imports)]
pub use report::{
    AiInsight, CommandGap, DupLocation, DupSeverity, GraphComponent, GraphData, GraphNode,
    RankedDup, ReportSection,
};
pub use runner::run_import_analyzer;
