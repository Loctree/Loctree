//! Memex module - AI Memory integration for Loctree.
//!
//! This module bridges static code analysis (Loctree) and AI memory systems (RMCP Memex).
//! It ingests structural analysis reports, converts code insights into semantic vectors,
//! and stores them in a local vector database for AI agent queries.

use anyhow::{Context, Result};
use chrono::Utc;
use rmcp_memex::embeddings::{EmbeddingClient, EmbeddingConfig};
use rmcp_memex::storage::{ChromaDocument, StorageManager};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::cli::MemexOptions;

// --- Data Structures for analysis.json ---

/// Root structure of the Loctree analysis JSON report.
#[derive(Debug, Deserialize)]
struct AnalysisReport {
    /// List of analysis runs (typically contains one latest run).
    analysis: Vec<AnalysisRun>,
}

/// A single execution run of the analysis.
#[derive(Debug, Deserialize)]
struct AnalysisRun {
    /// AI-optimized views of the codebase data.
    #[serde(rename = "aiViews")]
    ai_views: AiViews,
}

/// Aggregated insights designed for AI consumption.
#[derive(Debug, Deserialize)]
struct AiViews {
    /// Detected symbols that are exported but never used.
    #[serde(rename = "deadSymbols")]
    dead_symbols: Option<Vec<DeadSymbol>>,

    /// Summary of Code Intelligence metrics (e.g., duplication).
    #[serde(rename = "ciSummary")]
    ci_summary: Option<CiSummary>,
}

/// Represents a symbol (function, class, variable) that appears unused.
#[derive(Debug, Deserialize)]
struct DeadSymbol {
    /// The name of the unused symbol.
    name: String,
    /// File paths where the symbol is defined.
    paths: Vec<String>,
    /// Whether the symbol is exported publicly.
    #[serde(rename = "publicSurface")]
    public_surface: bool,
}

/// Summary of code duplication metrics.
#[derive(Debug, Deserialize)]
struct CiSummary {
    /// Total number of duplication clusters found.
    #[serde(rename = "duplicateClustersCount")]
    duplicate_count: usize,

    /// List of the most significant duplication clusters.
    #[serde(rename = "topClusters")]
    top_clusters: Option<Vec<DuplicateCluster>>,
}

/// A cluster of duplicated code.
#[derive(Debug, Deserialize)]
struct DuplicateCluster {
    /// The name of the duplicated symbol.
    #[serde(rename = "symbolName")]
    symbol_name: String,
    /// Number of occurrences.
    size: usize,
    /// Severity level ("high", "medium", "low").
    severity: String,
}

// --- Main Application Logic ---

/// Run the memex indexer with the given options.
///
/// Returns the number of documents indexed on success.
pub async fn run_memex(
    opts: &MemexOptions,
    json_output: bool,
    verbose: bool,
) -> Result<usize, String> {
    run_memex_inner(opts, json_output, verbose)
        .await
        .map_err(|e| e.to_string())
}

async fn run_memex_inner(opts: &MemexOptions, _json_output: bool, verbose: bool) -> Result<usize> {
    // 1. Resolve Analysis File Path
    let analysis_path = if opts.report_path.is_dir() {
        find_analysis_json(&opts.report_path)?
    } else {
        opts.report_path.clone()
    };

    // Canonicalize path to prevent path traversal attacks
    let analysis_path = analysis_path
        .canonicalize()
        .context("Failed to resolve analysis path - file may not exist or path is invalid")?;

    if verbose {
        eprintln!("[memex] Reading analysis report from: {:?}", analysis_path);
    }

    // 2. Load and Parse Data
    let content = fs::read_to_string(&analysis_path).context("Failed to read analysis.json")?;
    let report: AnalysisReport =
        serde_json::from_str(&content).context("Failed to parse analysis.json")?;

    let project_id = opts.project_id.as_deref().unwrap_or("unknown_project");

    // 3. Prepare Text Documents for Embedding
    let raw_docs = prepare_documents(&report, project_id);

    if raw_docs.is_empty() {
        if verbose {
            eprintln!("[memex] No indexable content found in analysis report.");
        }
        return Ok(0);
    }

    if verbose {
        eprintln!("[memex] Found {} items to index.", raw_docs.len());
    }

    // 4. Initialize AI Components (Embedder + Vector DB)
    if verbose {
        eprintln!("[memex] Initializing embedding model...");
    }
    let mut embedder = EmbeddingClient::new(&EmbeddingConfig::default())
        .await
        .context("Failed to initialize embedding client")?;

    let db_path = opts
        .db_path
        .clone()
        .unwrap_or_else(|| "~/.rmcp_servers/rmcp_memex/lancedb".to_string());

    if verbose {
        eprintln!("[memex] Opening vector storage at: {}", db_path);
    }
    // 512MB cache for LanceDB
    let storage = StorageManager::new(512, &db_path)
        .await
        .context("Failed to open storage")?;

    // 5. Generate Embeddings (Batch Processing)
    let texts: Vec<String> = raw_docs.iter().map(|(_, text)| text.clone()).collect();

    if verbose {
        eprintln!(
            "[memex] Generating embeddings for {} documents...",
            texts.len()
        );
    }
    let embeddings = embedder
        .embed_batch(&texts)
        .await
        .context("Failed to generate embeddings")?;

    // 6. Construct ChromaDocuments with Metadata
    let mut chroma_docs = Vec::new();
    for (i, (metadata_str, text)) in raw_docs.into_iter().enumerate() {
        let embedding = embeddings[i].clone();

        // Convert our internal metadata string back to JSON
        let meta_json = parse_metadata_string(&metadata_str);

        chroma_docs.push(ChromaDocument::new_flat(
            Uuid::new_v4().to_string(),
            opts.namespace.clone(),
            embedding,
            Value::Object(meta_json),
            text,
        ));
    }

    let doc_count = chroma_docs.len();

    // 7. Persist to Vector Database
    if verbose {
        eprintln!("[memex] Writing to storage...");
    }
    storage
        .add_to_store(chroma_docs)
        .await
        .context("Failed to save documents to LanceDB")?;

    Ok(doc_count)
}

