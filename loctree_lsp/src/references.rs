//! Find references implementation for loctree LSP
//!
//! Provides "find all references" functionality by analyzing snapshot edges
//! to find all files that import a given symbol.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::snapshot::ReferenceInfo;

/// Convert references to LSP Location objects
pub fn references_to_locations(
    references: &[ReferenceInfo],
    workspace_root: Option<&str>,
) -> Vec<Location> {
    references
        .iter()
        .filter_map(|r| {
            // Build full path
            let full_path = if let Some(root) = workspace_root {
                format!("{}/{}", root.trim_end_matches('/'), &r.file)
            } else {
                r.file.clone()
            };

            // Create URI
            let uri = Url::from_file_path(&full_path).ok()?;

            // Create range (line is 0-indexed in LSP)
            let line = if r.line > 0 { r.line - 1 } else { 0 } as u32;
            let range = Range::new(Position::new(line, 0), Position::new(line, 1000));

            Some(Location { uri, range })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_references_to_locations_empty() {
        let refs: Vec<ReferenceInfo> = vec![];
        let locs = references_to_locations(&refs, Some("/workspace"));
        assert!(locs.is_empty());
    }

    #[test]
    fn test_references_to_locations_with_root() {
        let refs = vec![ReferenceInfo {
            file: "src/main.rs".to_string(),
            line: 10,
        }];
        let locs = references_to_locations(&refs, Some("/workspace"));
        assert_eq!(locs.len(), 1);
        assert!(locs[0].uri.path().ends_with("/workspace/src/main.rs"));
        assert_eq!(locs[0].range.start.line, 9); // 1-indexed to 0-indexed
    }

    #[test]
    fn test_references_to_locations_line_zero() {
        let refs = vec![ReferenceInfo {
            file: "src/lib.rs".to_string(),
            line: 0,
        }];
        let locs = references_to_locations(&refs, Some("/workspace"));
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.line, 0); // line 0 stays 0
    }

    #[test]
    fn test_references_to_locations_without_root() {
        let refs = vec![ReferenceInfo {
            file: "/absolute/path/src/main.rs".to_string(),
            line: 5,
        }];
        let locs = references_to_locations(&refs, None);
        assert_eq!(locs.len(), 1);
        assert!(locs[0].uri.path().ends_with("/absolute/path/src/main.rs"));
    }
}
