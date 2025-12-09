use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::config::{Config, ServerConfig, expand_path, safe_copy, safe_read_to_string};

#[derive(Args, Debug, Clone)]
pub struct ScanArgs {
    /// Optional manifest output path.
    #[arg(long)]
    pub manifest: Option<PathBuf>,
    /// Manifest format: toml|json|yaml
    #[arg(long, default_value = "toml")]
    pub manifest_format: String,
    /// Optional snippet output path (per-host a suffix is added).
    #[arg(long)]
    pub snippet: Option<PathBuf>,
    /// Snippet format: toml|json|yaml
    #[arg(long, default_value = "toml")]
    pub snippet_format: String,
    /// Socket directory for generated services.
    #[arg(long, default_value = "~/.rmcp_servers/rmcp_mux/sockets")]
    pub socket_dir: String,
    /// Do not write files; print to stdout.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RewireArgs {
    /// Explicit path to host config; otherwise auto-discovery is used.
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Host kind to target (codex|cursor|vscode|claude|jetbrains).
    #[arg(long)]
    pub host: Option<String>,
    /// Socket directory used for proxy args.
    #[arg(long, default_value = "~/.rmcp_servers/rmcp_mux/sockets")]
    pub socket_dir: String,
    /// Proxy command used in rewritten config.
    #[arg(long, default_value = "rmcp_mux_proxy")]
    pub proxy_cmd: String,
    /// Extra args passed before --socket.
    #[arg(long, value_delimiter = ' ')]
    pub proxy_args: Vec<String>,
    /// Only show planned changes.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Args, Debug, Clone)]
pub struct StatusArgs {
    /// Explicit host config path.
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Host kind (codex|cursor|vscode|claude|jetbrains).
    #[arg(long)]
    pub host: Option<String>,
    /// Expected proxy command (default rmcp_mux_proxy).
    #[arg(long, default_value = "rmcp_mux_proxy")]
    pub proxy_cmd: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub enum HostKind {
    Codex,
    Cursor,
    VSCode,
    Claude,
    JetBrains,
    Unknown,
}

impl HostKind {
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
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum HostFormat {
    Toml,
    Json,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostFile {
    pub kind: HostKind,
    pub path: PathBuf,
    pub format: HostFormat,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostService {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub socket: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub host: HostFile,
    pub services: Vec<HostService>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RewireOutcome {
    pub path: PathBuf,
    pub backup: Option<PathBuf>,
    pub written: bool,
}

#[derive(Deserialize)]
struct RawHostConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: Option<HashMap<String, RawServer>>,
}

#[derive(Deserialize)]
struct RawServer {
    command: Option<String>,
    args: Option<Vec<String>>,
    socket: Option<String>,
    env: Option<HashMap<String, String>>,
}

pub fn discover_hosts() -> Vec<HostFile> {
    let mut files = Vec::new();
    let candidates: Vec<(HostKind, HostFormat, PathBuf)> = vec![
        (
            HostKind::Codex,
            HostFormat::Toml,
            expand_path("~/.codex/config.toml"),
        ),
        (
            HostKind::Cursor,
            HostFormat::Json,
            expand_path("~/Library/Application Support/Cursor/User/settings.json"),
        ),
        (
            HostKind::Cursor,
            HostFormat::Json,
            expand_path("~/.config/Cursor/User/settings.json"),
        ),
        (
            HostKind::VSCode,
            HostFormat::Json,
            expand_path("~/Library/Application Support/Code/User/settings.json"),
        ),
        (
            HostKind::VSCode,
            HostFormat::Json,
            expand_path("~/.config/Code/User/settings.json"),
        ),
        (
            HostKind::Claude,
            HostFormat::Json,
            expand_path("~/.config/Claude/claude_config.json"),
        ),
        (
            HostKind::JetBrains,
            HostFormat::Json,
            expand_path("~/Library/Application Support/JetBrains/LLM/mcp.json"),
        ),
    ];

    for (kind, format, path) in candidates {
        if path.exists() {
            files.push(HostFile { kind, path, format });
        }
    }
    files
}

pub fn scan_hosts() -> Vec<ScanResult> {
    discover_hosts()
        .into_iter()
        .filter_map(|hf| match scan_host_file(&hf) {
            Ok(res) => Some(res),
            Err(err) => {
                tracing::warn!("failed to scan {}: {err}", hf.path.display());
                None
            }
        })
        .collect()
}

pub fn format_for_host(host: &HostFile) -> &'static str {
    match host.format {
        HostFormat::Json => "json",
        HostFormat::Toml => "toml",
    }
}

pub fn scan_host_file(file: &HostFile) -> Result<ScanResult> {
    let data = safe_read_to_string(&file.path)?;
    let raw: RawHostConfig = match file.format {
        HostFormat::Toml => toml::from_str(&data)
            .with_context(|| format!("failed to parse toml {}", file.path.display()))?,
        HostFormat::Json => serde_json::from_str(&data)
            .with_context(|| format!("failed to parse json {}", file.path.display()))?,
    };

    let mut services = Vec::new();
    if let Some(map) = raw.mcp_servers {
        for (name, srv) in map {
            let command = srv
                .command
                .ok_or_else(|| anyhow!("service {name} missing command"))?;
            let args = srv.args.unwrap_or_default();
            services.push(HostService {
                name,
                command,
                args,
                socket: srv.socket,
                env: srv.env,
            });
        }
    }

    Ok(ScanResult {
        host: file.clone(),
        services,
    })
}

pub fn build_manifest(scans: &[ScanResult], socket_dir: &Path) -> Config {
    let mut cfg = Config::default();
    for scan in scans {
        for svc in &scan.services {
            let socket = svc.socket.clone().unwrap_or_else(|| {
                socket_dir
                    .join(format!("{}.sock", svc.name))
                    .to_string_lossy()
                    .to_string()
            });
            cfg.servers.insert(
                svc.name.clone(),
                ServerConfig {
                    socket: Some(socket),
                    cmd: Some(svc.command.clone()),
                    args: Some(svc.args.clone()),
                    max_active_clients: Some(5),
                    tray: Some(false),
                    service_name: Some(svc.name.clone()),
                    log_level: Some("info".into()),
                    lazy_start: Some(false),
                    max_request_bytes: Some(1_048_576),
                    request_timeout_ms: Some(30_000),
                    restart_backoff_ms: Some(1_000),
                    restart_backoff_max_ms: Some(30_000),
                    max_restarts: Some(5),
                    status_file: None,
                },
            );
        }
    }
    cfg
}

pub fn generate_snippet(
    scans: &[ScanResult],
    socket_dir: &Path,
    proxy_cmd: &str,
    proxy_args: &[String],
) -> HashMap<HostKind, serde_json::Value> {
    let mut snippets = HashMap::new();

    let mut servers = serde_json::Map::new();
    for scan in scans {
        for svc in &scan.services {
            let socket = svc.socket.clone().unwrap_or_else(|| {
                socket_dir
                    .join(format!("{}.sock", svc.name))
                    .to_string_lossy()
                    .to_string()
            });
            let mut args = Vec::new();
            args.extend(proxy_args.to_owned());
            args.push("--socket".to_string());
            args.push(socket);

            let mut server = serde_json::Map::new();
            server.insert(
                "command".to_string(),
                serde_json::Value::String(proxy_cmd.to_string()),
            );
            server.insert(
                "args".to_string(),
                serde_json::Value::Array(args.into_iter().map(serde_json::Value::String).collect()),
            );
            if let Some(env) = &svc.env {
                server.insert(
                    "env".to_string(),
                    serde_json::Value::Object(
                        env.iter()
                            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                            .collect(),
                    ),
                );
            }
            servers.insert(svc.name.clone(), serde_json::Value::Object(server));
        }
        let mut root = serde_json::Map::new();
        root.insert(
            "mcpServers".to_string(),
            serde_json::Value::Object(servers.clone()),
        );
        snippets.insert(
            scan.host.kind.clone(),
            serde_json::Value::Object(root.clone()),
        );
    }

    snippets
}

pub fn rewire_host(
    host: &HostFile,
    socket_dir: &Path,
    proxy_cmd: &str,
    proxy_args: &[String],
    dry_run: bool,
) -> Result<RewireOutcome> {
    let scan = scan_host_file(host)?;
    let snippets = generate_snippet(&[scan], socket_dir, proxy_cmd, proxy_args);
    let snippet = snippets
        .get(&host.kind)
        .ok_or_else(|| anyhow!("no snippet generated for host"))?;
    let format = format_for_host(host);
    let snippet_text = serialize_snippet(snippet, format)?;
    let data = safe_read_to_string(&host.path)?;

    let merged = match host.format {
        HostFormat::Json => {
            let mut root: serde_json::Value = serde_json::from_str(&data)
                .with_context(|| format!("failed to parse json {}", host.path.display()))?;
            let obj = root
                .as_object_mut()
                .ok_or_else(|| anyhow!("host json must be an object"))?;
            let snippet_json: serde_json::Value =
                serde_json::from_str(&snippet_text).context("parse snippet json")?;
            let mcp = snippet_json
                .get("mcpServers")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
            obj.insert("mcpServers".into(), mcp);
            serde_json::to_string_pretty(&root).context("serialize merged json")?
        }
        HostFormat::Toml => {
            let mut root: toml::Value = toml::from_str(&data)
                .with_context(|| format!("failed to parse toml {}", host.path.display()))?;
            let snippet_toml: toml::Value =
                toml::from_str(&snippet_text).context("parse snippet toml")?;
            let mcp = snippet_toml
                .get("mcpServers")
                .cloned()
                .unwrap_or_else(|| toml::Value::Table(Default::default()));
            let table = root
                .as_table_mut()
                .ok_or_else(|| anyhow!("host toml must be a table"))?;
            table.insert("mcpServers".into(), mcp);
            toml::to_string_pretty(&root).context("serialize merged toml")?
        }
    };

    let backup = write_with_backup(&host.path, &merged, dry_run)?;
    Ok(RewireOutcome {
        path: host.path.clone(),
        backup,
        written: !dry_run,
    })
}

pub fn write_with_backup(path: &Path, contents: &str, dry_run: bool) -> Result<Option<PathBuf>> {
    if dry_run {
        println!("--- {} (dry-run) ---\n{}", path.display(), contents);
        return Ok(None);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let backup = path.with_extension("bak");
    if path.exists() {
        safe_copy(path, &backup)?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(Some(backup))
}

pub fn serialize_config(config: &Config, format: &str) -> Result<String> {
    match format {
        "json" => serde_json::to_string_pretty(config).context("serialize json"),
        "yaml" | "yml" => serde_yaml::to_string(config).context("serialize yaml"),
        "toml" => toml::to_string_pretty(config).context("serialize toml"),
        other => Err(anyhow!("unsupported format {other}")),
    }
}

pub fn serialize_snippet(snippet: &serde_json::Value, format: &str) -> Result<String> {
    match format {
        "json" => serde_json::to_string_pretty(snippet).context("serialize snippet json"),
        "yaml" | "yml" => serde_yaml::to_string(snippet).context("serialize snippet yaml"),
        "toml" => toml::to_string_pretty(snippet).context("serialize snippet toml"),
        other => Err(anyhow!("unsupported format {other}")),
    }
}

pub fn resolve_host_from_args(args: &RewireArgs) -> Result<HostFile> {
    if let Some(path) = &args.path {
        let fmt = match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("toml") => HostFormat::Toml,
            _ => HostFormat::Json,
        };
        return Ok(HostFile {
            kind: HostKind::Unknown,
            path: path.clone(),
            format: fmt,
        });
    }

    let discovered = discover_hosts();
    if discovered.is_empty() {
        return Err(anyhow!("no host configs found"));
    }
    if let Some(host) = &args.host {
        let lower = host.to_ascii_lowercase();
        let target = discovered.into_iter().find(|h| h.kind.as_label() == lower);
        return target.ok_or_else(|| anyhow!("host {host} not found"));
    }

    Ok(discovered[0].clone())
}

pub fn resolve_status_host(args: &StatusArgs) -> Result<HostFile> {
    if let Some(path) = &args.path {
        let fmt = match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("toml") => HostFormat::Toml,
            _ => HostFormat::Json,
        };
        return Ok(HostFile {
            kind: HostKind::Unknown,
            path: path.clone(),
            format: fmt,
        });
    }
    let discovered = discover_hosts();
    if discovered.is_empty() {
        return Err(anyhow!("no host configs found"));
    }
    if let Some(host) = &args.host {
        let lower = host.to_ascii_lowercase();
        if let Some(h) = discovered.into_iter().find(|h| h.kind.as_label() == lower) {
            return Ok(h);
        }
        return Err(anyhow!("host {host} not found"));
    }
    Ok(discovered[0].clone())
}

pub fn run_scan_cmd(args: ScanArgs) -> Result<()> {
    let socket_dir = expand_path(args.socket_dir);
    let scans = scan_hosts();
    if scans.is_empty() {
        println!("No host configs discovered.");
        return Ok(());
    }

    let manifest = build_manifest(&scans, &socket_dir);

    if let Some(path) = args.manifest {
        let text = serialize_config(&manifest, &args.manifest_format.to_lowercase())?;
        let backup = write_with_backup(&path, &text, args.dry_run)?;
        println!(
            "Manifest {}written to {}{}",
            if args.dry_run { "(dry-run) " } else { "" },
            path.display(),
            backup
                .as_ref()
                .map(|b| format!(" (backup {})", b.display()))
                .unwrap_or_default()
        );
    } else {
        println!(
            "Discovered {} host(s), {} service(s). Use --manifest to save mux config.",
            scans.len(),
            manifest.servers.len()
        );
    }

    if args.snippet.is_some() || args.dry_run {
        let snippets = generate_snippet(&scans, &socket_dir, "rmcp_mux_proxy", &[]);
        for (kind, snippet) in snippets {
            let fmt = args.snippet_format.to_lowercase();
            let text = serialize_snippet(&snippet, &fmt)?;
            if let Some(base) = &args.snippet {
                let mut path = base.clone();
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("snippet")
                    .to_string();
                let parent = path.parent().unwrap_or_else(|| Path::new("."));
                path = parent.join(format!("{stem}-{}.{}", kind.as_label(), fmt));
                let backup = write_with_backup(&path, &text, args.dry_run)?;
                println!(
                    "Snippet for {} {}written to {}{}",
                    kind.as_label(),
                    if args.dry_run { "(dry-run) " } else { "" },
                    path.display(),
                    backup
                        .as_ref()
                        .map(|b| format!(" (backup {})", b.display()))
                        .unwrap_or_default()
                );
            } else {
                println!("--- snippet ({}) ---\n{}", kind.as_label(), text);
            }
        }
    }

    Ok(())
}

pub fn run_rewire_cmd(args: RewireArgs) -> Result<()> {
    let target = resolve_host_from_args(&args)?;
    let socket_dir = expand_path(&args.socket_dir);
    let outcome = rewire_host(
        &target,
        &socket_dir,
        &args.proxy_cmd,
        &args.proxy_args,
        args.dry_run,
    )?;
    println!(
        "{}rewired {} (backup: {})",
        if args.dry_run { "Would have " } else { "" },
        outcome.path.display(),
        outcome
            .backup
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".into())
    );
    Ok(())
}

pub fn run_status_cmd(args: StatusArgs) -> Result<()> {
    let target = resolve_status_host(&args)?;
    let scan = scan_host_file(&target)?;
    if scan.services.is_empty() {
        println!("{}: no services found in config", target.path.display());
        return Ok(());
    }
    println!(
        "Checking {} ({})",
        target.path.display(),
        target.kind.as_label()
    );
    for svc in &scan.services {
        let uses_proxy = svc.command == args.proxy_cmd;
        if uses_proxy {
            println!(" - {}: OK (via {})", svc.name, svc.command);
        } else {
            println!(
                " - {}: NOT rewired (command='{}', args={:?})",
                svc.name, svc.command, svc.args
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_json(path: &Path, body: &serde_json::Value) {
        fs::create_dir_all(path.parent().expect("json parent dir")).expect("create parent");
        let json = serde_json::to_string_pretty(body).expect("encode json");
        fs::write(path, json).expect("write json");
    }

    #[test]
    fn scan_host_file_reads_json() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let json = serde_json::json!({
            "mcpServers": {
                "memory": {"command": "npx", "args": ["@mcp/server-memory"], "socket": "/tmp/mem.sock"}
            }
        });
        write_json(&path, &json);

        let file = HostFile {
            kind: HostKind::Codex,
            path: path.clone(),
            format: HostFormat::Json,
        };
        let scan = scan_host_file(&file).expect("scan file");
        assert_eq!(scan.services.len(), 1);
        assert_eq!(scan.services[0].name, "memory");
        assert_eq!(scan.services[0].command, "npx");
        assert_eq!(scan.services[0].socket.as_deref(), Some("/tmp/mem.sock"));
    }

    #[test]
    fn build_manifest_populates_defaults() {
        let scans = vec![ScanResult {
            host: HostFile {
                kind: HostKind::Codex,
                path: PathBuf::from("dummy"),
                format: HostFormat::Toml,
            },
            services: vec![HostService {
                name: "memory".into(),
                command: "npx".into(),
                args: vec!["@mcp/server-memory".into()],
                socket: None,
                env: None,
            }],
        }];
        let cfg = build_manifest(&scans, Path::new("/tmp/sockets"));
        let svc = cfg.servers.get("memory").expect("memory svc");
        assert_eq!(svc.cmd.as_deref(), Some("npx"));
        assert_eq!(
            svc.args.as_ref().expect("args"),
            &vec!["@mcp/server-memory"]
        );
        assert!(
            svc.socket
                .as_ref()
                .expect("socket")
                .contains("/tmp/sockets/memory.sock")
        );
    }

    #[test]
    fn generate_snippet_uses_proxy() {
        let scans = vec![ScanResult {
            host: HostFile {
                kind: HostKind::Codex,
                path: PathBuf::from("dummy"),
                format: HostFormat::Toml,
            },
            services: vec![HostService {
                name: "svc".into(),
                command: "npx".into(),
                args: vec!["x".into()],
                socket: None,
                env: None,
            }],
        }];
        let snippets = generate_snippet(&scans, Path::new("/s"), "rmcp_mux", &["proxy".into()]);
        let node = snippets.get(&HostKind::Codex).expect("codex snippet");
        let servers = node
            .get("mcpServers")
            .and_then(|m| m.as_object())
            .expect("mcpServers map");
        let svc = servers
            .get("svc")
            .expect("svc entry")
            .as_object()
            .expect("svc object");
        assert_eq!(svc.get("command").expect("command"), "rmcp_mux");
        let args = svc
            .get("args")
            .expect("args")
            .as_array()
            .expect("args array")
            .iter()
            .map(|v| v.as_str().expect("string").to_string())
            .collect::<Vec<_>>();
        assert!(args.contains(&"proxy".to_string()));
    }

    #[test]
    fn rewire_updates_mcpservers_in_json() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        let json = serde_json::json!({
            "other": true,
            "mcpServers": {
                "memory": {"command": "npx", "args": ["@mcp/server-memory"]}
            }
        });
        write_json(&path, &json);
        let host = HostFile {
            kind: HostKind::Codex,
            path: path.clone(),
            format: HostFormat::Json,
        };
        rewire_host(
            &host,
            Path::new("/tmp/sockets"),
            "rmcp_mux",
            &["proxy".into()],
            false,
        )
        .expect("rewire");
        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("parse");
        let servers = updated
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .expect("mcpServers");
        let mem = servers
            .get("memory")
            .expect("memory")
            .as_object()
            .expect("memory obj");
        assert_eq!(mem.get("command").expect("command"), "rmcp_mux");
    }
}
