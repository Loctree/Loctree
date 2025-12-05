use anyhow::Result;
use pdf_extract;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    embeddings::{FastEmbedder, MLXBridge},
    storage::{ChromaDocument, StorageManager},
};

const DEFAULT_NAMESPACE: &str = "rag";

pub struct RAGPipeline {
    mlx_bridge: Arc<Mutex<Option<MLXBridge>>>,
    storage: Arc<StorageManager>,
    fast: Arc<FastEmbedder>,
}

impl RAGPipeline {
    pub async fn new(
        mlx_bridge: Arc<Mutex<Option<MLXBridge>>>,
        storage: Arc<StorageManager>,
    ) -> Result<Self> {
        let fast = Arc::new(FastEmbedder::new()?);
        Ok(Self {
            mlx_bridge,
            storage,
            fast,
        })
    }

    pub fn storage(&self) -> Arc<StorageManager> {
        self.storage.clone()
    }

    pub async fn has_mlx(&self) -> bool {
        self.mlx_bridge.lock().await.is_some()
    }

    pub async fn index_document(&self, path: &Path, namespace: Option<&str>) -> Result<()> {
        let text = self.extract_text(path).await?;

        // Chunk the text
        let chunks = self.chunk_text(&text, 512, 128)?;

        // Embed chunks
        let embeddings = self.embed_chunks(&chunks).await?;

        let ns = namespace.unwrap_or(DEFAULT_NAMESPACE);
        let mut documents = Vec::new();
        for (i, (chunk, embedding)) in chunks.iter().zip(embeddings.iter()).enumerate() {
            let doc = ChromaDocument {
                id: format!("{}_{}", path.to_str().unwrap_or("unknown"), i),
                namespace: ns.to_string(),
                embedding: embedding.clone(),
                metadata: json!({
                    "path": path.to_str(),
                    "chunk_index": i,
                    "total_chunks": chunks.len(),
                }),
                document: chunk.clone(),
            };
            documents.push(doc);
        }

        self.storage.add_to_store(documents).await?;

        Ok(())
    }

    pub async fn index_text(
        &self,
        namespace: Option<&str>,
        id: String,
        text: String,
        metadata: serde_json::Value,
    ) -> Result<String> {
        let ns = namespace.unwrap_or(DEFAULT_NAMESPACE).to_string();
        let embedding = self.embed_query(&text).await?;
        let doc = ChromaDocument {
            id: id.clone(),
            namespace: ns,
            embedding,
            metadata,
            document: text,
        };
        self.storage.add_to_store(vec![doc]).await?;
        Ok(id)
    }

    pub async fn memory_upsert(
        &self,
        namespace: &str,
        id: String,
        text: String,
        metadata: serde_json::Value,
    ) -> Result<()> {
        self.index_text(Some(namespace), id, text, metadata).await?;
        Ok(())
    }

    pub async fn memory_get(&self, namespace: &str, id: &str) -> Result<Option<SearchResult>> {
        if let Some(doc) = self.storage.get_document(namespace, id).await? {
            return Ok(Some(SearchResult {
                id: doc.id,
                namespace: doc.namespace,
                text: doc.document,
                score: 1.0,
                metadata: doc.metadata,
            }));
        }
        Ok(None)
    }

    pub async fn memory_delete(&self, namespace: &str, id: &str) -> Result<usize> {
        self.storage.delete_document(namespace, id).await
    }

    pub async fn purge_namespace(&self, namespace: &str) -> Result<usize> {
        self.storage.purge_namespace(namespace).await
    }