/// Recursively searches for `analysis.json` in a given directory.
fn find_analysis_json(path: &Path) -> Result<PathBuf> {
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_name() == "analysis.json" {
            return Ok(entry.path().to_path_buf());
        }
    }
    anyhow::bail!("No analysis.json found in {:?}", path)
}

/// Helper to parse "key:val|key2:val2" metadata string into a JSON Map.
fn parse_metadata_string(meta_str: &str) -> Map<String, Value> {
    let mut map = Map::new();
    for part in meta_str.split('|') {
        if let Some((k, v)) = part.split_once(':') {
            map.insert(k.to_string(), Value::String(v.to_string()));
        }
    }
    map.insert(
        "indexed_at".to_string(),
        Value::String(Utc::now().to_rfc3339()),
    );
    map
}

/// Converts the structured analysis report into a list of (MetadataString, TextContent) tuples.
fn prepare_documents(report: &AnalysisReport, project_id: &str) -> Vec<(String, String)> {
    let mut docs = Vec::new();

    for run in &report.analysis {
        // --- Process Dead Symbols ---
        if let Some(dead_symbols) = &run.ai_views.dead_symbols {
            for dead in dead_symbols {
                let context = format!(
                    "Dead Code Detection: Symbol '{}' is defined in {:?} but appears unused. Public Surface: {}.",
                    dead.name, dead.paths, dead.public_surface
                );

                let metadata =
                    format!("type:dead_code|project:{}|symbol:{}", project_id, dead.name);
                docs.push((metadata, context));
            }
        }

        // --- Process Duplicates ---
        if let Some(summary) = &run.ai_views.ci_summary {
            let summary_context = format!(
                "Code Duplication Summary: {} duplicate clusters detected.",
                summary.duplicate_count
            );
            let summary_meta = format!("type:duplication_summary|project:{}", project_id);
            docs.push((summary_meta, summary_context));

            if let Some(clusters) = &summary.top_clusters {
                for cluster in clusters {
                    let context = format!(
                        "Code Duplication: Symbol '{}' appears {} times. Severity: {}.",
                        cluster.symbol_name, cluster.size, cluster.severity
                    );

                    let metadata = format!(
                        "type:duplication|project:{}|symbol:{}",
                        project_id, cluster.symbol_name
                    );
                    docs.push((metadata, context));
                }
            }
        }
    }

    docs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_documents_dead_code() {
        let report = AnalysisReport {
            analysis: vec![AnalysisRun {
                ai_views: AiViews {
                    dead_symbols: Some(vec![DeadSymbol {
                        name: "GhostFunc".to_string(),
                        paths: vec!["src/ghost.rs".to_string()],
                        public_surface: false,
                    }]),
                    ci_summary: None,
                },
            }],
        };

        let docs = prepare_documents(&report, "test-proj");

        assert_eq!(docs.len(), 1);
        let (meta, text) = &docs[0];

        assert!(meta.contains("type:dead_code"));
        assert!(meta.contains("project:test-proj"));
        assert!(meta.contains("symbol:GhostFunc"));
        assert!(text.contains("Dead Code Detection"));
        assert!(text.contains("GhostFunc"));
    }

    #[test]
    fn test_metadata_parser() {
        let raw = "type:dead_code|project:abc|symbol:Foo";
        let map = parse_metadata_string(raw);

        assert_eq!(map.get("type").unwrap(), "dead_code");
        assert_eq!(map.get("project").unwrap(), "abc");
        assert_eq!(map.get("symbol").unwrap(), "Foo");
        assert!(map.contains_key("indexed_at"));
    }
}
