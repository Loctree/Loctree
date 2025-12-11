# rmcp_memex

Lightweight **Model Context Protocol (MCP)** server written in Rust. Provides a local RAG (Retrieval-Augmented Generation) toolset backed by embedded **LanceDB** vector storage and local embeddings via **fastembed**. Optional MLX HTTP bridge for Apple Silicon acceleration.

## Features

- **Vector Memory**: Store, search, and retrieve text chunks with semantic similarity
- **Document Indexing**: Index UTF-8 text files and PDFs for RAG queries
- **Namespace Isolation**: Organize data into separate namespaces
- **Health Monitoring**: Built-in health tool for status checks
- **Configuration Wizard**: Interactive TUI for easy setup and host configuration
- **MCP Compatible**: Works with Claude Desktop, Codex, Cursor, and other MCP hosts

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/Loctree/rmcp-memex.git
cd rmcp_memex
cargo build --release

# Install to ~/.cargo/bin (optional)
cp target/release/rmcp_memex ~/.cargo/bin/
```

Or use the install script:

```bash
./scripts/install.sh
# For macOS app bundle:
./scripts/install.sh --bundle-macos
```

### Run

```bash
# Default configuration (starts MCP server)
rmcp_memex

# Explicit serve subcommand
rmcp_memex serve

# With options
rmcp_memex --db-path ~/mydata/lancedb --log-level debug --cache-mb 2048
```

### Configuration Wizard

Interactive TUI for setting up rmcp_memex and configuring MCP host integrations:

```bash
# Launch wizard
rmcp_memex wizard

# Dry-run mode (preview changes without writing)
rmcp_memex wizard --dry-run
```

The wizard will:
1. Auto-detect installed MCP hosts (Codex, Cursor, Claude Desktop, JetBrains, VS Code)
2. Guide you through memex configuration (database path, cache size, log level, mode)
3. Generate config snippets for selected hosts
4. Run health checks to verify setup
5. Optionally write configuration files (with backups)

## Usage as Library

Add to your `Cargo.toml`:

```toml
[dependencies]
rmcp_memex = { git = "https://github.com/Loctree/rmcp-memex.git" }
```

### Example: Direct RAG Pipeline Access

```rust
use rmcp_memex::{RAGPipeline, StorageManager, ServerConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize storage
    let storage = Arc::new(StorageManager::new(4096, "~/.my_app/lancedb").await?);
    storage.ensure_collection().await?;

    // Create RAG pipeline (no MLX bridge)
    let mlx = Arc::new(Mutex::new(None));
    let rag = RAGPipeline::new(mlx, storage).await?;

    // Index text
    rag.memory_upsert(
        "my_namespace",
        "doc1".to_string(),
        "Important information to remember".to_string(),
        serde_json::json!({"source": "manual"}),
    ).await?;

    // Search
    let results = rag.memory_search("my_namespace", "important", 5).await?;
    for r in results {
        println!("{}: {} (score: {})", r.id, r.text, r.score);
    }

    Ok(())
}
```

### Example: Run Full MCP Server Programmatically

```rust
use rmcp_memex::{run_stdio_server, ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig::default()
        .with_db_path("~/custom/lancedb");
    
    run_stdio_server(config).await
}
```

### Exported Types

| Type | Description |
|------|-------------|
| `ServerConfig` | Configuration for server/pipeline |
| `RAGPipeline` | Core RAG operations: index, search, memory |
| `SearchResult` | Search result with id, text, score, metadata |
| `StorageManager` | LanceDB + sled + moka cache layer |
| `ChromaDocument` | Document struct for storage |
| `FastEmbedder` | Local embeddings via fastembed |
| `MLXBridge` | Optional MLX HTTP bridge for Apple Silicon |
| `MCPServer` | MCP protocol handler |

## MCP Tools

| Tool | Description |
|------|-------------|
| `health` | Server status: version, db_path, cache_dir, backend (mlx/fastembed) |
| `rag_index` | Index a file (UTF-8 text or PDF) into vector store |
| `rag_index_text` | Index raw text with optional ID and metadata |
| `rag_search` | Semantic search across indexed documents |
| `memory_upsert` | Store a text chunk in a namespace |
| `memory_get` | Retrieve a chunk by namespace + ID |
| `memory_search` | Semantic search within a namespace |
| `memory_delete` | Delete a chunk by namespace + ID |
| `memory_purge_namespace` | Delete all chunks in a namespace |

## Namespace Conventions

Namespaces provide data isolation and multi-tenancy. We recommend these naming patterns:

| Pattern | Use Case | Example |
|---------|----------|---------|
| `user:<id>` | Per-user memory isolation | `user:alice`, `user:12345` |
| `agent:<id>` | Per-AI-agent memory | `agent:claude`, `agent:codex` |
| `session:<id>` | Ephemeral session data | `session:abc123` |
| `kb:<name>` | Shared knowledge bases | `kb:docs`, `kb:codebase` |
| `project:<name>` | Project-scoped data | `project:myapp`, `project:rmcp_memex` |

### Best Practices

1. **Use prefixes consistently** — helps with retention policies and access control
2. **Keep IDs URL-safe** — avoid special characters; use alphanumeric + hyphen/underscore
3. **Document your schema** — maintain a mapping of namespace patterns in your project
4. **Consider lifecycle** — `session:*` for temporary, `kb:*` for persistent data

### Examples

```rust
// Per-user memory
rag.memory_upsert("user:alice", "pref1", "Likes dark mode".into(), json!({})).await?;