    pub async fn memory_search(
        &self,
        namespace: &str,
        query: &str,
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search_inner(Some(namespace), query, k).await
    }

    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        self.search_inner(None, query, k).await
    }

    pub async fn search_inner(
        &self,
        namespace: Option<&str>,
        query: &str,
        k: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_embedding = self.embed_query(query).await?;

        let candidates = self
            .storage
            .search_store(namespace, query_embedding.clone(), k * 3)
            .await?;

        // Rerank if we have candidates
        if !candidates.is_empty() {
            let documents: Vec<String> = candidates.iter().map(|c| c.document.clone()).collect();
            let metadatas: Vec<serde_json::Value> =
                candidates.iter().map(|c| c.metadata.clone()).collect();

            // Try MLX reranker; fallback to cosine if unavailable
            let reranked = if let Some(mlx) = self.mlx_bridge.lock().await.as_mut() {
                match mlx.rerank(query, &documents).await {
                    Ok(r) => Some(r),
                    Err(e) => {
                        tracing::warn!("MLX rerank failed, using cosine fallback: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            let reranked = if let Some(r) = reranked {
                r
            } else {
                // Cosine fallback
                let doc_embeddings = self.ensure_doc_embeddings(&documents, &candidates).await?;
                let scores = doc_embeddings
                    .iter()
                    .enumerate()
                    .map(|(idx, emb)| (idx, cosine(&query_embedding, emb)))
                    .collect::<Vec<_>>();
                let mut scores = scores;
                scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                scores
            };

            // Return top-k reranked results
            let results: Vec<SearchResult> = reranked
                .into_iter()
                .take(k)
                .filter_map(|(idx, score)| {
                    documents.get(idx).map(|text| SearchResult {
                        id: candidates
                            .get(idx)
                            .map(|c| c.id.clone())
                            .unwrap_or_default(),
                        namespace: candidates
                            .get(idx)
                            .map(|c| c.namespace.clone())
                            .unwrap_or_else(|| namespace.unwrap_or(DEFAULT_NAMESPACE).to_string()),
                        text: text.clone(),
                        score,
                        metadata: metadatas.get(idx).cloned().unwrap_or_else(|| json!({})),
                    })
                })
                .collect();

            return Ok(results);
        }

        Ok(vec![])
    }

    async fn extract_text(&self, path: &Path) -> Result<String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "pdf" {
            // pdf_extract is blocking; offload to blocking thread
            let path = path.to_path_buf();
            let pdf_text =
                tokio::task::spawn_blocking(move || pdf_extract::extract_text(&path)).await??;
            return Ok(pdf_text);
        }

        // Default: treat as UTF-8 text
        // Path is validated by caller (handlers::validate_path) before reaching this private method
        // nosemgrep: rust.actix.path-traversal.tainted-path.tainted-path
        tokio::fs::read_to_string(path).await.map_err(|e| e.into())
    }

    async fn embed_chunks(&self, chunks: &[String]) -> Result<Vec<Vec<f32>>> {
        // Try MLX first
        if let Some(mlx) = self.mlx_bridge.lock().await.as_mut() {
            let mut out = Vec::with_capacity(chunks.len());
            for chunk in chunks {
                out.push(mlx.embed(chunk).await?);
            }
            return Ok(out);
        }

        // Fallback to fastembed batch
        self.fast.embed_batch(chunks).await
    }

    async fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        if let Some(mlx) = self.mlx_bridge.lock().await.as_mut() {
            return mlx.embed(query).await;
        }
        let emb = self.fast.embed_batch(&[query.to_string()]).await?;
        Ok(emb.into_iter().next().unwrap_or_default())
    }

    async fn ensure_doc_embeddings(
        &self,
        documents: &[String],
        candidates: &[ChromaDocument],
    ) -> Result<Vec<Vec<f32>>> {
        // If Chroma returned embeddings, use them; otherwise embed locally
        let has_all = candidates.iter().all(|c| !c.embedding.is_empty());
        if has_all {
            return Ok(candidates.iter().map(|c| c.embedding.clone()).collect());
        }

        self.fast.embed_batch(documents).await
    }

    fn chunk_text(&self, text: &str, chunk_size: usize, overlap: usize) -> Result<Vec<String>> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();

        let mut start = 0;
        while start < chars.len() {
            let end = (start + chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            if end >= chars.len() {
                break;
            }

            start = end - overlap;
        }

        Ok(chunks)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub id: String,
    pub namespace: String,
    pub text: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}
