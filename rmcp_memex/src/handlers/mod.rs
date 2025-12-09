use anyhow::{Result, anyhow};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{ServerConfig, embeddings::MLXBridge, rag::RAGPipeline, storage::StorageManager};

/// Validates a file path to prevent path traversal attacks.
/// Returns the canonicalized path if valid, or an error if the path is unsafe.
fn validate_path(path_str: &str) -> Result<std::path::PathBuf> {
    if path_str.is_empty() {
        return Err(anyhow!("Path cannot be empty"));
    }

    // Expand ~ to home directory
    let expanded = shellexpand::tilde(path_str).to_string();
    // This IS the path validation/sanitization function - not a vulnerability
    // nosemgrep: rust.actix.path-traversal.tainted-path.tainted-path
    let path = Path::new(&expanded);

    // Check for obvious path traversal patterns before canonicalization
    let path_string = path_str.to_string();
    if path_string.contains("..") {
        return Err(anyhow!("Path traversal detected: '..' not allowed"));
    }

    // Canonicalize to resolve symlinks and get absolute path
    let canonical = path
        .canonicalize()
        .map_err(|e| anyhow!("Cannot resolve path '{}': {}", path_str, e))?;

    // Get user's home directory as safe base
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok();

    // Allow paths under home directory or current working directory
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
        return Err(anyhow!(
            "Access denied: path '{}' is outside allowed directories",
            path_str
        ));
    }

    Ok(canonical)
}

pub struct MCPServer {
    rag: Arc<RAGPipeline>,
    max_request_bytes: usize,
}

impl MCPServer {
    pub async fn run_stdio(self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        // Read newline-delimited JSON-RPC (standard MCP transport)
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue; // Skip empty lines
            }

