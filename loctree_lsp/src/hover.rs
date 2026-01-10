//! Hover provider for loctree LSP
//!
//! Provides rich hover information for exports and imports using snapshot data.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use std::collections::HashMap;

use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use crate::snapshot::SnapshotState;

/// Information about an export for hover display
#[derive(Debug, Clone)]
pub struct ExportHoverInfo {
    /// Symbol name
    pub symbol: String,
    /// File where the symbol is exported
    pub file: String,
    /// Number of files that import this export
    pub import_count: usize,
    /// List of consumer files (top N)
    pub top_consumers: Vec<String>,
    /// Export kind (function, class, const, type, etc.)
    pub kind: String,
    /// Line number of the export
    pub line: Option<usize>,
}

/// Information about an import for hover display
#[derive(Debug, Clone)]
pub struct ImportHoverInfo {
    /// Symbol name being imported
    pub symbol: String,
    /// Source file where the symbol is defined
    pub source_file: String,
    /// Line number where the symbol is defined
    pub source_line: Option<usize>,
    /// Kind of the imported symbol
    pub kind: String,
}

impl SnapshotState {
    /// Find export info at a given position in a file
    ///
    /// Returns hover info if the position is on an export symbol
    pub async fn find_export_at_position(
        &self,
        file_path: &str,
        position: Position,
    ) -> Option<ExportHoverInfo> {
        let guard = self.get().await?;
        let loaded = guard.as_ref()?;
        let snapshot = &loaded.snapshot;

        // Find the file in the snapshot
        let file = snapshot
            .files
            .iter()
            .find(|f| f.path.ends_with(file_path) || file_path.ends_with(&f.path))?;

        // Find export at the given line (1-based line numbers in LSP)
        let target_line = position.line as usize + 1;
        let export = file
            .exports
            .iter()
            .find(|e| e.line.map(|l| l == target_line).unwrap_or(false))?;

        // Count imports for this export by analyzing edges
        let (import_count, top_consumers) = self.count_imports_for_file(snapshot, &file.path);

        Some(ExportHoverInfo {
            symbol: export.name.clone(),
            file: file.path.clone(),
            import_count,
            top_consumers,
            kind: export.kind.clone(),
            line: export.line,
        })
    }

    /// Find import info at a given position in a file
    ///
    /// Returns hover info if the position is on an import statement
    pub async fn find_import_at_position(
        &self,
        file_path: &str,
        _position: Position,
    ) -> Option<ImportHoverInfo> {
        let guard = self.get().await?;
        let loaded = guard.as_ref()?;
        let snapshot = &loaded.snapshot;

        // Find the file in the snapshot
        let file = snapshot
            .files
            .iter()
            .find(|f| f.path.ends_with(file_path) || file_path.ends_with(&f.path))?;

        // Check if the position is on an import line
        // We'll look at edges from this file
        for edge in &snapshot.edges {
            if edge.from.ends_with(&file.path) || file.path.ends_with(&edge.from) {
                // Find the target file to get export info
                let target_file = snapshot.files.iter().find(|f| {
                    f.path == edge.to || edge.to.ends_with(&f.path) || f.path.ends_with(&edge.to)
                })?;

                // Look for matching export in target
                let export = target_file.exports.iter().find(|e| e.name == edge.label)?;

                return Some(ImportHoverInfo {
                    symbol: edge.label.clone(),
                    source_file: target_file.path.clone(),
                    source_line: export.line,
                    kind: export.kind.clone(),
                });
            }
        }

        None
    }

    /// Count how many files import a given file and return top consumers
    fn count_imports_for_file(
        &self,
        snapshot: &loctree::snapshot::Snapshot,
        file_path: &str,
    ) -> (usize, Vec<String>) {
        let mut consumers: HashMap<String, usize> = HashMap::new();

        for edge in &snapshot.edges {
            if edge.to == file_path || edge.to.ends_with(file_path) || file_path.ends_with(&edge.to)
            {
                *consumers.entry(edge.from.clone()).or_insert(0) += 1;
            }
        }

        let import_count = consumers.len();

        // Sort by import count and take top 5
        let mut sorted: Vec<_> = consumers.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        let top_consumers: Vec<String> = sorted.into_iter().take(5).map(|(f, _)| f).collect();

        (import_count, top_consumers)
    }

