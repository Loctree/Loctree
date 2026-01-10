# Loctree Suite - Installation Guide

Complete installation guide for the loctree-suite monorepo.

## Quick Start

```bash
# Fastest path - install core tools only (no protobuf needed)
cargo install loctree

# Or from source
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite
make install
```

## What Gets Installed

### Core Binaries

| Binary | Crate | Description |
|--------|-------|-------------|
| `loct` | loctree | Primary CLI - fast, agent-optimized |
| `loctree` | loctree | Full CLI (deprecated in v0.9.0, use `loct`) |

### MCP Servers

| Binary | Crate | Description |
|--------|-------|-------------|
| `loctree-mcp` | loctree-mcp | MCP server for AI agents (Claude, Cursor, etc.) |
| `rmcp_memex` | rmcp-memex | RAG/vector memory MCP server with LanceDB |
| `rmcp-mux` | rmcp-mux | MCP multiplexer (single process manages all servers) |

## Installation Methods

### 1. Cargo (Recommended)

Install from crates.io:

```bash
# Core only (most common)
cargo install loctree

# With MCP server
cargo install loctree loctree-mcp

# Full suite
cargo install loctree loctree-mcp rmcp-memex rmcp-mux
```

### 2. Homebrew (macOS/Linux)

```bash
# Coming soon - formula pending approval
brew install loctree
```

### 3. From Source

Clone and build:

```bash
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite

# Core only (no external deps)
make install

# Full suite (requires protobuf)
make install-all
```

### 4. Development Setup

```bash
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite

# Build all (debug)
cargo build --workspace

# Build all (release)
cargo build --workspace --release

# Run tests
cargo test --workspace
```

## Dependencies

### Core (loctree, loct)

No external dependencies. Pure Rust.

### MCP Servers (rmcp-memex)

Requires protobuf compiler for LanceDB:

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt install protobuf-compiler

# Fedora
sudo dnf install protobuf-compiler

# Or let Makefile handle it
make setup-protoc
```

## Workspace Structure

```
loctree-suite/
├── loctree_rs/          # Core library + CLI (loct, loctree)
├── loctree-mcp/         # MCP server for loctree
├── rmcp-memex/          # RAG/memory MCP server
├── rmcp-mux/            # MCP multiplexer
├── loctree_memex/       # Loctree + memex integration
├── reports/             # Leptos-based HTML reports
└── landing/             # WASM landing page
```

## Configuration

### MCP Server Setup (Claude Code / Cursor)

Add to your MCP config (`~/.config/claude/claude_desktop_config.json` or similar):

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    },
    "rmcp-memex": {
      "command": "rmcp_memex",
      "args": ["--mode", "full"]
    }
  }
}
```

### Multiplexer Setup (Shared Servers)

For multiple clients sharing a single set of MCP servers:

```bash
# Setup mux infrastructure
make mux-setup

# Configure servers in ~/.rmcp_servers/config/mux.toml
make mux-config

# Start rmcp-mux (single process manages ALL servers)
make mux-start

# Configure clients to use proxy subcommand
# ~/.claude.json
{
  "mcpServers": {
    "loctree": {
      "command": "rmcp-mux",
      "args": ["proxy", "--socket", "~/.rmcp_servers/sockets/loctree.sock"]
    }
  }
}
```

See `docs/dev/.TL_DR/00_mcp_quickstart.md` for complete setup guide.

## Verification

```bash
# Check versions
loct --version
loctree-mcp --version
rmcp_memex --version
rmcp-mux --version

# Test loctree on current directory
loct

# Test MCP server
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | loctree-mcp
```

## Makefile Targets

```bash
make install        # Install core (loct, loctree)
make install-all    # Install everything including MCP servers
make build          # Build all crates (release)
make build-core     # Build only core (no protobuf)
make test           # Run all tests
make check          # Check compilation
make fmt            # Format code
make clean          # Clean build artifacts

# MCP management
make mcp-build      # Build all MCP servers
make mcp-install    # Install all MCP servers
make mux-setup      # Setup mux infrastructure
make mux-start      # Start rmcp-mux (manages all servers)
make mux-stop       # Stop rmcp-mux
make mux-restart    # Restart rmcp-mux
make mux-status     # Check status of all servers
make mux-tui        # Launch TUI dashboard
make mux-restart-service SERVICE=name  # Restart single service
make mux-logs       # Tail mux log
```

## Troubleshooting

### Protobuf Not Found

```
error: could not find `protoc`
```

Solution:
```bash
# macOS
brew install protobuf

# Or use Makefile
make setup-protoc
```

### Build Lock Conflict

```
Another build running (PID xxxx). Aborting.
```

Solution:
```bash
make unlock
```

### LanceDB Lock (rmcp-memex)

```
ERROR: Database is locked by another process.
```

The `rmcp_memex index` command now uses lance-only mode and works concurrently with the server. If you still see this error with older versions, stop the server first.

### Cargo Install Conflicts

If you have both crates.io and local versions:

```bash
# Uninstall crates.io version first
cargo uninstall loctree

# Then install from source
make install
```

## Platform Support

| Platform | Core | MCP Servers | Notes |
|----------|------|-------------|-------|
| macOS (Apple Silicon) | Full | Full | Primary development platform |
| macOS (Intel) | Full | Full | Tested |
| Linux (x86_64) | Full | Full | Tested |
| Linux (ARM64) | Full | Full | Tested |
| Windows | Partial | Partial | WSL recommended |

## Version Management

```bash
# Show current versions
make version-show

# Bump version (patch)
make version TYPE=patch SCOPE=all

# Bump with tag
make version TYPE=patch SCOPE=all TAG=1

# Bump, tag, and push
make version TYPE=patch SCOPE=all TAG=1 PUSH=1
```

## Updating

```bash
# From crates.io
cargo install loctree --force

# From source
git pull
make install
```

## Uninstalling

```bash
# Cargo-installed binaries
cargo uninstall loctree loctree-mcp rmcp-memex rmcp-mux

# Or manually
rm ~/.cargo/bin/loct
rm ~/.cargo/bin/loctree
rm ~/.cargo/bin/loctree-mcp
rm ~/.cargo/bin/rmcp_memex
rm ~/.cargo/bin/rmcp-mux

# Clean mux state
rm -rf ~/.rmcp_servers
```

---

Created by M&K (c)2025 The LibraxisAI Team
