use anyhow::{Result, anyhow};
use fastembed::{TextEmbedding, TextInitOptions};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tokio::sync::Mutex;

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct RerankRequest {
    query: String,
    documents: Vec<String>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct RerankResponse {
    results: Vec<RerankResult>,
}

#[derive(Debug, Deserialize)]
struct RerankResult {
    index: usize,
    score: f32,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: String,
}

// =============================================================================
// EMBEDDING BACKEND INTERFACE
// =============================================================================
//
// To add a new embedding backend, implement a struct with the following methods:
//
//   async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>
//   async fn embed(&self, text: &str) -> Result<Vec<f32>>
//
// Current implementations:
//   - `FastEmbedder`: Local embeddings via fastembed (default, 384 dims)
//   - `MLXBridge`: Remote embeddings via MLX HTTP server (Apple Silicon)
//
// Future: Consider adding `async_trait` crate for formal trait definition.
// =============================================================================

pub struct MLXBridge {
    client: Client,
    embedder_url: String,
    reranker_url: String,
    embedder_model: String,
    reranker_model: String,
}

pub struct FastEmbedder {
    model: Mutex<TextEmbedding>,
}

impl FastEmbedder {
    pub fn new() -> Result<Self> {
        // Default fastembed cache to ~/.cache/fastembed unless user overrides via env.
        let cache_dir = std::env::var("FASTEMBED_CACHE_PATH")
            .or_else(|_| std::env::var("HF_HUB_CACHE"))
            .unwrap_or_else(|_| {
                let base = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                Path::new(&base)
                    .join(".cache/fastembed")
                    .to_string_lossy()
                    .into_owned()
            });

        // SAFETY: These env vars are set once during initialization, before any
        // multi-threaded operations begin. The fastembed/HF libraries read these
        // vars to determine cache locations.
        if std::env::var("FASTEMBED_CACHE_PATH").is_err() {
            unsafe { std::env::set_var("FASTEMBED_CACHE_PATH", &cache_dir) };
        }
        if std::env::var("HF_HUB_CACHE").is_err() {
            unsafe { std::env::set_var("HF_HUB_CACHE", &cache_dir) };
        }
        fs::create_dir_all(&cache_dir)?;

        let model = TextEmbedding::try_new(TextInitOptions::default())?;
        Ok(Self {
            model: Mutex::new(model),
        })
    }

    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut model = self.model.lock().await;
        let embeddings = model.embed(texts, None)?;
        Ok(embeddings)
    }
}

impl MLXBridge {
    pub async fn new() -> Result<Self> {
        if std::env::var("DISABLE_MLX").is_ok() {
            return Err(anyhow!("MLX bridge disabled via DISABLE_MLX"));
        }

        // Base URL for the MLX HTTP bridge. Default to localhost so it's meaningful for most users.
        let dragon_base =
            std::env::var("DRAGON_BASE_URL").unwrap_or_else(|_| "http://localhost".to_string());

        // Check if JIT mode (single port for all models)
        let jit_mode =
            std::env::var("MLX_JIT_MODE").unwrap_or_else(|_| "false".to_string()) == "true";

        let (embedder_port, reranker_port) = if jit_mode {
            // JIT mode - same port for both
            let port = std::env::var("MLX_JIT_PORT").unwrap_or_else(|_| "1234".to_string());
            (port.clone(), port)
        } else {
            // Separate ports mode
            let embedder_port =
                std::env::var("EMBEDDER_PORT").unwrap_or_else(|_| "12345".to_string());
            let reranker_port =
                std::env::var("RERANKER_PORT").unwrap_or_else(|_| "12346".to_string());
            (embedder_port, reranker_port)
        };

        let embedder_model = std::env::var("EMBEDDER_MODEL")
            .unwrap_or_else(|_| "Qwen/Qwen3-Embedding-4B".to_string());
        let reranker_model = std::env::var("RERANKER_MODEL")
            .unwrap_or_else(|_| "Qwen/Qwen3-Reranker-4B".to_string());

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let bridge = Self {
            client,
            embedder_url: format!("{}:{}/v1/embeddings", dragon_base, embedder_port),
            reranker_url: format!("{}:{}/v1/rerank", dragon_base, reranker_port),
            embedder_model,
            reranker_model,
        };

        // List available models
        if let Ok(models) = bridge.list_models(&dragon_base, &embedder_port).await {
            tracing::info!(
                "Available models on embedder port {}: {:?}",
                embedder_port,
                models
            );
        }

        if !jit_mode && let Ok(models) = bridge.list_models(&dragon_base, &reranker_port).await {
            tracing::info!(
                "Available models on reranker port {}: {:?}",
                reranker_port,
                models
            );
        }

        Ok(bridge)
    }

    async fn list_models(&self, base_url: &str, port: &str) -> Result<Vec<String>> {
        let url = format!("{}:{}/v1/models", base_url, port);
        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<ModelsResponse>()
            .await?;

        Ok(response.data.into_iter().map(|m| m.id).collect())
    }

    pub async fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            input: vec![text.to_string()],
            model: self.embedder_model.clone(),
        };

        let response = self
            .client
            .post(&self.embedder_url)
            .json(&request)
            .send()
            .await?
            .json::<EmbeddingResponse>()
            .await?;

        response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow!("No embedding returned"))
    }

    pub async fn rerank(&mut self, query: &str, documents: &[String]) -> Result<Vec<(usize, f32)>> {
        let request = RerankRequest {
            query: query.to_string(),
            documents: documents.to_vec(),
            model: self.reranker_model.clone(),
        };

        let response = self
            .client
            .post(&self.reranker_url)
            .json(&request)
            .send()
            .await?
            .json::<RerankResponse>()
            .await?;

        Ok(response
            .results
            .into_iter()
            .map(|r| (r.index, r.score))
            .collect())
    }
}
