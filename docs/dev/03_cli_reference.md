# Loctree Suite - CLI Reference

Complete reference for all binaries in loctree-suite.

## loct

Primary CLI for codebase analysis. Agent-optimized with artifact persistence.

### Basic Usage

```bash
# Scan current directory
loct

# Scan specific project
loct /path/to/project

# Force rescan
loct --force
```

### Query Mode (jq-style)

```bash
# Extract metadata
loct '.metadata'

# Count files
loct '.files | length'

# Get health score
loct '.summary.health_score'

# Filter edges
loct '.edges[] | select(.from | contains("api"))'

# Dead exports
loct '.dead_parrots'

# Cycles
loct '.cycles'
```

### Output Flags

```bash
loct --for-ai          # AI-optimized hierarchical output
loct --findings        # All issues (dead, cycles, twins)
loct --summary         # Quick summary stats
loct --json            # Raw JSON output
```

### Slice Command

Extract file context with dependencies and consumers:

```bash
loct slice src/App.tsx
loct slice src/api/client.ts --consumers
loct slice src/utils.ts --json
```

### Analysis Commands

```bash
loct health            # Quick health check
loct cycles            # Circular imports
loct dead              # Dead exports
loct dead --confidence high
loct twins             # Code duplication
loct audit             # Full audit report
```

### Report Generation

```bash
loct report            # Generate HTML report
loct report --serve    # Serve with live reload
loct report --open     # Open in browser
```

### Options

| Flag | Description |
|------|-------------|
| `--force`, `-f` | Force rescan |
| `--json` | JSON output |
| `--for-ai` | AI-optimized output |
| `--findings` | Show all issues |
| `--summary` | Show summary only |
| `--verbose`, `-v` | Verbose output |
| `--quiet`, `-q` | Minimal output |

---

## loctree

Full CLI with all features. **Deprecated** - use `loct` instead.

Shows deprecation warning:
```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  DEPRECATED: `loctree` will be removed in v0.9.0                  ┃
┃  Use `loct` instead - it has all features + agent optimizations  ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

Same commands as `loct`.

---

## loctree-mcp

MCP server for AI agents. Runs over stdio.

### Usage

```bash
# Start server (stdio)
loctree-mcp

# With logging
loctree-mcp --log-level debug
```

### MCP Tools

| Tool | Description |
|------|-------------|
| `for_ai` | AI-optimized project overview |
| `slice` | File context with deps + consumers |
| `find` | Symbol search |
| `impact` | Change impact analysis |
| `health` | Quick health check |
| `scan` | Trigger project scan |
| `query` | Graph queries |

### Configuration

Add to MCP config:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

---

## rmcp_memex

RAG/memory MCP server with vector storage.

### Subcommands

#### serve (default)

Start MCP server:

```bash
rmcp_memex serve
rmcp_memex  # same as serve
```

Options:

| Flag | Description |
|------|-------------|
| `--mode` | `memory` or `full` |
| `--cache-mb` | Cache size in MB |
| `--db-path` | LanceDB path |
| `--config` | TOML config file |

#### wizard

Interactive setup TUI:

```bash
rmcp_memex wizard
rmcp_memex wizard --dry-run
```

#### index

Batch document indexing:

```bash
# Single file
rmcp_memex index document.pdf

# Directory with glob
rmcp_memex index ./docs -r -g "*.md"

# With namespace
rmcp_memex index ./docs -r -g "*.md" -n my-project

# Custom depth
rmcp_memex index ./src -r --max-depth 3
```

Options:

| Flag | Short | Description |
|------|-------|-------------|
| `--namespace` | `-n` | Namespace for documents |
| `--recursive` | `-r` | Walk subdirectories |
| `--glob` | `-g` | File pattern filter |
| `--max-depth` | | Max directory depth |
| `--db-path` | | Custom LanceDB path |

### MCP Tools

| Tool | Description |
|------|-------------|
| `rag_index` | Index document from path |
| `rag_index_text` | Index raw text |
| `rag_search` | Semantic search |
| `memory_upsert` | Store memory chunk |
| `memory_get` | Get by namespace + ID |
| `memory_search` | Semantic memory search |
| `memory_delete` | Delete chunk |
| `memory_purge_namespace` | Clear namespace |
| `health` | Server health check |

### Configuration

TOML config file:

```toml
# ~/.config/rmcp-memex/config.toml
mode = "full"
features = "filesystem,memory,search"
cache_mb = 4096
db_path = "~/.rmcp_servers/rmcp_memex/lancedb"
log_level = "info"
```

MCP config:

```json
{
  "mcpServers": {
    "rmcp-memex": {
      "command": "rmcp_memex",
      "args": ["serve", "--mode", "full"]
    }
  }
}
```

---

## rmcp-mux

MCP server multiplexer. **Single process manages ALL servers**.

### Architecture

```
rmcp-mux --config mux.toml
    │
    ├── loctree-mcp (child)
    ├── rmcp_memex (child)
    ├── brave-search (child)
    └── ... (all servers from config)
