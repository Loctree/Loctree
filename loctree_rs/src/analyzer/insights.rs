use super::{AiInsight, CommandGap, RankedDup};
use crate::types::FileAnalysis;

pub fn collect_ai_insights(
    files: &[FileAnalysis],
    dups: &[RankedDup],
    cascades: &[(String, String)],
    gap_missing: &[CommandGap],
    _gap_unused: &[CommandGap],
) -> Vec<AiInsight> {
    let mut insights = Vec::new();

    let huge_files: Vec<_> = files.iter().filter(|f| f.loc > 2000).collect();
    if !huge_files.is_empty() {
        insights.push(AiInsight {
            title: "Huge files detected".to_string(),
            severity: "medium".to_string(),
            message: format!(
                "Found {} files with > 2000 LOC (e.g. {}). Consider splitting them.",
                huge_files.len(),
                huge_files[0].path
            ),
        });
    }

    if dups.len() > 10 {
        insights.push(AiInsight {
            title: "High number of duplicate exports".to_string(),
            severity: "medium".to_string(),
            message: format!(
                "Found {} duplicate export groups. Consider refactoring.",
                dups.len()
            ),
        });
    }

    if cascades.len() > 20 {
        insights.push(AiInsight {
            title: "Many re-export chains".to_string(),
            severity: "low".to_string(),
            message: format!(
                "Found {} re-export cascades. This might affect tree-shaking/bundling.",
                cascades.len()
            ),
        });
    }

    if !gap_missing.is_empty() {
        insights.push(AiInsight {
            title: "Missing Tauri Handlers".to_string(),
            severity: "high".to_string(),
            message: format!(
                "Frontend calls {} commands that are missing in Backend.",
                gap_missing.len()
            ),
        });
    }

    insights
}
