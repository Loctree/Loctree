pub mod assets;
mod css;
pub mod html;
pub mod js;
pub mod open_server;
pub mod py;
pub mod regexes;
pub mod report;
pub mod resolvers;
pub mod runner;
pub mod rust;

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

pub use report::{CommandGap, GraphData, GraphNode, RankedDup, ReportSection};
pub use runner::run_import_analyzer;
