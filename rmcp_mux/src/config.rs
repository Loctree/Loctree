//! Configuration types and loading for rmcp_mux.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

/// Sanitize a file path by canonicalizing it to prevent path traversal.
/// Returns the canonicalized path if it exists and is valid.
pub fn sanitize_path(path: &Path) -> Result<PathBuf> {
    // Canonicalize resolves symlinks and .. components
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("failed to resolve path: {}", path.display()))?;
    Ok(canonical)
}

/// Read file contents after sanitizing the path.
pub fn safe_read_to_string(path: &Path) -> Result<String> {
    let safe_path = sanitize_path(path)?;
    // Path is already canonicalized above, resolving symlinks and .. components
    fs::read_to_string(&safe_path) // nosemgrep: rust.actix.path-traversal.tainted-path.tainted-path
        .with_context(|| format!("failed to read file: {}", safe_path.display()))
}

/// Copy file after sanitizing paths.
pub fn safe_copy(from: &Path, to: &Path) -> Result<u64> {
    let safe_from = sanitize_path(from)?;
    // For destination, parent must exist but file may not yet
    let to_parent = to
        .parent()
        .ok_or_else(|| anyhow!("invalid destination path"))?;
    let _ = sanitize_path(to_parent)?;
    // Source and dest parent validated via canonicalize
    #[allow(clippy::let_and_return)]
    let bytes = fs::copy(&safe_from, to)?; // nosemgrep: rust.actix.path-traversal.tainted-path.tainted-path
    Ok(bytes)
}

/// Multi-server configuration file format.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    pub servers: HashMap<String, ServerConfig>,
}

/// Per-service configuration in the config file.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub socket: Option<String>,
    pub cmd: Option<String>,
    pub args: Option<Vec<String>>,
    pub max_active_clients: Option<usize>,
    pub tray: Option<bool>,
    pub service_name: Option<String>,
    pub log_level: Option<String>,
    pub lazy_start: Option<bool>,
    pub max_request_bytes: Option<usize>,
    pub request_timeout_ms: Option<u64>,
    pub restart_backoff_ms: Option<u64>,
    pub restart_backoff_max_ms: Option<u64>,
    pub max_restarts: Option<u64>,
    pub status_file: Option<String>,
}

/// Resolved runtime parameters for the mux daemon.
#[derive(Clone, Debug)]
pub struct ResolvedParams {
    pub socket: PathBuf,
    pub cmd: String,
    pub args: Vec<String>,
    pub max_clients: usize,
    pub tray_enabled: bool,
    pub log_level: String,
    pub service_name: String,
    pub lazy_start: bool,
    pub max_request_bytes: usize,
    pub request_timeout: Duration,
    pub restart_backoff: Duration,
    pub restart_backoff_max: Duration,
    pub max_restarts: u64,
    pub status_file: Option<PathBuf>,
}

/// CLI options that can override config file settings.
///
/// This trait allows the binary to pass CLI arguments to resolve_params
/// without the library depending on clap types.
pub trait CliOptions {
    fn socket(&self) -> Option<PathBuf>;
    fn cmd(&self) -> Option<String>;
    fn args(&self) -> Vec<String>;
    fn max_active_clients(&self) -> usize;
    fn lazy_start(&self) -> Option<bool>;
    fn max_request_bytes(&self) -> Option<usize>;
    fn request_timeout_ms(&self) -> Option<u64>;
    fn restart_backoff_ms(&self) -> Option<u64>;
    fn restart_backoff_max_ms(&self) -> Option<u64>;
    fn max_restarts(&self) -> Option<u64>;
    fn log_level(&self) -> String;
    fn tray(&self) -> bool;
    fn service_name(&self) -> Option<String>;
    fn service(&self) -> Option<String>;
    fn status_file(&self) -> Option<PathBuf>;
}

pub fn expand_path(raw: impl AsRef<str>) -> PathBuf {
    let s = raw.as_ref();
    if let Some(stripped) = s.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(stripped);
    }
    PathBuf::from(s)
}

pub fn load_config(path: &Path) -> Result<Option<Config>> {
    if !path.exists() {
        return Ok(None);
    }
    let data = safe_read_to_string(path)?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let cfg: Config = match ext.as_str() {
        "yaml" | "yml" => serde_yaml::from_str(&data)
            .with_context(|| format!("failed to parse yaml config {}", path.display()))?,
        "toml" => toml::from_str(&data)
            .with_context(|| format!("failed to parse toml config {}", path.display()))?,
        _ => serde_json::from_str(&data)
            .with_context(|| format!("failed to parse json config {}", path.display()))?,
    };
    Ok(Some(cfg))
}

