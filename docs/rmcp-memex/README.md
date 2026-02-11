# rmcp-memex

RAG/Memory MCP Server with LanceDB vector storage for AI agents.

## Overview

`rmcp-memex` is an MCP (Model Context Protocol) server providing:
- **RAG (Retrieval-Augmented Generation)** - document indexing and semantic search
- **Vector Memory** - semantic storage and retrieval of text chunks
- **Namespace Isolation** - data isolation in namespaces
- **Security** - token-based access control for protected namespaces
- **Onion Slice Architecture** - hierarchical embeddings (OUTER→MIDDLE→INNER→CORE)
- **Preprocessing** - automatic noise filtering from conversation exports (~36-40% reduction)
- **Exact-Match Deduplication** - SHA256-based dedup for overlapping exports

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      rmcp-memex                              │
├─────────────────────────────────────────────────────────────┤
│  MCP Server (JSON-RPC over stdio)                           │
│  ├── handlers/mod.rs    - Request routing & validation      │
│  ├── security/mod.rs    - Namespace access control          │
│  └── rag/mod.rs         - RAG pipeline                      │
├─────────────────────────────────────────────────────────────┤
│  Storage Layer                                               │
│  ├── LanceDB           - Vector embeddings                  │
│  ├── sled              - Key-value store                    │
│  └── moka              - In-memory cache                    │
├─────────────────────────────────────────────────────────────┤
│  Embeddings (External Providers)                             │
│  ├── Ollama            - Local models (recommended)         │
│  ├── MLX Bridge        - Apple Silicon acceleration         │
│  └── OpenAI-compatible - Any compatible endpoint            │
└─────────────────────────────────────────────────────────────┘
```

## Features

### RAG Tools
| Tool | Description |
|------|-------------|
| `rag_index` | Index document from file |
| `rag_index_text` | Index raw text |
| `rag_search` | Search documents semantically |

### Memory Tools
| Tool | Description |
|------|-------------|
| `memory_upsert` | Add/update chunk in namespace |
| `memory_get` | Get chunk by ID |
| `memory_search` | Search semantically in namespace |
| `memory_delete` | Delete chunk |
| `memory_purge_namespace` | Delete all chunks in namespace |

### Security Tools
| Tool | Description |
|------|-------------|
| `namespace_create_token` | Create access token for namespace |
| `namespace_revoke_token` | Revoke token (namespace becomes public) |
| `namespace_list_protected` | List protected namespaces |
| `namespace_security_status` | Security system status |

## Quick Start

### Installation

```bash
cd loctree-suite
cargo install --path rmcp-memex
```

### Running

```bash
# Default mode (all features)
rmcp_memex serve

# Memory-only mode (no filesystem access)
rmcp_memex serve --mode memory

# With security enabled
rmcp_memex serve --security-enabled
```

### Configuration (TOML)

```toml
# ~/.rmcp_servers/config/rmcp-memex.toml

mode = "full"
db_path = "~/.rmcp_servers/rmcp_memex/lancedb"
cache_mb = 4096
log_level = "info"

# Whitelist of allowed paths
allowed_paths = [
    "~",
    "/Volumes/ExternalDrive/data"
]

# Security
security_enabled = true
token_store_path = "~/.rmcp_servers/rmcp_memex/tokens.json"
```

## Documentation

- [01_security.md](./01_security.md) - Security system (namespace tokens)
- [02_configuration.md](./02_configuration.md) - Configuration and CLI options

## Onion Slice Architecture

Instead of traditional flat chunking, rmcp-memex offers hierarchical "onion slices":

```
┌─────────────────────────────────────────┐
│  OUTER (~100 chars)                     │  ← Minimum context, maximum navigation
│  Keywords + ultra-compression           │
├─────────────────────────────────────────┤
│  MIDDLE (~300 chars)                    │  ← Key sentences + context
├─────────────────────────────────────────┤
│  INNER (~600 chars)                     │  ← Expanded content
├─────────────────────────────────────────┤
│  CORE (full text)                       │  ← Complete document
└─────────────────────────────────────────┘
```

**Philosophy:** "Minimum info → Maximum navigation paths"

### CLI Commands

```bash
# Index with onion slicing (default)
rmcp_memex index -n memories /path/to/data/ --slice-mode onion

# Index with flat chunking (backward compatible)
rmcp_memex index -n memories /path/to/data/ --slice-mode flat

# Search in namespace
rmcp_memex search -n memories -q "best moments" --limit 10

# Search only in specific layer
rmcp_memex search -n memories -q "query" --layer outer

# Drill down in hierarchy (expand children)
rmcp_memex expand -n memories -i "slice_id_here"

# Get chunk by ID
rmcp_memex get -n memories -i "chunk_abc123"

# RAG search (cross-namespace)
rmcp_memex rag-search -q "search term" --limit 5

# List namespaces with stats
rmcp_memex namespaces --stats

# Export namespace to JSON
rmcp_memex export -n memories -o backup.json --include-embeddings
```

### Preprocessing (Noise Filtering)

Automatic removal of ~36-40% noise from conversation exports:
- MCP tool artifacts (`<function_calls>`, `<invoke>`, etc.)
- CLI output (git status, cargo build, npm install)
- Metadata (UUIDs, timestamps → placeholders)
- Empty/boilerplate content

```bash
# Index with preprocessing
rmcp_memex index -n memories /path/to/export.json --preprocess
```

### Exact-Match Deduplication

SHA256-based dedup for overlapping exports (e.g., quarterly exports containing 6 months of data):

```bash
# Dedup enabled (default)
rmcp_memex index -n memories /path/to/data/

# Disable dedup
rmcp_memex index -n memories /path/to/data/ --no-dedup
```

**Output with statistics:**
```
Indexing complete:
  New chunks:          234
  Files indexed:       67
  Skipped (duplicate): 33
  Deduplication:       enabled
```

## Code Structure

```
rmcp-memex/
├── src/
│   ├── lib.rs              # Public API & ServerConfig
│   ├── bin/
│   │   └── rmcp_memex.rs   # CLI binary (serve, index, search, get, expand, etc.)
│   ├── handlers/
│   │   └── mod.rs          # MCP request handlers
│   ├── security/
│   │   └── mod.rs          # Namespace access control
│   ├── rag/
│   │   └── mod.rs          # RAG pipeline + OnionSlice architecture
│   ├── preprocessing/
│   │   └── mod.rs          # Noise filtering for conversation exports
│   ├── storage/
│   │   └── mod.rs          # LanceDB + sled (schema v3 with content_hash)
│   ├── embeddings/
│   │   └── mod.rs          # MLX/FastEmbed bridge
│   └── tui/
│       └── mod.rs          # Configuration wizard
└── Cargo.toml
```

## Claude/MCP Integration

Add to `~/.claude.json`:

```json
{
  "mcpServers": {
    "rmcp-memex": {
      "command": "rmcp_memex",
      "args": ["serve", "--security-enabled"]
    }
  }
}
```

---

Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
