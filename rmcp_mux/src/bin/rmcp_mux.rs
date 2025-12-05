//! rmcp_mux CLI binary
//!
//! This is the command-line interface for rmcp_mux. For library usage,
//! see the `rmcp_mux` crate documentation.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand};
use tracing_subscriber::filter::LevelFilter;

use rmcp_mux::config::{CliOptions, expand_path, load_config, resolve_params};
use rmcp_mux::runtime::{health_check, run_mux, run_proxy};
use rmcp_mux::scan::{
    RewireArgs, ScanArgs, StatusArgs, run_rewire_cmd, run_scan_cmd, run_status_cmd,
};
use rmcp_mux::wizard::WizardArgs;

/// Robust MCP mux: single MCP server child, many clients via UNIX socket,
/// initialize cache, ID rewriting, child restarts, and active client limit.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct RootCli {
    #[command(subcommand)]
    command: Option<CliCommand>,
    #[command(flatten)]
    run: Cli,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Interactive wizard (ratatui) to build mux and host config snippets.
    Wizard(WizardArgs),
    /// Scan host configs and generate mux manifests/snippets.
    Scan(ScanArgs),
    /// Rewire a host config to point to rmcp_mux proxy.
    Rewire(RewireArgs),
    /// Proxy STDIO to a mux socket (for MCP hosts).
    Proxy(ProxyArgs),
    /// Check whether host configs are already pointed at the mux proxy.
    Status(StatusArgs),
    /// Simple health check: resolve config and try connecting to the mux socket.
    Health(HealthArgs),
}

#[derive(Args, Debug, Clone)]
struct Cli {
    /// Unix socket path for the mux listener. Can be overridden by config.
    #[arg(long)]
    socket: Option<PathBuf>,
    /// MCP server command (e.g. `npx`). Can be overridden by config.
    #[arg(long)]
    cmd: Option<String>,
    /// Arguments passed to the MCP server command.
    #[arg(last = true)]
    args: Vec<String>,
    /// Max active clients (permits for concurrent server use).
    #[arg(long, default_value = "5")]
    max_active_clients: usize,
    /// Lazy start MCP child only when first request arrives.
    #[arg(long)]
    lazy_start: Option<bool>,
    /// Maximum request size in bytes before rejecting.
    #[arg(long)]
    max_request_bytes: Option<usize>,
    /// Request timeout in milliseconds before the mux aborts pending calls.
    #[arg(long)]
    request_timeout_ms: Option<u64>,
    /// Initial restart backoff in milliseconds.
    #[arg(long)]
    restart_backoff_ms: Option<u64>,
    /// Maximum restart backoff in milliseconds.
    #[arg(long)]
    restart_backoff_max_ms: Option<u64>,
    /// Maximum restarts before marking server failed (0 = unlimited).
    #[arg(long)]
    max_restarts: Option<u64>,
    /// Log level (trace|debug|info|warn|error).
    #[arg(long, default_value = "info")]
    log_level: String,
    /// Enable tray icon with live server status.
    #[arg(long, default_value_t = false)]
    tray: bool,
    /// Service name shown in tray (defaults to socket file stem).
    #[arg(long)]
    service_name: Option<String>,
    /// Optional config file (default ~/.codex/mcp.json)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Service key inside config (`servers.<name>`)
    #[arg(long)]
    service: Option<String>,
    /// Optional path to write JSON status snapshots.
    #[arg(long)]
    status_file: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
struct ProxyArgs {
    /// Socket path to connect to.
    #[arg(long)]
    socket: PathBuf,
}

#[derive(Args, Debug, Clone)]
struct HealthArgs {
    #[command(flatten)]
    cli: Cli,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = RootCli::parse();

    match &cli.command {
        Some(CliCommand::Wizard(wargs)) => {
            rmcp_mux::wizard::run_wizard(wargs.clone()).await?;
            return Ok(());
        }
        Some(CliCommand::Scan(args)) => {
            run_scan_cmd(args.clone())?;
            return Ok(());
        }
        Some(CliCommand::Rewire(args)) => {
            run_rewire_cmd(args.clone())?;
            return Ok(());
        }
        Some(CliCommand::Proxy(args)) => {
            return run_proxy(args.socket.clone()).await;
        }
        Some(CliCommand::Status(args)) => {
            run_status_cmd(args.clone())?;
            return Ok(());
        }
        Some(CliCommand::Health(args)) => {
            run_health(args.cli.clone()).await?;
            return Ok(());
        }
        None => {}
    }

    let cli = cli.run;

    let config_path = cli
        .config
        .clone()
        .unwrap_or_else(|| expand_path("~/.codex/mcp.json"));
    let config = load_config(&config_path)?;

    let params = resolve_params(&cli, config.as_ref())?;

    let level = params
        .log_level
        .parse::<LevelFilter>()
        .map_err(|_| anyhow!("invalid log level: {}", params.log_level))?;

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    tracing::info!(
        service = params.service_name.as_str(),
        socket = %params.socket.display(),
        cmd = %params.cmd,
        max_clients = params.max_clients,
        tray = params.tray_enabled,
        "mux starting"
    );

    run_mux(params).await
}

async fn run_health(cli: Cli) -> Result<()> {
    let config_path = cli
        .config
        .clone()
        .unwrap_or_else(|| expand_path("~/.codex/mcp.json"));
    let config = load_config(&config_path)?;
    let params = resolve_params(&cli, config.as_ref())?;
    health_check(&params).await?;
    println!("OK: connected to {}", params.socket.display());
    Ok(())
}

// Implement CliOptions trait for the Cli struct
impl CliOptions for Cli {
    fn socket(&self) -> Option<PathBuf> {
        self.socket.clone()
    }
    fn cmd(&self) -> Option<String> {
        self.cmd.clone()
    }
    fn args(&self) -> Vec<String> {
        self.args.clone()
    }
    fn max_active_clients(&self) -> usize {
        self.max_active_clients
    }
    fn lazy_start(&self) -> Option<bool> {
        self.lazy_start
    }
    fn max_request_bytes(&self) -> Option<usize> {
        self.max_request_bytes
    }
    fn request_timeout_ms(&self) -> Option<u64> {
        self.request_timeout_ms
    }
    fn restart_backoff_ms(&self) -> Option<u64> {
        self.restart_backoff_ms
    }
    fn restart_backoff_max_ms(&self) -> Option<u64> {
        self.restart_backoff_max_ms
    }
    fn max_restarts(&self) -> Option<u64> {
        self.max_restarts
    }
    fn log_level(&self) -> String {
        self.log_level.clone()
    }
    fn tray(&self) -> bool {
        self.tray
    }
    fn service_name(&self) -> Option<String> {
        self.service_name.clone()
    }
    fn service(&self) -> Option<String> {
        self.service.clone()
    }
    fn status_file(&self) -> Option<PathBuf> {
        self.status_file.clone()
    }
}
