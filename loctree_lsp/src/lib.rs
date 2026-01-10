//! Loctree Language Server Protocol implementation
//!
//! Provides IDE integration for dead code detection, cycles, and navigation.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use tower_lsp::{LspService, Server};

mod actions;
mod backend;
mod diagnostics;
mod hover;
mod navigation;
mod references;
mod snapshot;

pub use backend::Backend;
pub use snapshot::SnapshotState;

/// Run the LSP server over stdio
pub async fn run_server() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
