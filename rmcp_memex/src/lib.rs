pub mod embeddings;
pub mod handlers;
pub mod rag;
pub mod storage;
pub mod tui;

use anyhow::Result;
use tracing::Level;

// Re-export core types for library consumers
pub use embeddings::{FastEmbedder, MLXBridge};
pub use handlers::{MCPServer, create_server};
pub use rag::{RAGPipeline, SearchResult};
pub use storage::{ChromaDocument, StorageManager};
pub use tui::{HostDetection, HostKind, WizardConfig, detect_hosts, run_wizard};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Enabled features (namespaced strings)
    pub features: Vec<String>,

    /// Cache size in MB for sled/moka
    pub cache_mb: usize,

    /// Path for embedded vector store (LanceDB)
    pub db_path: String,

    /// Max allowed request size (bytes) for JSON-RPC framing
    pub max_request_bytes: usize,

    /// Default log level to use when wiring tracing
    pub log_level: Level,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            features: vec![
                "filesystem".to_string(),
                "memory".to_string(),
                "search".to_string(),
            ],
            cache_mb: 4096,
            db_path: "~/.rmcp_servers/rmcp_memex/lancedb".to_string(),
            max_request_bytes: 5 * 1024 * 1024,
            log_level: Level::INFO,
        }
    }
}

impl ServerConfig {
    /// Create a memory-only configuration (no filesystem access).
    /// Suitable for pure vector memory server use cases.
    pub fn for_memory_only() -> Self {
        Self {
            features: vec!["memory".to_string(), "search".to_string()],
            ..Self::default()
        }
    }

    /// Create a full RAG configuration with all features enabled.
    pub fn for_full_rag() -> Self {
        Self::default()
    }

    pub fn with_db_path(mut self, db_path: impl Into<String>) -> Self {
        self.db_path = db_path.into();
        self
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
}

/// Helper to build and run the stdin/stdout server for library consumers.
pub async fn run_stdio_server(config: ServerConfig) -> Result<()> {
    let server = create_server(config).await?;
    server.run_stdio().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = ServerConfig::default();
        assert!(cfg.features.contains(&"filesystem".to_string()));
        assert_eq!(cfg.cache_mb, 4096);
        assert_eq!(cfg.db_path, "~/.rmcp_servers/rmcp_memex/lancedb");
        assert_eq!(cfg.max_request_bytes, 5 * 1024 * 1024);
    }
}