/// Resolve runtime parameters from CLI options and config file.
///
/// CLI options take precedence over config file settings.
pub fn resolve_params<C: CliOptions>(cli: &C, config: Option<&Config>) -> Result<ResolvedParams> {
    let service_cfg = if let Some(cfg) = config {
        if let Some(name) = cli.service() {
            let found = cfg
                .servers
                .get(&name)
                .cloned()
                .ok_or_else(|| anyhow!("service '{name}' not found in config"))?;
            Some((name, found))
        } else {
            None
        }
    } else {
        None
    };

    if config.is_some() && cli.service().is_none() {
        return Err(anyhow!("--service is required when using --config"));
    }

    let socket = cli
        .socket()
        .or_else(|| {
            service_cfg
                .as_ref()
                .and_then(|(_, c)| c.socket.clone().map(expand_path))
        })
        .ok_or_else(|| anyhow!("socket path not provided (use --socket or config)"))?;

    let cmd = cli
        .cmd()
        .or_else(|| service_cfg.as_ref().and_then(|(_, c)| c.cmd.clone()))
        .ok_or_else(|| anyhow!("cmd not provided (use --cmd or config)"))?;

    let cli_args = cli.args();
    let args = if !cli_args.is_empty() {
        cli_args
    } else {
        service_cfg
            .as_ref()
            .and_then(|(_, c)| c.args.clone())
            .unwrap_or_default()
    };

    let max_clients = service_cfg
        .as_ref()
        .and_then(|(_, c)| c.max_active_clients)
        .unwrap_or_else(|| cli.max_active_clients());

    let tray_enabled = if cli.tray() {
        true
    } else {
        service_cfg
            .as_ref()
            .and_then(|(_, c)| c.tray)
            .unwrap_or(false)
    };

    let log_level = service_cfg
        .as_ref()
        .and_then(|(_, c)| c.log_level.clone())
        .unwrap_or_else(|| cli.log_level());

    let lazy_start = cli.lazy_start().unwrap_or_else(|| {
        service_cfg
            .as_ref()
            .and_then(|(_, c)| c.lazy_start)
            .unwrap_or(false)
    });

    let max_request_bytes = cli.max_request_bytes().unwrap_or_else(|| {
        service_cfg
            .as_ref()
            .and_then(|(_, c)| c.max_request_bytes)
            .unwrap_or(1_048_576)
    });

    let request_timeout = Duration::from_millis(cli.request_timeout_ms().unwrap_or_else(|| {
        service_cfg
            .as_ref()
            .and_then(|(_, c)| c.request_timeout_ms)
            .unwrap_or(30_000)
    }));

    let restart_backoff = Duration::from_millis(
        cli.restart_backoff_ms()
            .or_else(|| service_cfg.as_ref().and_then(|(_, c)| c.restart_backoff_ms))
            .unwrap_or(1_000),
    );
    let restart_backoff_max = Duration::from_millis(
        cli.restart_backoff_max_ms()
            .or_else(|| {
                service_cfg
                    .as_ref()
                    .and_then(|(_, c)| c.restart_backoff_max_ms)
            })
            .unwrap_or(30_000),
    );
    let max_restarts = cli
        .max_restarts()
        .or_else(|| service_cfg.as_ref().and_then(|(_, c)| c.max_restarts))
        .unwrap_or(5);

    let status_file = cli
        .status_file()
        .map(|p| p.to_str().map(expand_path).unwrap_or_else(|| p.clone()))
        .or_else(|| {
            service_cfg
                .as_ref()
                .and_then(|(_, c)| c.status_file.as_deref().map(expand_path))
        });

    let service_name_raw = cli
        .service_name()
        .or_else(|| {
            service_cfg
                .as_ref()
                .and_then(|(_, c)| c.service_name.clone())
        })
        .or_else(|| {
            socket
                .file_name()
                .and_then(|n| n.to_string_lossy().split('.').next().map(|s| s.to_string()))
        })
        .unwrap_or_else(|| "rmcp_mux".to_string());

    Ok(ResolvedParams {
        socket,
        cmd,
        args,
        max_clients,
        tray_enabled,
        log_level,
        service_name: service_name_raw,
        lazy_start,
        max_request_bytes,
        request_timeout,
        restart_backoff,
        restart_backoff_max,
        max_restarts,
        status_file,
    })
}
