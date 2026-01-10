//! Loctree LSP Server binary entry point
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use loctree_lsp::run_server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    if let Err(e) = run_server().await {
        tracing::error!("LSP server error: {}", e);
        std::process::exit(1);
    }
}
