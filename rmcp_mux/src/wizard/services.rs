//! Service loading, detection, and health check logic.

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::config::{ServerConfig, expand_path, load_config};

use super::types::{FormState, HealthStatus, ServiceEntry, ServiceSource};

// ─────────────────────────────────────────────────────────────────────────────
// Service loading
// ─────────────────────────────────────────────────────────────────────────────

pub fn load_all_services(path: &Path) -> Result<Vec<ServiceEntry>> {
    let cfg = load_config(path)?;
    let mut services = Vec::new();

    // Load from config file
    if let Some(cfg) = cfg {
        for (name, server_cfg) in cfg.servers {
            services.push(ServiceEntry {
                name,
                config: server_cfg,
                health: HealthStatus::Unknown,
                dirty: false,
                source: ServiceSource::Config,
                pid: None,
                selected: true,
            });
        }
    }

    // Detect running MCP processes and merge
    let detected = detect_running_mcp_servers();
    for mut det in detected {
        // Check if we already have this service in config (by matching command+args)
        let already_configured = services.iter().any(|s| {
            // Match by name or by command+args combination
            s.name == det.name
                || (s.config.cmd == det.config.cmd && s.config.args == det.config.args)
        });

        if !already_configured {
            // Generate a socket path for the detected service
            let socket_path = format!("~/mcp-sockets/{}.sock", det.name);
            det.config.socket = Some(socket_path);
            services.push(det);
        }
    }

    // Sort: config entries first, then detected, both alphabetically
    services.sort_by(|a, b| match (&a.source, &b.source) {
        (ServiceSource::Config, ServiceSource::Detected) => std::cmp::Ordering::Less,
        (ServiceSource::Detected, ServiceSource::Config) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(services)
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP process detection
// ─────────────────────────────────────────────────────────────────────────────

/// Patterns that indicate an MCP server process
const MCP_PATTERNS: &[&str] = &[
    "@modelcontextprotocol/",
    "mcp-server-",
    "server-memory",
    "server-filesystem",
    "server-github",
    "server-gitlab",
    "server-slack",
    "server-google-drive",
    "server-postgres",
    "server-sqlite",
    "server-redis",
    "server-brave-search",
    "server-fetch",
    "server-puppeteer",
    "server-sequential-thinking",
    "claude-mcp",
    "mcp_server",
];

/// Detect running MCP server processes by scanning `ps` output
pub fn detect_running_mcp_servers() -> Vec<ServiceEntry> {
    let mut detected = Vec::new();

    // Run `ps -eo pid,args` to get all processes with their arguments
    let output = match Command::new("ps").args(["-eo", "pid,args"]).output() {
        Ok(o) => o,
        Err(_) => return detected,
    };

    if !output.status.success() {
        return detected;
    }

    let reader = BufReader::new(&output.stdout[..]);
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim();

        // Skip header line
        if line.starts_with("PID") {
            continue;
        }

        // Parse PID and args
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            continue;
        }

        let pid: u32 = match parts[0].trim().parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let args = parts[1].trim();

        // Check if this process matches any MCP pattern
        let is_mcp = MCP_PATTERNS.iter().any(|pattern| args.contains(pattern));
        if !is_mcp {
            continue;
        }

        // Skip rmcp_mux itself and its proxy
        if args.contains("rmcp_mux") {
            continue;
        }

        // Extract a meaningful name from the process
        let name = extract_service_name(args);

        // Ensure unique names
        let unique_name = if seen_names.contains(&name) {
            let mut counter = 2;
            loop {
                let candidate = format!("{}-{}", name, counter);
                if !seen_names.contains(&candidate) {
                    break candidate;
                }
                counter += 1;
            }
        } else {
            name
        };
        seen_names.insert(unique_name.clone());

        // Try to extract command and args from the process line
        let (cmd, cmd_args) = extract_cmd_and_args(args);

        let config = ServerConfig {
            socket: None, // Will be generated when user configures
            cmd: Some(cmd),
            args: Some(cmd_args),
            max_active_clients: Some(5),
            tray: Some(false),
            service_name: Some(unique_name.clone()),
            log_level: Some("info".into()),
            lazy_start: Some(false),
            max_request_bytes: Some(1_048_576),
            request_timeout_ms: Some(30_000),
            restart_backoff_ms: Some(1_000),
            restart_backoff_max_ms: Some(30_000),
            max_restarts: Some(5),
            status_file: None,
        };

        detected.push(ServiceEntry {
            name: unique_name,
            config,
            health: HealthStatus::Healthy, // It's running, so it's "healthy"
            dirty: false,
            source: ServiceSource::Detected,
            pid: Some(pid),
            selected: true,
        });
    }

    detected
}

/// Extract a human-readable service name from process arguments
fn extract_service_name(args: &str) -> String {
    // Try to find @modelcontextprotocol/server-XXX pattern
    if let Some(idx) = args.find("@modelcontextprotocol/") {
        let rest = &args[idx + "@modelcontextprotocol/".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            return name;
        }
    }

    // Try to find mcp-server-XXX pattern
    if let Some(idx) = args.find("mcp-server-") {
        let rest = &args[idx + "mcp-server-".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            return format!("mcp-{}", name);
        }
    }

    // Try to find server-XXX pattern (common MCP naming)
    if let Some(idx) = args.find("server-") {
        let rest = &args[idx + "server-".len()..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            return name;
        }
    }

    // Fallback: use a generic name
    "detected-mcp".into()
}

/// Extract command and arguments from a process command line
fn extract_cmd_and_args(args: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        return ("unknown".into(), vec![]);
    }

    // Find the main command (npx, node, python, etc.)
    let cmd = if parts[0].contains('/') {
        // Full path - extract just the binary name
        parts[0].rsplit('/').next().unwrap_or(parts[0]).to_string()
    } else {
        parts[0].to_string()
    };

    // Everything after the command is args
    let cmd_args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    (cmd, cmd_args)
}

