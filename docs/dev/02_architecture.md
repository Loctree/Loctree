# Loctree - Architecture

Technical architecture of the public Loctree OSS workspace.

## Workspace Overview

```
Loctree/                         # Cargo workspace root
├── Cargo.toml                    # Workspace manifest
├── Makefile                      # Build automation
│
├── loctree_rs/                   # Core library + CLI
│   ├── src/lib.rs                # Public API
│   ├── src/bin/loct.rs           # Canonical CLI
│   └── src/bin/loctree.rs        # Compatibility alias
│
├── loctree-mcp/                  # MCP server
│   └── src/main.rs               # stdio MCP transport
│
├── rmcp-memex/                   # RAG/memory server
│   ├── src/lib.rs                # Library exports
│   ├── src/bin/rmcp_memex.rs     # CLI (serve, wizard, index)
│   ├── src/rag/                  # RAG pipeline
│   ├── src/storage/              # LanceDB + sled storage
│   ├── src/embeddings/           # FastEmbed + MLX bridge
│   └── src/handlers/             # MCP tool handlers
│
├── rmcp-mux/                     # MCP multiplexer (single-process)
│   ├── src/lib.rs                # Library exports
│   ├── src/bin/rmcp_mux.rs       # Single daemon managing all servers
│   └── src/multi_tui.rs          # TUI dashboard for multi-server
│
├── loctree_memex/                # Loctree + memex integration
│
├── reports/                      # Leptos HTML reports
│   └── src/lib.rs                # Report generation WASM
│
└── landing/                      # WASM landing page
    └── src/main.rs               # Leptos app
```

## Crate Dependency Graph

```
                    ┌─────────────┐
                    │   loctree   │  (core library)
                    └──────┬──────┘
                           │
           ┌───────────────┼───────────────┐
           │               │               │
           ▼               ▼               ▼
    ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
    │ loctree-mcp │ │   reports   │ │loctree_memex│
    └─────────────┘ └─────────────┘ └─────────────┘
                                           │
                                           ▼
                                    ┌─────────────┐
                                    │ rmcp-memex  │
                                    └─────────────┘

    ┌─────────────┐
    │  rmcp-mux   │  (independent)
    └─────────────┘
```

## Crate Details

### loctree (loctree_rs)

**Version**: 0.7.4
**Type**: Library + 2 binaries
**Dependencies**: Pure Rust (no external deps)

Core static analysis library:
- Multi-language parsing (TS/JS, Python, Rust, Go, C/C++)
- Dependency graph construction
- Dead code detection
- Cycle detection
- Code duplication (twins) analysis

**Binaries**:
- `loct` - Canonical CLI with artifact persistence
- `loctree` - Compatibility alias for `loct`

**Key modules**:
```
src/
├── parsers/          # Language-specific AST parsing
│   ├── typescript.rs # OXC-based TS/JS parser
│   ├── python.rs     # tree-sitter Python
│   ├── rust_lang.rs  # syn-based Rust parser
│   └── ...
├── graph/            # Dependency graph algorithms
├── analysis/         # Dead code, cycles, twins
├── artifacts/        # .loctree/ output generation
└── commands/         # CLI command handlers
```

### loctree-mcp

**Version**: 0.1.15
**Type**: Binary only
**Dependencies**: rmcp, loctree

MCP server exposing loctree functionality to AI agents:
- `for_ai` - AI-optimized project overview
- `slice` - File context with deps + consumers
- `find` - Symbol search
- `impact` - Change impact analysis
- `health` - Quick health check
- `scan` - Project scanning
- `query` - Graph queries

### rmcp-memex

**Version**: 0.1.11
**Type**: Library + binary
**Dependencies**: rmcp, lancedb (embeddings via external providers)

RAG/memory MCP server:
- Vector storage with LanceDB
- Document indexing (PDF, text, markdown)
- Semantic search with reranking
- Namespace-based memory isolation

**CLI commands**:
```bash
rmcp_memex serve              # Start MCP server
rmcp_memex wizard             # Interactive setup
rmcp_memex index <path>       # Batch index documents
```

