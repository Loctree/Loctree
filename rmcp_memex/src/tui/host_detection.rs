//! Host detection module for MCP server configurations.
//!
//! Scans known locations for MCP host configurations (Codex, Cursor, Claude Desktop, JetBrains).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostKind {
    Codex,
    Cursor,
    ClaudeDesktop,
    JetBrains,
    VSCode,
}

impl HostKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            HostKind::Codex => "Codex CLI",
            HostKind::Cursor => "Cursor",
            HostKind::ClaudeDesktop => "Claude Desktop",
            HostKind::JetBrains => "JetBrains IDEs",
            HostKind::VSCode => "VS Code",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostFormat {
    Toml,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HostDetection {
    pub kind: HostKind,
    pub path: PathBuf,
    pub format: HostFormat,
    pub exists: bool,
    pub has_rmcp_memex: bool,
    pub servers: Vec<McpServerEntry>,
}

impl HostDetection {
    pub fn status_icon(&self) -> &'static str {
        if !self.exists {
            "○"
        } else if self.has_rmcp_memex {
            "●"
        } else {
            "◐"
        }
    }

    pub fn status_text(&self) -> &'static str {
        if !self.exists {
            "Not found"
        } else if self.has_rmcp_memex {
            "Configured"
        } else {
            "Detected (no rmcp_memex)"
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn get_host_config_path(kind: HostKind) -> Option<(PathBuf, HostFormat)> {
    let home = home_dir()?;

    match kind {
        HostKind::Codex => Some((home.join(".codex/config.toml"), HostFormat::Toml)),
        HostKind::Cursor => {
            #[cfg(target_os = "macos")]
            let path = home.join(
                "Library/Application Support/Cursor/User/globalStorage/cursor.mcp/config.json",
            );
            #[cfg(target_os = "linux")]
            let path = home.join(".config/Cursor/User/globalStorage/cursor.mcp/config.json");
            #[cfg(target_os = "windows")]
            let path =
                home.join("AppData/Roaming/Cursor/User/globalStorage/cursor.mcp/config.json");
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            let path = home.join(".config/Cursor/config.json");
            Some((path, HostFormat::Json))
        }
        HostKind::ClaudeDesktop => {
            #[cfg(target_os = "macos")]
            let path = home.join("Library/Application Support/Claude/claude_desktop_config.json");
            #[cfg(target_os = "linux")]
            let path = home.join(".config/Claude/claude_desktop_config.json");
            #[cfg(target_os = "windows")]
            let path = home.join("AppData/Roaming/Claude/claude_desktop_config.json");
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            let path = home.join(".config/Claude/claude_desktop_config.json");
            Some((path, HostFormat::Json))
        }
        HostKind::JetBrains => {
            // JetBrains uses a common MCP config location
            #[cfg(target_os = "macos")]
            let path = home.join("Library/Application Support/JetBrains/mcp.json");
            #[cfg(target_os = "linux")]
            let path = home.join(".config/JetBrains/mcp.json");
            #[cfg(target_os = "windows")]
            let path = home.join("AppData/Roaming/JetBrains/mcp.json");
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            let path = home.join(".config/JetBrains/mcp.json");
            Some((path, HostFormat::Json))
        }
        HostKind::VSCode => {
            #[cfg(target_os = "macos")]
            let path = home.join("Library/Application Support/Code/User/globalStorage/anthropic.claude-vscode/settings/cline_mcp_settings.json");
            #[cfg(target_os = "linux")]
            let path = home.join(".config/Code/User/globalStorage/anthropic.claude-vscode/settings/cline_mcp_settings.json");
            #[cfg(target_os = "windows")]
            let path = home.join("AppData/Roaming/Code/User/globalStorage/anthropic.claude-vscode/settings/cline_mcp_settings.json");
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            let path = home.join(".config/Code/cline_mcp_settings.json");
            Some((path, HostFormat::Json))
        }
    }
}

fn parse_toml_mcp_servers(content: &str) -> Vec<McpServerEntry> {
    let mut servers = Vec::new();

    if let Ok(value) = content.parse::<toml::Value>()
        && let Some(mcp_servers) = value.get("mcp_servers").and_then(|v| v.as_table())
    {
        for (name, config) in mcp_servers {
            let command = config
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let args = config
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let env = config
                .get("env")
                .and_then(|v| v.as_table())
                .map(|t| {
                    t.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            servers.push(McpServerEntry {
                name: name.clone(),
                command,
                args,
                env,
            });
        }
    }

    servers
}

fn parse_json_mcp_servers(content: &str) -> Vec<McpServerEntry> {
    let mut servers = Vec::new();

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
        let mcp_servers = value.get("mcpServers").or_else(|| value.get("mcp_servers"));

        if let Some(mcp_obj) = mcp_servers.and_then(|v| v.as_object()) {
            for (name, config) in mcp_obj {
                let command = config
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let args = config
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let env = config
                    .get("env")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default();

                servers.push(McpServerEntry {
                    name: name.clone(),
                    command,
                    args,
                    env,
                });
            }
        }
    }

    servers
}

fn detect_single_host(kind: HostKind) -> Option<HostDetection> {
    let (path, format) = get_host_config_path(kind)?;
    let exists = path.exists();

    let (has_rmcp_memex, servers) = if exists {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let servers = match format {
                HostFormat::Toml => parse_toml_mcp_servers(&content),
                HostFormat::Json => parse_json_mcp_servers(&content),
            };
            let has_rmcp = servers
                .iter()
                .any(|s| s.name.contains("rmcp_memex") || s.command.contains("rmcp_memex"));
            (has_rmcp, servers)
        } else {
            (false, Vec::new())
        }
    } else {
        (false, Vec::new())
    };

    Some(HostDetection {
        kind,
        path,
        format,
        exists,
        has_rmcp_memex,
        servers,
    })
}

/// Detect all known MCP host configurations.
pub fn detect_hosts() -> Vec<HostDetection> {
    let kinds = [
        HostKind::Codex,
        HostKind::Cursor,
        HostKind::ClaudeDesktop,
        HostKind::JetBrains,
        HostKind::VSCode,
    ];

    kinds
        .iter()
        .filter_map(|&k| detect_single_host(k))
        .collect()
}

/// Generate a config snippet for a specific host.
pub fn generate_snippet(kind: HostKind, binary_path: &str, db_path: &str) -> String {
    match get_host_config_path(kind) {
        Some((_, HostFormat::Toml)) => {
            format!(
                r#"[mcp_servers.rmcp_memex]
command = "{}"
args = ["--db-path", "{}", "--log-level", "info"]
"#,
                binary_path, db_path
            )
        }
        Some((_, HostFormat::Json)) => {
            format!(
                r#"{{
  "mcpServers": {{
    "rmcp_memex": {{
      "command": "{}",
      "args": ["--db-path", "{}", "--log-level", "info"]
    }}
  }}
}}"#,
                binary_path, db_path
            )
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml_mcp_servers() {
        let toml_content = r#"
[mcp_servers.rmcp_memex]
command = "/usr/local/bin/rmcp_memex"
args = ["--db-path", "~/.rmcp/db"]

[mcp_servers.other_server]
command = "other"
"#;
        let servers = parse_toml_mcp_servers(toml_content);
        assert_eq!(servers.len(), 2);
        assert!(servers.iter().any(|s| s.name == "rmcp_memex"));
    }

    #[test]
    fn test_parse_json_mcp_servers() {
        let json_content = r#"{
  "mcpServers": {
    "rmcp_memex": {
      "command": "/usr/local/bin/rmcp_memex",
      "args": ["--db-path", "~/.rmcp/db"]
    }
  }
}"#;
        let servers = parse_json_mcp_servers(json_content);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "rmcp_memex");
    }

    #[test]
    fn test_generate_toml_snippet() {
        let snippet = generate_snippet(HostKind::Codex, "/usr/bin/rmcp_memex", "~/.rmcp/db");
        assert!(snippet.contains("[mcp_servers.rmcp_memex]"));
        assert!(snippet.contains("/usr/bin/rmcp_memex"));
    }

    #[test]
    fn test_generate_json_snippet() {
        let snippet =
            generate_snippet(HostKind::ClaudeDesktop, "/usr/bin/rmcp_memex", "~/.rmcp/db");
        assert!(snippet.contains("\"mcpServers\""));
        assert!(snippet.contains("\"rmcp_memex\""));
    }
}
