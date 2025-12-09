use anyhow::{Result, anyhow};
use arrow_array::types::Float32Type;
use arrow_array::{
    FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{ArrowError, DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{Table, connect};
use moka::future::Cache;
use serde::Serialize;
use serde_json::{Value, json};
use sled::Db;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Schema version for LanceDB tables. Increment when changing table structure.
/// See docs/MIGRATION.md for migration procedures.
pub const SCHEMA_VERSION: u32 = 1;

// =============================================================================
// STORAGE BACKEND INTERFACE
// =============================================================================
//
// To add a new storage backend, implement a struct with the following methods:
//
//   async fn add_to_store(&self, documents: Vec<ChromaDocument>) -> Result<()>
//   async fn get_document(&self, namespace: &str, id: &str) -> Result<Option<ChromaDocument>>
//   async fn search(&self, namespace: Option<&str>, embedding: &[f32], k: usize) -> Result<Vec<ChromaDocument>>
//   async fn delete(&self, namespace: &str, id: &str) -> Result<usize>
//   async fn delete_namespace(&self, namespace: &str) -> Result<usize>
//
// Current implementation:
//   - `StorageManager`: LanceDB (vector store) + sled (KV) + moka (cache)
//
// Future alternatives to consider:
//   - Qdrant, Milvus, Pinecone (external vector DBs)
//   - SQLite with vector extension
// =============================================================================

#[derive(Debug, Serialize, Clone)]
pub struct ChromaDocument {
    pub id: String,
    pub namespace: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
    pub document: String,
}

pub struct StorageManager {
    cache: Arc<Cache<String, Vec<u8>>>,
    db: Db,
    lance: Connection,
    table: Arc<Mutex<Option<Table>>>,
    collection_name: String,
    lance_path: String,
}

type BatchIter =
    RecordBatchIterator<std::vec::IntoIter<std::result::Result<RecordBatch, ArrowError>>>;

impl StorageManager {
    pub async fn new(cache_mb: usize, db_path: &str) -> Result<Self> {
        // In-memory cache for misc K/V usage
        let cache_bytes = cache_mb * 1024 * 1024;
        let cache = Cache::builder()
            .max_capacity(cache_bytes as u64)
            .time_to_live(Duration::from_secs(3600))
            .build();

        // Persistent K/V for auxiliary state
        let sled_path = shellexpand::tilde("~/.rmcp_servers/sled").to_string();
        let db = sled::open(sled_path)?;

        // Embedded LanceDB path (expand ~, allow override via env)
        let lance_env = std::env::var("LANCEDB_PATH").unwrap_or_else(|_| db_path.to_string());
        let lance_path = if lance_env.trim().is_empty() {
            shellexpand::tilde("~/.rmcp_servers/rmcp_memex/lancedb").to_string()
        } else {
            shellexpand::tilde(&lance_env).to_string()
        };

        let lance = connect(&lance_path).execute().await?;

        Ok(Self {
            cache: Arc::new(cache),
            db,
            lance,
            table: Arc::new(Mutex::new(None)),
            collection_name: "mcp_documents".to_string(),
            lance_path,
        })
    }

    pub fn lance_path(&self) -> &str {
        &self.lance_path
    }

    pub async fn ensure_collection(&self) -> Result<()> {
        // Attempt to open; if missing, create empty table lazily on first add
        let mut guard = self.table.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        match self
            .lance
            .open_table(self.collection_name.as_str())
            .execute()
            .await
        {
            Ok(table) => {
                *guard = Some(table);
                info!("Found existing Lance table '{}'", self.collection_name);
            }
            Err(_) => {
                info!(
                    "Lance table '{}' will be created on first insert",
                    self.collection_name
                );
            }
        }
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        if let Some(value) = self.cache.get(key).await {
            return Ok(Some(value));
        }
        if let Some(value) = self.db.get(key)? {
            let vec = value.to_vec();
            self.cache.insert(key.to_string(), vec.clone()).await;
            return Ok(Some(vec));
        }
        Ok(None)
    }

    pub async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.cache.insert(key.to_string(), value.clone()).await;
        self.db.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    pub async fn add_to_store(&self, documents: Vec<ChromaDocument>) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }
        let dim = documents
            .first()
            .ok_or_else(|| anyhow!("No documents to add"))?
            .embedding
            .len();
        if dim == 0 {
            return Err(anyhow!("Embedding dimension is zero"));
        }

        let table = self.ensure_table(dim).await?;
        let batch = self.docs_to_batch(&documents, dim)?;
        table.add(batch).execute().await?;
        debug!("Inserted {} documents into Lance", documents.len());
        Ok(())
    }

    pub async fn search_store(
        &self,
        namespace: Option<&str>,
        embedding: Vec<f32>,
        k: usize,
    ) -> Result<Vec<ChromaDocument>> {
        if embedding.is_empty() {
            return Ok(vec![]);
        }
        let dim = embedding.len();
        let table = self.ensure_table(dim).await?;

        let mut query = table.query();
        if let Some(ns) = namespace {
            query = query.only_if(self.namespace_filter(ns).as_str());
        }
        let mut stream = query.nearest_to(embedding)?.limit(k).execute().await?;

        let mut results = Vec::new();
        while let Some(batch) = stream.try_next().await? {
            let mut docs = self.batch_to_docs(&batch)?;
            results.append(&mut docs);
        }
        debug!("Lance returned {} results", results.len());
        Ok(results)
    }

    pub async fn get_document(&self, namespace: &str, id: &str) -> Result<Option<ChromaDocument>> {
        let table = match self.ensure_table(0).await {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        let filter = format!(
            "{} AND {}",
            self.namespace_filter(namespace),
            self.id_filter(id)
        );
        let mut stream = table
            .query()
            .only_if(filter.as_str())
            .limit(1)
            .execute()
            .await?;
        if let Some(batch) = stream.try_next().await? {
            let mut docs = self.batch_to_docs(&batch)?;
            if let Some(doc) = docs.pop() {
                return Ok(Some(doc));
            }
        }
        Ok(None)
    }

    pub async fn delete_document(&self, namespace: &str, id: &str) -> Result<usize> {
        let table = match self.ensure_table(0).await {
            Ok(t) => t,
            Err(_) => return Ok(0),
        };
        let predicate = format!(
            "{} AND {}",
            self.namespace_filter(namespace),
            self.id_filter(id)
        );
        let deleted = table.delete(predicate.as_str()).await?;
        Ok(deleted.version as usize)
    }

    pub async fn purge_namespace(&self, namespace: &str) -> Result<usize> {
        let table = match self.ensure_table(0).await {
            Ok(t) => t,
            Err(_) => return Ok(0),
        };
        let predicate = self.namespace_filter(namespace);
        let deleted = table.delete(predicate.as_str()).await?;
        Ok(deleted.version as usize)
    }

    pub fn get_collection_name(&self) -> &str {
        &self.collection_name
    }

    async fn ensure_table(&self, dim: usize) -> Result<Table> {
        let mut guard = self.table.lock().await;
        if let Some(table) = guard.as_ref() {
            return Ok(table.clone());
        }

        let maybe_table = self
            .lance
            .open_table(self.collection_name.as_str())
            .execute()
            .await;

        let table = if let Ok(tbl) = maybe_table {
            tbl
        } else {
            if dim == 0 {
                return Err(anyhow!(
                    "Vector table '{}' not found and dimension is unknown",
                    self.collection_name
                ));
            }
            info!(
                "Creating Lance table '{}' with vector dimension {}",
                self.collection_name, dim
            );
            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("namespace", DataType::Utf8, false),
                Field::new(
                    "vector",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        dim as i32,
                    ),
                    false,
                ),
                Field::new("text", DataType::Utf8, true),
                Field::new("metadata", DataType::Utf8, true),
            ]));
            self.lance
                .create_empty_table(self.collection_name.as_str(), schema)
                .execute()
                .await?
        };

        *guard = Some(table.clone());
        Ok(table)
    }

    fn docs_to_batch(&self, documents: &[ChromaDocument], dim: usize) -> Result<BatchIter> {
        let ids = documents.iter().map(|d| d.id.as_str()).collect::<Vec<_>>();
        let namespaces = documents
            .iter()
            .map(|d| d.namespace.as_str())
            .collect::<Vec<_>>();
        let texts = documents
            .iter()
            .map(|d| d.document.as_str())
            .collect::<Vec<_>>();
        let metadata_strings = documents
            .iter()
            .map(|d| serde_json::to_string(&d.metadata).unwrap_or_else(|_| "{}".to_string()))
            .collect::<Vec<_>>();

        let vectors = documents.iter().map(|d| {
            if d.embedding.len() != dim {
                None
            } else {
                Some(d.embedding.iter().map(|v| Some(*v)).collect::<Vec<_>>())
            }
        });

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("namespace", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dim as i32,
                ),
                false,
            ),
            Field::new("text", DataType::Utf8, true),
            Field::new("metadata", DataType::Utf8, true),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(StringArray::from(namespaces)),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vectors, dim as i32,
                    ),
                ),
                Arc::new(StringArray::from(texts)),
                Arc::new(StringArray::from(metadata_strings)),
            ],
        )?;

        Ok(RecordBatchIterator::new(
            vec![Ok(batch)].into_iter(),
            schema,
        ))
    }

    fn batch_to_docs(&self, batch: &RecordBatch) -> Result<Vec<ChromaDocument>> {
        let id_col = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| anyhow!("Missing id column"))?;
        let ns_col = batch
            .column_by_name("namespace")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| anyhow!("Missing namespace column"))?;
        let text_col = batch
            .column_by_name("text")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| anyhow!("Missing text column"))?;
        let metadata_col = batch
            .column_by_name("metadata")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| anyhow!("Missing metadata column"))?;
        let vector_col = batch
            .column_by_name("vector")
            .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
            .ok_or_else(|| anyhow!("Missing vector column"))?;

        let dim = vector_col.value_length() as usize;
        let values = vector_col
            .values()
            .as_any()
            .downcast_ref::<Float32Array>()
            .ok_or_else(|| anyhow!("Vector inner type mismatch"))?;

        let mut docs = Vec::new();
        for i in 0..batch.num_rows() {
            let id = id_col.value(i).to_string();
            let text = text_col.value(i).to_string();
            let namespace = ns_col.value(i).to_string();
            let meta_str = metadata_col.value(i);
            let metadata: Value = serde_json::from_str(meta_str).unwrap_or_else(|_| json!({}));

            let offset = i * dim;
            let mut emb = Vec::with_capacity(dim);
            for j in 0..dim {
                emb.push(values.value(offset + j));
            }

            docs.push(ChromaDocument {
                id,
                namespace,
                embedding: emb,
                metadata,
                document: text,
            });
        }
        Ok(docs)
    }

    fn namespace_filter(&self, namespace: &str) -> String {
        format!("namespace = '{}'", namespace.replace('\'', "''"))
    }

    fn id_filter(&self, id: &str) -> String {
        format!("id = '{}'", id.replace('\'', "''"))
    }
}