```

### Basic Usage

```bash
# Start all servers from config (default)
rmcp-mux --config ~/.rmcp_servers/config/mux.toml

# Start only specific servers
rmcp-mux --config mux.toml --only loctree,rmcp-memex

# Start all except some
rmcp-mux --config mux.toml --except youtube-transcript

# Show status of all servers
rmcp-mux --show-status --config mux.toml

# Restart a specific service
rmcp-mux --restart-service memex --config mux.toml
```

### Subcommands

#### wizard

Interactive server configuration TUI:

```bash
rmcp-mux wizard
```

#### scan

Scan for existing MCP servers in host configs:

```bash
rmcp-mux scan
```

#### proxy

Bridge STDIO to a mux socket (for MCP hosts):

```bash
rmcp-mux proxy --socket ~/.rmcp_servers/sockets/loctree.sock
```

#### health

Check connection to a mux socket:

```bash
rmcp-mux health --config mux.toml --service loctree
```

### CLI Flags

| Flag | Description |
|------|-------------|
| `--config` | Path to mux.toml config file |
| `--only` | Comma-separated list of servers to start |
| `--except` | Comma-separated list of servers to exclude |
| `--show-status` | Show status of all servers and exit |
| `--restart-service NAME` | Restart a specific service |
| `--lazy-start` | Enable lazy loading for all servers |
| `--log-level` | Log level (trace, debug, info, warn, error) |

### Configuration

```toml
# ~/.rmcp_servers/config/mux.toml

[servers.loctree]
socket = "~/.rmcp_servers/sockets/loctree.sock"
cmd = "loctree-mcp"
args = []
max_active_clients = 5
mode = "eager"  # or "lazy"

[servers.rmcp-memex]
socket = "~/.rmcp_servers/sockets/rmcp-memex.sock"
cmd = "rmcp_memex"
args = ["serve", "--db-path", "~/.rmcp_servers/rmcp_memex/lancedb"]
env = {}
max_active_clients = 5
mode = "lazy"
```

### Makefile Targets

```bash
make mux-setup     # Create directories and initial config
make mux-start     # Start rmcp-mux (manages all servers)
make mux-stop      # Stop rmcp-mux
make mux-restart   # Restart rmcp-mux
make mux-status    # Check status of all servers
make mux-kill      # Force kill rmcp-mux
make mux-tui       # Launch TUI dashboard
make mux-restart-service SERVICE=name  # Restart single service
make mux-logs      # Tail mux.log
make mux-config    # Edit mux.toml
make mcp-health    # Health check all sockets
```

### Host Configuration

Use `rmcp-mux proxy` subcommand in MCP configs:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "rmcp-mux",
      "args": ["proxy", "--socket", "~/.rmcp_servers/sockets/loctree.sock"]
    },
    "rmcp-memex": {
      "command": "rmcp-mux",
      "args": ["proxy", "--socket", "~/.rmcp_servers/sockets/rmcp-memex.sock"]
    }
  }
}
```

---

## Environment Variables

| Variable | Used By | Description |
|----------|---------|-------------|
| `LANCEDB_PATH` | rmcp-memex | Override LanceDB path |
| `RUST_LOG` | all | Log level (trace, debug, info, warn, error) |
| `CARGO_HOME` | make install | Cargo bin directory |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 101 | Panic (Rust) |

---

## Examples

### Full Workflow

```bash
# 1. Scan project
cd my-project
loct

# 2. Check health
loct health

# 3. Find issues
loct --findings

# 4. Get AI context for specific file
loct slice src/main.ts --consumers --json

# 5. Generate report
loct report --serve
```

### Memory Indexing

```bash
# Index project docs
rmcp_memex index ./docs -r -g "*.md" -n project-docs

# Index with server running (works concurrently)
rmcp_memex index ./notes -r -n personal-notes
```

### Multiplexer Setup

```bash
# Setup infrastructure
make mux-setup

# Configure servers in mux.toml
make mux-config

# Start rmcp-mux (single process for all servers)
make mux-start

# Verify all servers are running
make mux-status

# Launch TUI dashboard
make mux-tui

# Configure Claude/Cursor to use rmcp-mux proxy subcommand
# See Host Configuration section above
```

---

Created by M&K (c)2025 The LibraxisAI Team
