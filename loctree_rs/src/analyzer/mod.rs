pub mod assets;
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

pub(super) fn brace_list_to_names(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|item| {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Some((_, alias)) = trimmed.split_once(" as ") {
                Some(alias.trim().to_string())
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

pub(super) fn offset_to_line(content: &str, offset: usize) -> usize {
    content[..offset].bytes().filter(|b| *b == b'\n').count() + 1
}

#[allow(unused_imports)]
pub use report::{
    AiInsight, CommandGap, GraphComponent, GraphData, GraphNode, RankedDup, ReportSection,
};
pub use runner::run_import_analyzer;
