//! Client (host application) detection logic.

use std::path::PathBuf;

use crate::config::expand_path;
use crate::scan::{HostFormat, HostKind, discover_hosts, scan_host_file};

use super::types::ClientEntry;

/// Known client application paths for detection
/// Returns: (HostKind, HostFormat, config_path, app_indicator_paths)
fn get_known_clients() -> Vec<(HostKind, HostFormat, PathBuf, Vec<PathBuf>)> {
    vec![
        // Codex
        (
            HostKind::Codex,
            HostFormat::Toml,
            expand_path("~/.codex/config.toml"),
            vec![expand_path("~/.codex")], // Codex dir exists if installed
        ),
        // Cursor (macOS)
        (
            HostKind::Cursor,
            HostFormat::Json,
            expand_path("~/Library/Application Support/Cursor/User/settings.json"),
            vec![
                expand_path("~/Library/Application Support/Cursor"),
                expand_path("/Applications/Cursor.app"),
            ],
        ),
        // Cursor (Linux)
        (
            HostKind::Cursor,
            HostFormat::Json,
            expand_path("~/.config/Cursor/User/settings.json"),
            vec![expand_path("~/.config/Cursor")],
        ),
        // VSCode (macOS)
        (
            HostKind::VSCode,
            HostFormat::Json,
            expand_path("~/Library/Application Support/Code/User/settings.json"),
            vec![
                expand_path("~/Library/Application Support/Code"),
                expand_path("/Applications/Visual Studio Code.app"),
            ],
        ),
        // VSCode (Linux)
        (
            HostKind::VSCode,
            HostFormat::Json,
            expand_path("~/.config/Code/User/settings.json"),
            vec![expand_path("~/.config/Code")],
        ),
        // Claude
        (
            HostKind::Claude,
            HostFormat::Json,
            expand_path("~/.config/Claude/claude_config.json"),
            vec![
                expand_path("~/.config/Claude"),
                expand_path("/Applications/Claude.app"),
            ],
        ),
        // JetBrains
        (
            HostKind::JetBrains,
            HostFormat::Json,
            expand_path("~/Library/Application Support/JetBrains/LLM/mcp.json"),
            vec![expand_path("~/Library/Application Support/JetBrains")],
        ),
    ]
}

/// Check if any of the indicator paths exist (app is installed)
fn is_app_installed(indicator_paths: &[PathBuf]) -> bool {
    indicator_paths.iter().any(|p| p.exists())
}

/// Detect MCP clients (host applications)
/// Detects both clients with existing config AND installed apps without config
pub fn detect_clients() -> Vec<ClientEntry> {
    // First, get clients with existing configs (original behavior)
    let hosts_with_config = discover_hosts();
    let mut clients = Vec::new();
    let mut seen_kinds: std::collections::HashSet<(HostKind, PathBuf)> =
        std::collections::HashSet::new();

    // Add clients with existing config files
    for host in hosts_with_config {
        seen_kinds.insert((host.kind.clone(), host.path.clone()));

        let scan_result = scan_host_file(&host).ok();

        let services: Vec<String> = scan_result
            .as_ref()
            .map(|r| r.services.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default();

        let already_rewired = scan_result
            .as_ref()
            .map(|r| {
                r.services
                    .iter()
                    .any(|s| s.command.contains("rmcp_mux") || s.command.contains("rmcp_mux_proxy"))
            })
            .unwrap_or(false);

        clients.push(ClientEntry {
            kind: host.kind,
            config_path: host.path,
            selected: !already_rewired,
            services,
            already_rewired,
            config_exists: true,
        });
    }

    // Now check for installed apps without config files
    for (kind, _format, config_path, indicator_paths) in get_known_clients() {
        // Skip if we already found this config
        if seen_kinds.contains(&(kind.clone(), config_path.clone())) {
            continue;
        }

        // Check if app is installed but config doesn't exist
        if !config_path.exists() && is_app_installed(&indicator_paths) {
            clients.push(ClientEntry {
                kind,
                config_path,
                selected: true, // Auto-select new clients
                services: Vec::new(),
                already_rewired: false,
                config_exists: false,
            });
        }
    }

    clients
}
