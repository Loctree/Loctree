//! # rmcp_mux - MCP Server Multiplexer
//!
//! A library for multiplexing MCP (Model Context Protocol) servers, allowing
//! a single server process to serve multiple clients via Unix sockets.
//!
//! ## Features
//!
//! - **Single server, multiple clients**: One MCP server child process serves many clients
//! - **Initialize caching**: First initialize response is cached for subsequent clients
//! - **Request ID rewriting**: Transparent request routing with ID collision avoidance
//! - **Automatic restarts**: Exponential backoff restart of failed server processes
//! - **Active client limiting**: Semaphore-based concurrency control
//!
//! ## Usage as Library
//!
//! ```rust,no_run
//! use rmcp_mux::{MuxConfig, run_mux_server};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = MuxConfig::new("/tmp/my-mcp.sock", "npx")
//!         .with_args(vec!["-y".into(), "@anthropic/mcp-server".into()])
//!         .with_max_clients(10)
//!         .with_service_name("my-mcp-server");
//!
//!     run_mux_server(config).await
//! }
//! ```
//!
//! ## Usage with Multiple Mux Instances
//!
//! ```rust,no_run
//! use rmcp_mux::{MuxConfig, spawn_mux_server, MuxHandle};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Spawn multiple mux servers in a single process
//!     let handles: Vec<MuxHandle> = vec![
//!         spawn_mux_server(MuxConfig::new("/tmp/mcp1.sock", "server1")).await?,
//!         spawn_mux_server(MuxConfig::new("/tmp/mcp2.sock", "server2")).await?,
//!     ];
//!
//!     // Wait for all to complete (or shutdown signal)
//!     for handle in handles {
//!         handle.wait().await?;
//!     }
//!     Ok(())
//! }
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

// ─────────────────────────────────────────────────────────────────────────────
// Public modules
// ─────────────────────────────────────────────────────────────────────────────

pub mod config;
pub mod runtime;
pub mod state;

// CLI-only modules (feature-gated)
#[cfg(feature = "cli")]
pub mod scan;
#[cfg(feature = "tray")]
pub mod tray;
#[cfg(feature = "cli")]
pub mod wizard;

// ─────────────────────────────────────────────────────────────────────────────
// Re-exports for convenience
// ─────────────────────────────────────────────────────────────────────────────

pub use config::{CliOptions, Config, ResolvedParams, ServerConfig};
pub use runtime::{MAX_PENDING, MAX_QUEUE, health_check, run_mux, run_mux_internal, run_proxy};
pub use state::{MuxState, ServerStatus, StatusSnapshot};

// ─────────────────────────────────────────────────────────────────────────────
// Library-first configuration builder
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for embedding rmcp_mux in your application.
///
/// Use the builder pattern to configure the mux server:
///
/// ```rust
/// use rmcp_mux::MuxConfig;
/// use std::time::Duration;
///
/// let config = MuxConfig::new("/tmp/my-mcp.sock", "npx")
///     .with_args(vec!["-y".into(), "my-mcp-server".into()])
///     .with_max_clients(10)
///     .with_request_timeout(Duration::from_secs(60));
/// ```
#[derive(Debug, Clone)]
pub struct MuxConfig {
    /// Unix socket path for the mux listener
    pub socket: PathBuf,
    /// MCP server command (e.g., "npx", "node", "python")
    pub cmd: String,
    /// Arguments passed to the MCP server command
    pub args: Vec<String>,
    /// Maximum concurrent active clients (default: 5)
    pub max_clients: usize,
    /// Service name for logging and status (default: socket filename)
    pub service_name: Option<String>,
    /// Log level (default: "info")
    pub log_level: String,
    /// Lazy start - only spawn server on first client connect (default: false)
    pub lazy_start: bool,
    /// Maximum request size in bytes (default: 1MB)
    pub max_request_bytes: usize,
    /// Request timeout before aborting (default: 30s)
    pub request_timeout: Duration,
    /// Initial restart backoff (default: 1s)
    pub restart_backoff: Duration,
    /// Maximum restart backoff (default: 30s)
    pub restart_backoff_max: Duration,
    /// Maximum restarts before marking server failed (0 = unlimited, default: 5)
    pub max_restarts: u64,
    /// Optional path to write JSON status snapshots
    pub status_file: Option<PathBuf>,
    /// Enable tray icon (only with "tray" feature, default: false)
    pub tray_enabled: bool,
}

impl MuxConfig {
    /// Create a new MuxConfig with required parameters.
    ///
    /// # Arguments
    /// * `socket` - Unix socket path for the mux listener
    /// * `cmd` - MCP server command to execute
    pub fn new(socket: impl Into<PathBuf>, cmd: impl Into<String>) -> Self {
        Self {
            socket: socket.into(),
            cmd: cmd.into(),
            args: Vec::new(),
            max_clients: 5,
            service_name: None,
            log_level: "info".to_string(),
            lazy_start: false,
            max_request_bytes: 1_048_576,
            request_timeout: Duration::from_secs(30),
            restart_backoff: Duration::from_secs(1),
            restart_backoff_max: Duration::from_secs(30),
            max_restarts: 5,
            status_file: None,
            tray_enabled: false,
        }
    }

    /// Set command arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set maximum concurrent clients.
    pub fn with_max_clients(mut self, max: usize) -> Self {
        self.max_clients = max;
        self
    }

