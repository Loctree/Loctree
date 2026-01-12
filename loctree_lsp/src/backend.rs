//! LSP Backend implementation for loctree
//!
//! Provides lifecycle handlers and document synchronization for the LSP server.
//!
//! Created by M&K (c)2025 The LibraxisAI Team

use std::path::PathBuf;

use dashmap::DashMap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::actions;
use crate::diagnostics;
use crate::navigation::{definition_to_response, get_word_at_position};
use crate::references::references_to_locations;
use crate::snapshot::SnapshotState;

/// Loctree LSP backend state
pub struct Backend {
    /// LSP client for sending notifications/responses
    client: Client,
    /// Document content cache (uri -> content)
    documents: DashMap<Url, String>,
    /// Cached diagnostics per document URI
    cached_diagnostics: DashMap<Url, Vec<Diagnostic>>,
    /// Workspace root path
    workspace_root: RwLock<Option<String>>,
    /// Loaded snapshot state
    snapshot: SnapshotState,
}

impl Backend {
    /// Create a new Backend instance
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            cached_diagnostics: DashMap::new(),
            workspace_root: RwLock::new(None),
            snapshot: SnapshotState::new(),
        }
    }

    /// Load snapshot from workspace
    async fn load_snapshot(&self) {
        let root = self.workspace_root.read().await.clone();
        if let Some(root_path) = root {
            let path = PathBuf::from(&root_path);
            match self.snapshot.load(&path).await {
                Ok(()) => {
                    tracing::info!("Loaded snapshot from {}", root_path);
                    self.client
                        .log_message(MessageType::INFO, "loctree snapshot loaded")
                        .await;
                }
                Err(e) => {
                    tracing::warn!("Failed to load snapshot: {}", e);
                    self.client
                        .log_message(MessageType::WARNING, format!("{}", e))
                        .await;
                }
            }
        }
    }

    /// Trigger diagnostics for a document
    async fn publish_diagnostics(&self, uri: Url) {
        // Extract file path from URI
        let file_path = uri.path();
        tracing::debug!("Analyzing: {}", file_path);

        // Collect diagnostics from snapshot
        let diags = diagnostics::collect_diagnostics(&self.snapshot, file_path).await;

        // Cache and publish
        self.cached_diagnostics.insert(uri.clone(), diags.clone());
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    /// Get the workspace root path
    #[allow(dead_code)]
    pub async fn workspace_root(&self) -> Option<String> {
        self.workspace_root.read().await.clone()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Store workspace root
        if let Some(root) = params.root_uri {
            let mut wr = self.workspace_root.write().await;
            *wr = Some(root.path().to_string());
            tracing::info!("Workspace root: {}", root.path());
        }

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "loctree-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("loctree".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("loctree-lsp server initialized");

        // Load snapshot from workspace
        self.load_snapshot().await;

        self.client
            .log_message(MessageType::INFO, "loctree-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("loctree-lsp server shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let content = params.text_document.text.clone();
        tracing::debug!("did_open: {} ({} bytes)", uri, content.len());

        // Store document content
        self.documents.insert(uri.clone(), content);

        // Trigger diagnostics
        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        tracing::debug!("did_change: {}", uri);

        // With FULL sync, we get the entire content in the first change
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.insert(uri.clone(), change.text);

            // Trigger diagnostics on change
            self.publish_diagnostics(uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        tracing::debug!("did_save: {}", uri);

        // Update content if provided
        if let Some(text) = params.text {
            self.documents.insert(uri.clone(), text);
        }

        // Trigger full diagnostics on save
        self.publish_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        tracing::debug!("did_close: {}", uri);

        // Remove from document cache
        self.documents.remove(&uri);

        // Clear cached diagnostics
        self.cached_diagnostics.remove(&uri);

        // Publish empty diagnostics to clear any shown in the editor
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;
        tracing::debug!("goto_definition: {} at {:?}", uri, position);

        // Get document content
        let content = match self.documents.get(&uri) {
            Some(doc) => doc.clone(),
            None => {
                tracing::warn!("Document not found in cache: {}", uri);
                return Ok(None);
            }
        };

        // Extract word at cursor position
        let symbol = match get_word_at_position(&content, position) {
            Some(word) => word,
            None => {
                tracing::debug!("No word at cursor position");
                return Ok(None);
            }
        };
        tracing::debug!("Looking up definition for symbol: {}", symbol);

        // Get file path relative to workspace
        let file_path = uri.path();

        // Look up definition in snapshot
        let definition = self.snapshot.find_definition(file_path, &symbol).await;

        match definition {
            Some(def) => {
                tracing::debug!("Found definition: {:?}", def);
                let workspace_root = self.workspace_root.read().await;
                let root = workspace_root.as_deref().unwrap_or("");
                Ok(definition_to_response(&def, root))
            }
            None => {
                tracing::debug!("No definition found for symbol: {}", symbol);
                Ok(None)
            }
        }
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;
        tracing::debug!(
            "references: {} at {:?} (include_declaration: {})",
            uri,
            position,
            include_declaration
        );

        // Get document content
        let content = match self.documents.get(&uri) {
            Some(doc) => doc.clone(),
            None => {
                tracing::warn!("Document not found in cache: {}", uri);
                return Ok(None);
            }
        };

        // Extract word at cursor position
        let symbol = match get_word_at_position(&content, position) {
            Some(word) => word,
            None => {
                tracing::debug!("No word at cursor position");
                return Ok(None);
            }
        };
        tracing::debug!("Finding references for symbol: {}", symbol);

        // Get file path relative to workspace
        let file_path = uri.path();

        // Find all references using snapshot edges
        let references = self
            .snapshot
            .find_references(file_path, Some(&symbol))
            .await;

        if references.is_empty() {
            tracing::debug!("No references found for symbol: {}", symbol);
            return Ok(None);
        }

        tracing::debug!(
            "Found {} references for symbol: {}",
            references.len(),
            symbol
        );

        // Convert to LSP locations
        let workspace_root = self.workspace_root.read().await;
        let root = workspace_root.as_deref();
        let mut locations = references_to_locations(&references, root);

        // Include declaration if requested
        if include_declaration
            && let Some((decl_file, decl_line)) =
                self.snapshot.find_export_location(file_path, &symbol).await
        {
            // Build full path for declaration
            let full_path = if let Some(root) = root {
                format!("{}/{}", root.trim_end_matches('/'), &decl_file)
            } else {
                decl_file
            };

            if let Ok(decl_uri) = Url::from_file_path(&full_path) {
                let line = if decl_line > 0 { decl_line - 1 } else { 0 } as u32;
                let decl_location = Location {
                    uri: decl_uri,
                    range: Range::new(Position::new(line, 0), Position::new(line, 1000)),
                };
                // Insert declaration at the beginning
                locations.insert(0, decl_location);
            }
        }

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let range = params.range;
        tracing::debug!("code_action: {} at {:?}", uri, range);

        let mut code_actions: Vec<CodeActionOrCommand> = Vec::new();
        let file_path = uri.path();

        // Get document content for symbol detection
        let content = self.documents.get(&uri).map(|doc| doc.clone());

        // Get cached diagnostics for this file
        let diagnostics_in_range: Vec<Diagnostic> = self
            .cached_diagnostics
            .get(&uri)
            .map(|d| {
                d.iter()
                    .filter(|diag| ranges_overlap(&diag.range, &range))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        // Get workspace root for quickfix actions
        let workspace_root = self.workspace_root.read().await;
        let root = workspace_root.as_deref();

        // Add quickfix actions for diagnostics (cycles, dead exports)
        for diag in &diagnostics_in_range {
            if let Some(NumberOrString::String(code)) = &diag.code {
                match code.as_str() {
                    "circular-import" | "cycle" => {
                        // Add quickfix actions for cycles
                        let quickfix_actions = actions::cycle_fixes(diag, &uri);
                        for action in quickfix_actions {
                            code_actions.push(CodeActionOrCommand::CodeAction(action));
                        }

                        // Get cycle info from snapshot and add cycle refactors
                        let cycles = self.snapshot.cycles_for_file(file_path).await;
                        for cycle in cycles {
                            let cycle_actions = actions::cycle_refactors(file_path, &cycle.files);
                            for action in cycle_actions {
                                code_actions.push(CodeActionOrCommand::CodeAction(action));
                            }
                        }
                    }
                    "dead-export" => {
                        // Add quickfix actions for dead exports
                        let quickfix_actions = actions::dead_export_fixes(diag, &uri, root);
                        for action in quickfix_actions {
                            code_actions.push(CodeActionOrCommand::CodeAction(action));
                        }

                        // Extract symbol from diagnostic message for refactor actions
                        if let Some(symbol) = extract_symbol_from_diagnostic(&diag.message) {
                            let export_actions = actions::export_refactors(&symbol, &uri, 0);
                            for action in export_actions {
                                code_actions.push(CodeActionOrCommand::CodeAction(action));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Release workspace root lock before further async operations
        drop(workspace_root);

        // Add file-level refactoring actions
        let consumers = self.snapshot.find_references(file_path, None).await;
        let file_actions = actions::file_refactors(&uri, file_path, consumers.len());
        for action in file_actions {
            code_actions.push(CodeActionOrCommand::CodeAction(action));
        }

        // Add symbol-specific refactoring actions if cursor is on a symbol
        if let Some(ref content) = content
            && let Some(symbol) = get_word_at_position(content, range.start)
        {
            // Find how many files import this symbol
            let references = self
                .snapshot
                .find_references(file_path, Some(&symbol))
                .await;
            let symbol_actions = actions::export_refactors(&symbol, &uri, references.len());
            for action in symbol_actions {
                code_actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        if code_actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(code_actions))
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;
        tracing::debug!("hover: {} at {:?}", uri, position);

        // Extract file path from URI
        let file_path = uri.path();

        // Get hover info from snapshot
        let hover = self.snapshot.get_hover_info(file_path, position).await;

        if hover.is_some() {
            tracing::debug!("Hover info found for {}", file_path);
        } else {
            tracing::debug!("No hover info at position");
        }

        Ok(hover)
    }
}

/// Check if two ranges overlap
fn ranges_overlap(a: &Range, b: &Range) -> bool {
    // Ranges overlap if neither is completely before or after the other
    !(a.end.line < b.start.line
        || (a.end.line == b.start.line && a.end.character < b.start.character)
        || b.end.line < a.start.line
        || (b.end.line == a.start.line && b.end.character < a.start.character))
}

/// Extract symbol name from diagnostic message
///
/// Parses messages like "Export 'foo' is unused (0 imports)" to extract "foo"
fn extract_symbol_from_diagnostic(message: &str) -> Option<String> {
    // Look for text between single quotes
    let start = message.find('\'')?;
    let rest = &message[start + 1..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}
