use crate::types::{FileAnalysis, ImportEntry, ImportKind};

use super::regexes::regex_css_import;

pub(crate) fn analyze_css_file(content: &str, relative: String) -> FileAnalysis {
    let mut analysis = FileAnalysis::new(relative);
    for caps in regex_css_import().captures_iter(content) {
        let source = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        analysis
            .imports
            .push(ImportEntry::new(source, ImportKind::Static));
    }

    analysis
}
