//! Host detection types for MCP client configurations.
//!
//! These types represent different MCP host applications (Codex, Cursor, Claude Desktop, etc.)
//! and their configuration file formats.

use serde::{Deserialize, Serialize};

/// Supported MCP host application kinds.
///
/// Each variant represents a different MCP client application that can be configured
/// to use rmcp-mux or rmcp-memex servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostKind {
    /// Codex CLI (uses TOML config)
    Codex,
    /// Cursor editor (uses JSON config)
    Cursor,
    /// VS Code with MCP extension (uses JSON config)
    VSCode,
    /// Claude Desktop application (uses JSON config)
    Claude,
    /// JetBrains IDEs with MCP plugin (uses JSON config)
    JetBrains,
    /// Unknown or custom host
    Unknown,
}

impl HostKind {
    /// Returns a lowercase label for the host kind.
    ///
    /// Useful for CLI arguments, file naming, and configuration keys.
    pub fn as_label(&self) -> &'static str {
        match self {
            HostKind::Codex => "codex",
            HostKind::Cursor => "cursor",
            HostKind::VSCode => "vscode",
            HostKind::Claude => "claude",
            HostKind::JetBrains => "jetbrains",
            HostKind::Unknown => "unknown",
        }
    }

    /// Returns a human-readable display name for the host kind.
    pub fn display_name(&self) -> &'static str {
        match self {
            HostKind::Codex => "Codex CLI",
            HostKind::Cursor => "Cursor",
            HostKind::VSCode => "VS Code",
            HostKind::Claude => "Claude Desktop",
            HostKind::JetBrains => "JetBrains IDEs",
            HostKind::Unknown => "Unknown",
        }
    }
}

/// Configuration file format for MCP hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostFormat {
    /// TOML format (used by Codex)
    Toml,
    /// JSON format (used by most other hosts)
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_kind_labels() {
        assert_eq!(HostKind::Codex.as_label(), "codex");
        assert_eq!(HostKind::Claude.as_label(), "claude");
        assert_eq!(HostKind::Unknown.as_label(), "unknown");
    }

    #[test]
    fn host_kind_display_names() {
        assert_eq!(HostKind::Codex.display_name(), "Codex CLI");
        assert_eq!(HostKind::Claude.display_name(), "Claude Desktop");
    }

    #[test]
    fn host_kind_serde_roundtrip() {
        let kind = HostKind::Cursor;
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: HostKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, parsed);
    }

    #[test]
    fn host_format_serde_roundtrip() {
        let format = HostFormat::Json;
        let json = serde_json::to_string(&format).unwrap();
        let parsed: HostFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(format, parsed);
    }
}
