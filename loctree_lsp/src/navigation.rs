//! Go-to-definition navigation utilities for loctree LSP
//!
//! Provides helper functions for implementing `textDocument/definition`:
//! - Word extraction at cursor position
//! - Converting definition locations to LSP response format
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};

use crate::snapshot::DefinitionLocation;

/// Extract word at cursor position from document text
///
/// Returns the identifier under the cursor, handling common identifier characters.
pub fn get_word_at_position(text: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let line_idx = position.line as usize;

    if line_idx >= lines.len() {
        return None;
    }

    let line = lines[line_idx];
    let char_idx = position.character as usize;

    if char_idx > line.len() {
        return None;
    }

    // Find word boundaries
    let chars: Vec<char> = line.chars().collect();

    // Check if cursor is on an identifier character
    // If not (e.g., on whitespace or punctuation), return None
    if !is_identifier_char(chars.get(char_idx).copied()) {
        return None;
    }

    // Find start of word
    let mut start = char_idx;
    while start > 0 && is_identifier_char(chars.get(start - 1).copied()) {
        start -= 1;
    }

    // Find end of word
    let mut end = char_idx;
    while end < chars.len() && is_identifier_char(chars.get(end).copied()) {
        end += 1;
    }

    if start == end {
        return None;
    }

    let word: String = chars[start..end].iter().collect();
    if word.is_empty() { None } else { Some(word) }
}

/// Check if a character is valid in an identifier
fn is_identifier_char(c: Option<char>) -> bool {
    match c {
        Some(ch) => ch.is_alphanumeric() || ch == '_' || ch == '$',
        None => false,
    }
}

/// Convert a DefinitionLocation to LSP GotoDefinitionResponse
///
/// # Arguments
/// * `def` - The definition location from snapshot lookup
/// * `workspace_root` - The workspace root path for constructing absolute URIs
pub fn definition_to_response(
    def: &DefinitionLocation,
    workspace_root: &str,
) -> Option<GotoDefinitionResponse> {
    // Construct full file path
    let full_path = if def.file.starts_with('/') {
        def.file.clone()
    } else {
        format!("{}/{}", workspace_root.trim_end_matches('/'), def.file)
    };

    // Create file URI
    let uri = Url::from_file_path(&full_path).ok()?;

    // Line is 1-based in snapshot, LSP uses 0-based
    let line = if def.line > 0 { def.line - 1 } else { 0 };

    let location = Location {
        uri,
        range: Range {
            start: Position {
                line: line as u32,
                character: 0,
            },
            end: Position {
                line: line as u32,
                character: 0,
            },
        },
    };

    Some(GotoDefinitionResponse::Scalar(location))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_word_at_position_simple() {
        let text = "import { foo } from './bar'";
        let pos = Position {
            line: 0,
            character: 9,
        }; // on 'foo'
        assert_eq!(get_word_at_position(text, pos), Some("foo".to_string()));
    }

    #[test]
    fn test_get_word_at_position_start_of_word() {
        let text = "const myVariable = 42";
        let pos = Position {
            line: 0,
            character: 6,
        }; // at 'm' of myVariable
        assert_eq!(
            get_word_at_position(text, pos),
            Some("myVariable".to_string())
        );
    }

    #[test]
    fn test_get_word_at_position_end_of_word() {
        let text = "const myVariable = 42";
        let pos = Position {
            line: 0,
            character: 15,
        }; // at 'e' of myVariable
        assert_eq!(
            get_word_at_position(text, pos),
            Some("myVariable".to_string())
        );
    }

    #[test]
    fn test_get_word_at_position_on_space() {
        let text = "const foo = bar";
        let pos = Position {
            line: 0,
            character: 5,
        }; // on space
        assert_eq!(get_word_at_position(text, pos), None);
    }

    #[test]
    fn test_get_word_at_position_multiline() {
        let text = "first line\nsecond line";
        let pos = Position {
            line: 1,
            character: 0,
        }; // at 's' of second
        assert_eq!(get_word_at_position(text, pos), Some("second".to_string()));
    }

    #[test]
    fn test_get_word_with_underscore() {
        let text = "const my_var = 42";
        let pos = Position {
            line: 0,
            character: 8,
        }; // in 'my_var'
        assert_eq!(get_word_at_position(text, pos), Some("my_var".to_string()));
    }

    #[test]
    fn test_get_word_with_dollar() {
        let text = "const $element = document";
        let pos = Position {
            line: 0,
            character: 6,
        }; // on '$element'
        assert_eq!(
            get_word_at_position(text, pos),
            Some("$element".to_string())
        );
    }
}