// ─────────────────────────────────────────────────────────────────────────────
// Health check
// ─────────────────────────────────────────────────────────────────────────────

pub fn check_health(config: &ServerConfig) -> HealthStatus {
    let socket_path = match &config.socket {
        Some(s) => expand_path(s),
        None => return HealthStatus::Unknown,
    };

    // Try to connect to the socket synchronously
    match UnixStream::connect(&socket_path) {
        Ok(_) => HealthStatus::Healthy,
        Err(_) => HealthStatus::Unhealthy,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Default config and form conversions
// ─────────────────────────────────────────────────────────────────────────────

pub fn default_server_config() -> ServerConfig {
    ServerConfig {
        socket: Some("~/mcp-sockets/general-memory.sock".into()),
        cmd: Some("npx".into()),
        args: Some(vec!["@modelcontextprotocol/server-memory".into()]),
        max_active_clients: Some(5),
        tray: Some(false),
        service_name: None,
        log_level: Some("info".into()),
        lazy_start: Some(false),
        max_request_bytes: Some(1_048_576),
        request_timeout_ms: Some(30_000),
        restart_backoff_ms: Some(1_000),
        restart_backoff_max_ms: Some(30_000),
        max_restarts: Some(5),
        status_file: None,
    }
}

pub fn form_from_service(svc: &ServiceEntry) -> FormState {
    FormState {
        service_name: svc.name.clone(),
        socket: svc.config.socket.clone().unwrap_or_default(),
        cmd: svc.config.cmd.clone().unwrap_or_else(|| "npx".into()),
        args: svc.config.args.clone().unwrap_or_default().join(" "),
        max_clients: svc.config.max_active_clients.unwrap_or(5).to_string(),
        log_level: svc
            .config
            .log_level
            .clone()
            .unwrap_or_else(|| "info".into()),
        tray: svc.config.tray.unwrap_or(false),
        dirty: false,
    }
}

pub fn service_from_form(form: &FormState) -> ServerConfig {
    let args_vec: Vec<String> = form
        .args
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    ServerConfig {
        socket: Some(form.socket.clone()),
        cmd: Some(form.cmd.clone()),
        args: Some(args_vec),
        max_active_clients: form.max_clients.trim().parse().ok(),
        tray: Some(form.tray),
        service_name: Some(form.service_name.clone()),
        log_level: Some(form.log_level.clone()),
        lazy_start: Some(false),
        max_request_bytes: Some(1_048_576),
        request_timeout_ms: Some(30_000),
        restart_backoff_ms: Some(1_000),
        restart_backoff_max_ms: Some(30_000),
        max_restarts: Some(5),
        status_file: None,
    }
}