    /// Set service name for logging and status.
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = Some(name.into());
        self
    }

    /// Set log level (trace, debug, info, warn, error).
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }

    /// Enable lazy start (spawn server on first client connect).
    pub fn with_lazy_start(mut self, lazy: bool) -> Self {
        self.lazy_start = lazy;
        self
    }

    /// Set maximum request size in bytes.
    pub fn with_max_request_bytes(mut self, bytes: usize) -> Self {
        self.max_request_bytes = bytes;
        self
    }

    /// Set request timeout.
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set restart backoff parameters.
    pub fn with_restart_backoff(mut self, initial: Duration, max: Duration) -> Self {
        self.restart_backoff = initial;
        self.restart_backoff_max = max;
        self
    }

    /// Set maximum restarts (0 = unlimited).
    pub fn with_max_restarts(mut self, max: u64) -> Self {
        self.max_restarts = max;
        self
    }

    /// Set status file path for JSON snapshots.
    pub fn with_status_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.status_file = Some(path.into());
        self
    }

    /// Enable tray icon (requires "tray" feature).
    pub fn with_tray(mut self, enabled: bool) -> Self {
        self.tray_enabled = enabled;
        self
    }

    /// Get the service name (or derive from socket path).
    pub fn service_name(&self) -> String {
        self.service_name.clone().unwrap_or_else(|| {
            self.socket
                .file_name()
                .and_then(|n| n.to_string_lossy().split('.').next().map(|s| s.to_string()))
                .unwrap_or_else(|| "rmcp_mux".to_string())
        })
    }
}

impl From<MuxConfig> for ResolvedParams {
    fn from(cfg: MuxConfig) -> Self {
        let service_name = cfg.service_name();
        ResolvedParams {
            socket: cfg.socket,
            cmd: cfg.cmd,
            args: cfg.args,
            max_clients: cfg.max_clients,
            tray_enabled: cfg.tray_enabled,
            log_level: cfg.log_level,
            service_name,
            lazy_start: cfg.lazy_start,
            max_request_bytes: cfg.max_request_bytes,
            request_timeout: cfg.request_timeout,
            restart_backoff: cfg.restart_backoff,
            restart_backoff_max: cfg.restart_backoff_max,
            max_restarts: cfg.max_restarts,
            status_file: cfg.status_file,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Library entry points
// ─────────────────────────────────────────────────────────────────────────────

/// Run a mux server blocking until shutdown.
///
/// This is the simplest way to run a mux server. It blocks until
/// a shutdown signal (Ctrl+C) is received.
///
/// # Example
/// ```rust,no_run
/// use rmcp_mux::{MuxConfig, run_mux_server};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = MuxConfig::new("/tmp/my-mcp.sock", "my-server");
///     run_mux_server(config).await
/// }
/// ```
pub async fn run_mux_server(config: MuxConfig) -> Result<()> {
    let params: ResolvedParams = config.into();
    run_mux(params).await
}

/// Handle for a spawned mux server.
///
/// Use this to manage multiple mux servers in a single process.
pub struct MuxHandle {
    shutdown: CancellationToken,
    join_handle: tokio::task::JoinHandle<Result<()>>,
}

impl MuxHandle {
    /// Request shutdown of this mux server.
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    /// Wait for the mux server to complete.
    pub async fn wait(self) -> Result<()> {
        self.join_handle.await?
    }

    /// Check if the mux server is still running.
    pub fn is_running(&self) -> bool {
        !self.join_handle.is_finished()
    }
}

/// Spawn a mux server as a background task.
///
/// Returns a handle that can be used to shutdown the server
/// or wait for it to complete.
///
/// # Example
/// ```rust,no_run
/// use rmcp_mux::{MuxConfig, spawn_mux_server};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let handle = spawn_mux_server(MuxConfig::new("/tmp/mcp.sock", "server")).await?;
///
///     // Do other work...
///
///     // Later, shutdown and wait
///     handle.shutdown();
///     handle.wait().await?;
///     Ok(())
/// }
/// ```
pub async fn spawn_mux_server(config: MuxConfig) -> Result<MuxHandle> {
    let shutdown = CancellationToken::new();
    let params: ResolvedParams = config.into();

    let shutdown_clone = shutdown.clone();
    let join_handle = tokio::spawn(async move {
        // Override the internal shutdown signal with our token
        run_mux_with_shutdown(params, shutdown_clone).await
    });

    Ok(MuxHandle {
        shutdown,
        join_handle,
    })
}

/// Run mux with external shutdown control.
///
/// This is useful for embedding where you want to control shutdown
/// programmatically rather than via Ctrl+C.
pub async fn run_mux_with_shutdown(
    params: ResolvedParams,
    shutdown: CancellationToken,
) -> Result<()> {
    runtime::run_mux_internal(params, shutdown).await
}

/// Perform a health check on a mux socket.
///
/// Returns Ok if the socket is reachable, Err otherwise.
pub async fn check_health(socket: impl AsRef<Path>) -> Result<()> {
    let params = ResolvedParams {
        socket: socket.as_ref().to_path_buf(),
        cmd: String::new(),
        args: Vec::new(),
        max_clients: 1,
        tray_enabled: false,
        log_level: "error".to_string(),
        service_name: "health-check".to_string(),
        lazy_start: false,
        max_request_bytes: 0,
        request_timeout: Duration::from_secs(5),
        restart_backoff: Duration::from_secs(1),
        restart_backoff_max: Duration::from_secs(1),
        max_restarts: 0,
        status_file: None,
    };
    health_check(&params).await
}

// ─────────────────────────────────────────────────────────────────────────────
// Version info
// ─────────────────────────────────────────────────────────────────────────────

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