// Shared knowledge base
rag.memory_upsert("kb:company-docs", "doc1", "Q4 report...".into(), json!({"type": "report"})).await?;

// Session-scoped (clean up after session ends)
rag.memory_upsert("session:abc123", "context", "Current task...".into(), json!({})).await?;
rag.purge_namespace("session:abc123").await?; // Cleanup
```

## Configuration

### CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `--config` | — | Path to TOML config file |
| `--mode` | `full` | Server mode: `memory` (memory-only) or `full` (all features) |
| `--db-path` | `~/.rmcp_servers/rmcp_memex/lancedb` | LanceDB storage path |
| `--cache-mb` | `4096` | Cache size in MB |
| `--log-level` | `info` | Logging level: trace, debug, info, warn, error |
| `--max-request-bytes` | `5242880` (5 MB) | Max JSON-RPC request size |
| `--features` | `filesystem,memory,search` | Feature flags (overrides `--mode`) |

### Server Modes

| Mode | Features | Use Case |
|------|----------|----------|
| `full` | filesystem, memory, search | Full RAG with document indexing |
| `memory` | memory, search | Pure vector memory server (no filesystem access) |

```bash
# Memory-only mode (recommended for AI assistants)
rmcp_memex --mode memory

# Full RAG mode (default)
rmcp_memex --mode full
```

### TOML Config File

```toml
mode = "memory"  # or "full"
db_path = "~/.rmcp_servers/rmcp_memex/lancedb"
cache_mb = 4096
log_level = "info"
max_request_bytes = 5242880
# features = "memory,search"  # Optional: overrides mode
```

CLI flags override config file values.

### Environment Variables

| Variable | Description |
|----------|-------------|
| `FASTEMBED_CACHE_PATH` | Fastembed model cache (default: `$HOME/.cache/fastembed`) |
| `HF_HUB_CACHE` | HuggingFace cache path |
| `DISABLE_MLX` | Set to disable MLX bridge, use fastembed only |
| `DRAGON_BASE_URL` | MLX HTTP server base URL (default: `http://localhost`) |
| `MLX_JIT_MODE` | `true` for single-port MLX mode |
| `MLX_JIT_PORT` | JIT mode port (default: `1234`) |
| `EMBEDDER_PORT` | Non-JIT embeddings port (default: `12345`) |
| `RERANKER_PORT` | Non-JIT reranker port (default: `12346`) |

## MCP Host Configuration

### Codex (`~/.codex/config.toml`)

```toml
[mcp_servers."rmcp_memex"]
command = "/path/to/rmcp_memex"
args = ["--db-path", "~/.rmcp_servers/rmcp_memex/lancedb", "--log-level", "info"]
startup_timeout_sec = 120
```

### Claude Desktop (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "rmcp_memex": {
      "command": "/path/to/rmcp_memex",
      "args": ["--log-level", "info"]
    }
  }
}
```

### Cursor / Other MCP Hosts

Use stdio transport with the binary path and desired arguments.

## Library Usage

```rust
use rmcp_memex::{run_stdio_server, ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ServerConfig::default()
        .with_db_path("/custom/path/lancedb");
    
    run_stdio_server(config).await
}
```

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   MCP Client    │────▶│   rmcp_memex     │────▶│    LanceDB      │
│ (Claude, Codex) │     │  (JSON-RPC/stdio)│     │  (embeddings)   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                               │
                               ▼
                        ┌──────────────────┐
                        │  fastembed/MLX   │
                        │   (embeddings)   │
                        └──────────────────┘
```

- **Transport**: Newline-delimited JSON-RPC over stdio
- **Vector Store**: Embedded LanceDB (no external DB required)
- **Embeddings**: fastembed (default) or MLX HTTP bridge
- **Caching**: moka (in-memory) + sled (persistent KV)

## Development

```bash
# Format
cargo fmt

# Lint
cargo clippy --all-targets -- -D warnings

# Test
cargo test

# Build release
cargo build --release
```

### Git Hooks

Install pre-commit and pre-push hooks:

```bash
./tools/install-githooks.sh
```

Pre-push runs: fmt check, clippy, tests, semgrep.

## Requirements

- **Rust**: Stable toolchain
- **OS**: macOS, Linux (Windows untested)
- **Protobuf**: Build uses vendored protoc, but system protoc may be needed:
  - macOS: `brew install protobuf`
  - Linux: `apt install protobuf-compiler`

## License

MIT — see [LICENSE](LICENSE).

## Links

- **Repository**: https://github.com/Loctree/rmcp_memex
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **Loctree Integration**: [docs/LOCTREE_INTEGRATION_PROPOSAL.md](docs/LOCTREE_INTEGRATION_PROPOSAL.md)