    /// Get hover info for any symbol at position
    ///
    /// Checks exports first, then imports
    pub async fn get_hover_info(&self, file_path: &str, position: Position) -> Option<Hover> {
        // Try export first
        if let Some(export_info) = self.find_export_at_position(file_path, position).await {
            return Some(format_export_hover(&export_info));
        }

        // Try import
        if let Some(import_info) = self.find_import_at_position(file_path, position).await {
            return Some(format_import_hover(&import_info));
        }

        None
    }
}

/// Format export info as Markdown hover content
fn format_export_hover(info: &ExportHoverInfo) -> Hover {
    let mut lines = vec![format!("**Export: `{}`**", info.symbol)];

    if !info.kind.is_empty() {
        lines.push(format!("- Kind: {}", info.kind));
    }

    if info.import_count > 0 {
        let file_word = if info.import_count == 1 {
            "file"
        } else {
            "files"
        };
        lines.push(format!(
            "- {} imports across {} {}",
            info.import_count, info.import_count, file_word
        ));

        if !info.top_consumers.is_empty() {
            let consumers: Vec<String> = info
                .top_consumers
                .iter()
                .map(|f| format!("`{}`", shorten_path(f)))
                .collect();
            lines.push(format!("- Top consumers: {}", consumers.join(", ")));
        }
    } else {
        lines.push("- No imports found (potentially dead code)".to_string());
    }

    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: lines.join("\n"),
        }),
        range: None,
    }
}

/// Format import info as Markdown hover content
fn format_import_hover(info: &ImportHoverInfo) -> Hover {
    let mut lines = vec![format!("**Import: `{}`**", info.symbol)];

    lines.push(format!(
        "- Defined in: `{}`",
        shorten_path(&info.source_file)
    ));

    if let Some(line) = info.source_line {
        lines.push(format!("- Line: {}", line));
    }

    if !info.kind.is_empty() {
        lines.push(format!("- Kind: {}", info.kind));
    }

    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: lines.join("\n"),
        }),
        range: None,
    }
}

/// Shorten a file path for display (show last 2-3 segments)
fn shorten_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 3 {
        path.to_string()
    } else {
        parts[parts.len() - 3..].join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorten_path_short() {
        assert_eq!(shorten_path("src/main.ts"), "src/main.ts");
        assert_eq!(shorten_path("a/b/c"), "a/b/c");
    }

    #[test]
    fn test_shorten_path_long() {
        assert_eq!(
            shorten_path("project/src/components/Button.tsx"),
            "src/components/Button.tsx"
        );
    }

    #[test]
    fn test_format_export_hover_with_imports() {
        let info = ExportHoverInfo {
            symbol: "Button".to_string(),
            file: "src/components/Button.tsx".to_string(),
            import_count: 5,
            top_consumers: vec!["App.tsx".to_string(), "Page.tsx".to_string()],
            kind: "function".to_string(),
            line: Some(10),
        };

        let hover = format_export_hover(&info);
        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("**Export: `Button`**"));
            assert!(content.value.contains("5 imports"));
            assert!(content.value.contains("`App.tsx`"));
        } else {
            panic!("Expected Markup content");
        }
    }

    #[test]
    fn test_format_export_hover_no_imports() {
        let info = ExportHoverInfo {
            symbol: "unused".to_string(),
            file: "src/unused.ts".to_string(),
            import_count: 0,
            top_consumers: vec![],
            kind: "const".to_string(),
            line: Some(1),
        };

        let hover = format_export_hover(&info);
        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("No imports found"));
            assert!(content.value.contains("dead code"));
        } else {
            panic!("Expected Markup content");
        }
    }

    #[test]
    fn test_format_import_hover() {
        let info = ImportHoverInfo {
            symbol: "useState".to_string(),
            source_file: "node_modules/react/index.js".to_string(),
            source_line: Some(42),
            kind: "function".to_string(),
        };

        let hover = format_import_hover(&info);
        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("**Import: `useState`**"));
            assert!(content.value.contains("react/index.js"));
            assert!(content.value.contains("Line: 42"));
        } else {
            panic!("Expected Markup content");
        }
    }
}