            // Check size limit
            if trimmed.len() > self.max_request_bytes {
                let err = json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32600, "message": format!("Request too large: {} bytes (max {})", trimmed.len(), self.max_request_bytes)},
                    "id": serde_json::Value::Null
                });
                let payload = serde_json::to_string(&err)?;
                stdout.write_all(payload.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }

            let request: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(req) => req,
                Err(e) => {
                    let err = json!({
                        "jsonrpc": "2.0",
                        "error": {"code": -32700, "message": format!("Parse error: {}", e)},
                        "id": serde_json::Value::Null
                    });
                    let payload = serde_json::to_string(&err)?;
                    stdout.write_all(payload.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            let response = self.handle_request(request).await;
            let payload = serde_json::to_string(&response)?;
            stdout.write_all(payload.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        self.run_stdio().await
    }

    pub async fn handle_request(&self, request: serde_json::Value) -> serde_json::Value {
        let method = request["method"].as_str().unwrap_or("");
        let id = request["id"].clone();

        let result = match method {
            "initialize" => json!({
                "protocolVersion": "1.0",
                "serverInfo": {
                    "name": "rmcp_memex",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": true,
                    "resources": true,
                }
            }),

            "tools/list" => json!({
                "tools": [
                    {
                        "name": "health",
                        "description": "Health/status of rmcp_memex server",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "rag_index",
                        "description": "Index a document for RAG",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": {"type": "string"},
                                "namespace": {"type": "string"}
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "rag_index_text",
                        "description": "Index raw text for RAG/memory",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "text": {"type": "string"},
                                "id": {"type": "string"},
                                "namespace": {"type": "string"},
                                "metadata": {"type": "object"}
                            },
                            "required": ["text"]
                        }
                    },
                    {
                        "name": "rag_search",
                        "description": "Search documents using RAG",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {"type": "string"},
                                "k": {"type": "integer", "default": 10},
                                "namespace": {"type": "string"}
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "memory_upsert",
                        "description": "Upsert a text chunk into vector memory",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": {"type": "string"},
                                "id": {"type": "string"},
                                "text": {"type": "string"},
                                "metadata": {"type": "object"}
                            },
                            "required": ["namespace", "id", "text"]
                        }
                    },
                    {
                        "name": "memory_get",
                        "description": "Get a stored chunk by namespace + id",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": {"type": "string"},
                                "id": {"type": "string"}
                            },
                            "required": ["namespace", "id"]
                        }
                    },
                    {
                        "name": "memory_search",
                        "description": "Semantic search within a namespace",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": {"type": "string"},
                                "query": {"type": "string"},
                                "k": {"type": "integer", "default": 5}
                            },
                            "required": ["namespace", "query"]
                        }
                    },
                    {
                        "name": "memory_delete",
                        "description": "Delete a chunk by namespace + id",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": {"type": "string"},
                                "id": {"type": "string"}
                            },
                            "required": ["namespace", "id"]
                        }
                    },
                    {
                        "name": "memory_purge_namespace",
                        "description": "Delete all chunks in a namespace",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "namespace": {"type": "string"}
                            },
                            "required": ["namespace"]
                        }
                    }
                ]
            }),

            "tools/call" => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("");
                let args = &request["params"]["arguments"];

                match tool_name {
                    "health" => {
                        let status = json!({
                            "version": env!("CARGO_PKG_VERSION"),
                            "db_path": self.rag.storage().lance_path(),
                            "cache_dir": std::env::var("FASTEMBED_CACHE_PATH")
                                .or_else(|_| std::env::var("HF_HUB_CACHE"))
                                .unwrap_or_else(|_| "not-set".to_string()),
                            "backend": if self.rag.has_mlx().await { "mlx" } else { "fastembed" }
                        });
                        json!({
                            "content": [{"type": "text", "text": serde_json::to_string(&status).unwrap_or_default()}]
                        })
                    }
                    "rag_index" => {
                        let path_str = args["path"].as_str().unwrap_or("");
                        let namespace = args["namespace"].as_str();

                        // Validate path to prevent path traversal attacks
                        let validated_path = match validate_path(path_str) {
                            Ok(p) => p,
                            Err(e) => {
                                return json!({
                                    "jsonrpc": "2.0",
                                    "error": {"code": -32602, "message": e.to_string()},
                                    "id": id
                                });
                            }
                        };

                        match self.rag.index_document(&validated_path, namespace).await {
                            Ok(_) => json!({
                                "content": [{"type": "text", "text": format!("Indexed: {}", path_str)}]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    "rag_index_text" => {
                        let text = args["text"].as_str().unwrap_or("").to_string();
                        let namespace = args["namespace"].as_str();
                        let metadata = args.get("metadata").cloned().unwrap_or_else(|| json!({}));
                        let id = args
                            .get("id")
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| Uuid::new_v4().to_string());

                        match self
                            .rag
                            .index_text(namespace, id.clone(), text, metadata)
                            .await
                        {
                            Ok(returned_id) => json!({
                                "content": [{"type": "text", "text": format!("Indexed text with id {}", returned_id)}]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    "rag_search" => {
                        let query = args["query"].as_str().unwrap_or("");
                        let k = args["k"].as_u64().unwrap_or(10) as usize;
                        let namespace = args["namespace"].as_str();

                        match self.rag.search_inner(namespace, query, k).await {
                            Ok(results) => json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string(&results).unwrap_or_default()
                                }]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    "memory_upsert" => {
                        let namespace = args["namespace"].as_str().unwrap_or("default");
                        let id = args["id"].as_str().unwrap_or("").to_string();
                        let text = args["text"].as_str().unwrap_or("").to_string();
                        let metadata = args.get("metadata").cloned().unwrap_or_else(|| json!({}));

                        match self
                            .rag
                            .memory_upsert(namespace, id.clone(), text, metadata)
                            .await
                        {
                            Ok(_) => json!({
                                "content": [{"type": "text", "text": format!("Upserted {}", id)}]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    "memory_get" => {
                        let namespace = args["namespace"].as_str().unwrap_or("default");
                        let id = args["id"].as_str().unwrap_or("");
                        match self.rag.memory_get(namespace, id).await {
                            Ok(Some(doc)) => json!({
                                "content": [{"type": "text", "text": serde_json::to_string(&doc).unwrap_or_default()}]
                            }),
                            Ok(None) => json!({
                                "content": [{"type": "text", "text": "Not found"}]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    "memory_search" => {
                        let namespace = args["namespace"].as_str().unwrap_or("default");
                        let query = args["query"].as_str().unwrap_or("");
                        let k = args["k"].as_u64().unwrap_or(5) as usize;
                        match self.rag.memory_search(namespace, query, k).await {
                            Ok(results) => json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string(&results).unwrap_or_default()
                                }]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    "memory_delete" => {
                        let namespace = args["namespace"].as_str().unwrap_or("default");
                        let id = args["id"].as_str().unwrap_or("");
                        match self.rag.memory_delete(namespace, id).await {
                            Ok(deleted) => json!({
                                "content": [{"type": "text", "text": format!("Deleted {} rows", deleted)}]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    "memory_purge_namespace" => {
                        let namespace = args["namespace"].as_str().unwrap_or("default");
                        match self.rag.purge_namespace(namespace).await {
                            Ok(deleted) => json!({
                                "content": [{"type": "text", "text": format!("Purged namespace '{}', removed {} rows", namespace, deleted)}]
                            }),
                            Err(e) => json!({
                                "error": {"message": e.to_string()}
                            }),
                        }
                    }
                    _ => json!({"error": {"message": "Unknown tool"}}),
                }
            }

            _ => json!({"error": {"message": "Unknown method"}}),
        };

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    }
}

pub async fn create_server(config: ServerConfig) -> Result<MCPServer> {
    // Initialize components
    let mlx_bridge = match MLXBridge::new().await {
        Ok(mlx) => Some(mlx),
        Err(e) => {
            tracing::warn!(
                "MLX bridge unavailable, falling back to fastembed only: {}",
                e
            );
            None
        }
    };
    let mlx_bridge = Arc::new(Mutex::new(mlx_bridge));
    let db_path = shellexpand::tilde(&config.db_path).to_string();
    let storage = Arc::new(StorageManager::new(config.cache_mb, &db_path).await?);
    storage.ensure_collection().await?;
    let rag = Arc::new(RAGPipeline::new(mlx_bridge, storage).await?);

    Ok(MCPServer {
        rag,
        max_request_bytes: config.max_request_bytes,
    })
}