**MCP tools**:
- `rag_index` - Index document
- `rag_index_text` - Index raw text
- `rag_search` - Semantic search
- `memory_upsert` - Store memory chunk
- `memory_get` - Retrieve by ID
- `memory_search` - Semantic memory search
- `memory_delete` - Delete chunk
- `memory_purge_namespace` - Clear namespace

### rmcp-mux

**Version**: 0.3.3
**Type**: Library + binary
**Dependencies**: rmcp, tokio, ratatui

MCP server multiplexer - **single process manages ALL servers**:
- One daemon process for all configured MCP servers
- Unix socket communication per server
- Automatic server lifecycle management
- TUI dashboard for monitoring
- Lazy loading support (spawn on first request)
- Heartbeat monitoring with auto-restart

**Binary**:
- `rmcp-mux` - Multiplexer daemon with `proxy` subcommand

**CLI flags**:
- `--config` - Path to mux.toml
- `--only` - Start only specific servers
- `--except` - Exclude specific servers
- `--show-status` - Show status and exit
- `--restart-service` - Restart single service
- `proxy --socket` - Bridge STDIO to socket

### reports

**Version**: 0.1.9
**Type**: Library (WASM)
**Dependencies**: leptos

Leptos-based HTML report generation:
- Interactive dependency graphs
- Health dashboards
- Export to standalone HTML

### landing

**Type**: WASM application
**Dependencies**: leptos, trunk

Marketing landing page built with Leptos.

## Data Flow

### Loctree Analysis

```
Project Files
     │
     ▼
┌─────────────┐
│   Parser    │  (OXC, tree-sitter, syn)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Graph    │  (nodes: files, edges: imports)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Analysis   │  (dead code, cycles, twins)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Artifacts  │  (.loctree/*.json)
└─────────────┘
```

### rmcp-memex RAG Pipeline

```
Document
    │
    ▼
┌──────────────┐
│ Text Extract │  (PDF, markdown, code)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Chunker    │  (512 chars, 128 overlap)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Embedder    │  (FastEmbed / MLX)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   LanceDB    │  (vector storage)
└──────────────┘
```

## Storage Locations

```
~/.rmcp_servers/
├── config/
│   └── mux.toml             # rmcp-mux server configuration
├── logs/
│   └── mux.log              # Unified mux log
├── pids/
│   └── mux.pid              # Single PID for mux daemon
├── sockets/
│   ├── loctree.sock         # Per-server Unix sockets
│   ├── rmcp-memex.sock
│   └── ...
├── sled/                    # K/V cache (rmcp-memex server)
└── rmcp_memex/
    └── lancedb/             # Vector storage

~/.config/
├── claude/
│   └── claude_desktop_config.json
└── cursor/
    └── mcp.json

/tmp/
└── loctree-make.lock        # Build lock
```

## Configuration Files

### Loctree

```
project/
├── .loctignore              # Files to ignore (gitignore syntax)
└── .loctree/
    ├── manifest.json        # Artifact index
    ├── snapshot.json        # Full graph data
    ├── findings.json        # Issues (dead, cycles, twins)
    └── agent.json           # AI-optimized bundle
```

### rmcp-memex

```toml
# ~/.config/rmcp-memex/config.toml
mode = "full"
cache_mb = 4096
db_path = "~/.rmcp_servers/rmcp_memex/lancedb"
```

## Build Profiles

```toml
# Cargo.toml [profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

## Testing

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p loctree
cargo test -p rmcp-memex

# With output
cargo test -- --nocapture
```

## Feature Flags

### rmcp-mux

```toml
[features]
default = ["cli", "tray"]
cli = ["clap", "ratatui", "crossterm"]
tray = ["tray-icon"]
```

## External Dependencies

| Dependency | Used By | Purpose |
|------------|---------|---------|
| OXC | loctree | TypeScript/JavaScript parsing |
| tree-sitter | loctree | Python, Go parsing |
| syn | loctree | Rust parsing |
| LanceDB | rmcp-memex | Vector storage |
| FastEmbed | rmcp-memex | Local embeddings |
| Leptos | reports, landing | WASM UI |
| rmcp | loctree-mcp, rmcp-* | MCP protocol |

---

VibeCrafted with AI Agents (c)2026 Loctree Team
