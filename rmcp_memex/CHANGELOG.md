# Changelog

## [0.1.9] - 2025-12-04
- **Security: Path traversal protection**: Added `validate_path()` function to prevent path traversal attacks in `rag_index` and config file loading. Paths are validated against HOME/CWD directories.
- **Dependency updates**:
  - `rmcp`: 0.9.0 → 0.10.0 (MCP SDK update)
  - `crossterm`: 0.28 → 0.29
- **Rust edition 2024**: Migrated to Rust 2024 edition.
- **TUI Configuration Wizard**: Added `rmcp_memex wizard` subcommand with interactive terminal UI.
  - Auto-detection of MCP host configurations (Codex, Cursor, Claude Desktop, JetBrains, VS Code)
  - Step-by-step configuration: Welcome → Settings → Host Selection → Preview → Health Check → Summary
  - Config snippet generation for TOML (Codex) and JSON (Claude, Cursor, etc.) formats
  - Dry-run mode (`--dry-run`) for safe preview without writing files
  - Health check: binary verification and database path validation
- **New dependencies**: Added `ratatui` and `crossterm` for TUI rendering.
- **CLI restructure**: Introduced subcommands (`serve`, `wizard`); `serve` is default when no subcommand specified.
- **Repository rename**: Updated repo URL to `github.com/Loctree/rmcp-memex`.

## [0.1.8] - 2025-12-04
- **Server modes**: Added `--mode memory|full` CLI flag and `ServerConfig::for_memory_only()` / `::for_full_rag()` factory methods for simplified configuration.
- **Namespace conventions**: Documented recommended namespace patterns (`user:<id>`, `agent:<id>`, `session:<id>`, `kb:<name>`, `project:<name>`) in README.
- **Schema versioning**: Added `SCHEMA_VERSION` constant and `docs/MIGRATION.md` with migration procedures.
- **Backend interfaces**: Documented embedding and storage backend interfaces for future extensibility.
- **Docs cleanup**: Updated GUIDELINES.md to remove references to removed dependencies (octocrab, scraper, quick-xml).

## [0.1.7] - 2025-12-04
- **Library-first architecture**: Refactored crate to expose full public API for library consumers.
  - Binary is now a thin wrapper (CLI + logger + `run_stdio_server()` call).
  - Re-exports: `RAGPipeline`, `SearchResult`, `StorageManager`, `ChromaDocument`, `FastEmbedder`, `MLXBridge`, `MCPServer`, `ServerConfig`.
- Added explicit `[lib]` and `[[bin]]` sections to `Cargo.toml`.
- Added crate metadata: description, license, repository, keywords, categories.
- Enables direct integration as a dependency (e.g., for loctree embedding pipelines).

## [0.1.6] - 2025-12-04
- **Transport fix**: Switched from LSP-style Content-Length framing to newline-delimited JSON (standard MCP transport). Fixes compatibility with Codex and other MCP hosts.
- **Dependency optimization**: Removed unused crates (`octocrab`, `scraper`, `quick-xml`); disabled LanceDB cloud features (`aws`, `azure`, `gcs`, `oss`, `dynamodb`). Reduced unique dependencies from ~1011 to ~618 (~39% reduction).
- Added Loctree integration proposal (`docs/LOCTREE_INTEGRATION_PROPOSAL.md`).
- Silenced vendored protoc build warning.
- Updated documentation and improved Codex config example.

## [0.1.5] - 2025-12-03
- Renamed crate/binary from `mcp_memex` to `rmcp_memex`.
- Added GitHub Actions CI: fmt, clippy, semgrep, tests, tarpaulin coverage (with protoc install).
- Introduced config loader (`--config <toml>`) with flag overrides; added `max_request_bytes` limit (default 5 MB) and improved log-level parsing.
- Added `health` tool (version, db_path, cache_dir, backend) and safer JSON-RPC framing.
- Improved temp LanceDB isolation in tests; clarified embed example and env handling.
- Added pre-push hook with full quality gate (fmt, clippy, test, semgrep).

## [0.1.1] - 2025-11-25
- Defaulted fastembed/HF cache to `$HOME/.cache/fastembed` to avoid `.fastembed_cache` in CWDs.
- Refined hooks/clippy and installer/build scripts.
- Fixed build script path (`build = "src/build.rs"`).

## [0.1.0] - 2024-11-20
- Switched vector storage from ChromaDB to embedded LanceDB.
- Added namespaces and memory tools to the RAG server.
- Initial MCP Rust server structure, README, and build scripts.
