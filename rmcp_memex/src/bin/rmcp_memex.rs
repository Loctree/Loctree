use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

use rmcp_memex::{ServerConfig, WizardConfig, run_stdio_server, run_wizard};

fn parse_features(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn load_file_config(path: &str) -> Result<FileConfig> {
    let expanded = shellexpand::tilde(path).to_string();
    // This is the START of path validation - canonicalize resolves symlinks
    let canonical = std::path::Path::new(&expanded) // nosemgrep
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Cannot resolve config path '{}': {}", path, e))?;

    // Security: validate path is under home directory or current working directory
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok();
    let cwd = std::env::current_dir().ok();

    let is_safe = home
        .as_ref()
        .map(|h| canonical.starts_with(h))
        .unwrap_or(false)
        || cwd
            .as_ref()
            .map(|c| canonical.starts_with(c))
            .unwrap_or(false);

    if !is_safe {
        return Err(anyhow::anyhow!(
            "Access denied: config path '{}' is outside allowed directories",
            path
        ));
    }

    // Path is validated above: canonicalized + checked against HOME/CWD
    let contents = std::fs::read_to_string(&canonical)?; // nosemgrep
    toml::from_str(&contents).map_err(Into::into)
}

#[derive(serde::Deserialize, Default)]
struct FileConfig {
    mode: Option<String>,
    features: Option<String>,
    cache_mb: Option<usize>,
    db_path: Option<String>,
    max_request_bytes: Option<usize>,
    log_level: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "RAG/memory MCP server with LanceDB vector storage", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional config file (TOML) to load settings from; CLI flags override file when set.
    #[arg(long, global = true)]
    config: Option<String>,

    /// Server mode: "memory" (memory-only, no filesystem) or "full" (all features)
    #[arg(long, value_parser = ["memory", "full"], global = true)]
    mode: Option<String>,

    /// Enable specific features (comma-separated). Overrides --mode if set.
    #[arg(long, global = true)]
    features: Option<String>,

    /// Cache size in MB
    #[arg(long, global = true)]
    cache_mb: Option<usize>,

    /// Path for embedded vector store (LanceDB)
    #[arg(long, global = true)]
    db_path: Option<String>,

    /// Max allowed request size in bytes for JSON-RPC framing
    #[arg(long, global = true)]
    max_request_bytes: Option<usize>,

    /// Log level
    #[arg(long, global = true)]
    log_level: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the MCP server (default if no subcommand specified)
    Serve,

    /// Launch interactive configuration wizard
    Wizard {
        /// Dry run mode - show changes without writing files
        #[arg(long)]
        dry_run: bool,
    },
}

impl Cli {
    fn into_server_config(self) -> Result<ServerConfig> {
        let file_cfg = self
            .config
            .as_deref()
            .map(load_file_config)
            .transpose()?
            .unwrap_or_default();

        // Determine base config from mode (CLI > file > default)
        let mode = self.mode.as_deref().or(file_cfg.mode.as_deref());
        let base_cfg = match mode {
            Some("memory") => ServerConfig::for_memory_only(),
            Some("full") => ServerConfig::for_full_rag(),
            _ => ServerConfig::default(),
        };

        // CLI --features overrides mode-derived features
        let features = self
            .features
            .or(file_cfg.features)
            .map(|s| parse_features(&s))
            .unwrap_or(base_cfg.features);

        Ok(ServerConfig {
            features,
            cache_mb: self
                .cache_mb
                .or(file_cfg.cache_mb)
                .unwrap_or(base_cfg.cache_mb),
            db_path: self
                .db_path
                .or(file_cfg.db_path)
                .unwrap_or(base_cfg.db_path),
            max_request_bytes: self
                .max_request_bytes
                .or(file_cfg.max_request_bytes)
                .unwrap_or(base_cfg.max_request_bytes),
            log_level: self
                .log_level
                .or(file_cfg.log_level)
                .map(|s| parse_log_level(&s))
                .unwrap_or(base_cfg.log_level),
        })
    }
}

fn parse_log_level(level: &str) -> Level {
    match level.to_ascii_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Wizard { dry_run }) => {
            // Run TUI wizard (no logging setup needed - TUI handles terminal)
            let wizard_config = WizardConfig {
                config_path: cli.config,
                dry_run,
            };
            run_wizard(wizard_config)
        }
        Some(Commands::Serve) | None => {
            // Run MCP server
            let config = cli.into_server_config()?;

            // Send logs to stderr to keep stdout clean for JSON-RPC.
            let subscriber = FmtSubscriber::builder()
                .with_max_level(config.log_level)
                .with_writer(std::io::stderr)
                .with_ansi(false)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;

            info!("Starting RMCP Memex");
            info!("Features (informational): {:?}", config.features);
            info!("Cache: {}MB", config.cache_mb);
            info!("DB Path: {}", config.db_path);

            run_stdio_server(config).await
        }
    }
}
