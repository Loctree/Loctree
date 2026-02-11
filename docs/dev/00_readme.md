# Loctree Suite - Developer Documentation

Technical documentation for developers and contributors.

## Contents

| Document | Description |
|----------|-------------|
| [INSTALLATION.md](INSTALLATION.md) | Complete installation guide with all methods |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Workspace structure and crate relationships |
| [BINARIES.md](BINARIES.md) | CLI reference for all binaries |

## Quick Links

### Getting Started

```bash
# Install core tools
cargo install loctree

# Or from source
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite
make install
```

### Crates

| Crate | Version | Description |
|-------|---------|-------------|
| [loctree](../../loctree_rs) | 0.7.4 | Core library + CLI |
| [loctree-mcp](../../loctree-mcp) | 0.1.15 | MCP server for AI agents |
| [rmcp-memex](../../rmcp-memex) | 0.1.11 | RAG/memory MCP server |
| [rmcp-mux](../../rmcp-mux) | 0.3.3 | MCP multiplexer |

### Binaries

| Binary | Purpose |
|--------|---------|
| `loct` | Primary CLI (recommended) |
| `loctree` | Full CLI (deprecated v0.9.0) |
| `loctree-mcp` | MCP server |
| `rmcp_memex` | Memory/RAG server |
| `rmcp-mux` | MCP multiplexer (single process manages all servers) |

## Development

```bash
# Build all
cargo build --workspace

# Test all
cargo test --workspace

# Format
cargo fmt --all

# Check
cargo clippy --workspace
```

## Related Documentation

- [README.md](../../README.md) - Project overview
- [CHANGELOG.md](../../CHANGELOG.md) - Version history
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guide
- [MIGRATION_0.7.0.md](../MIGRATION_0.7.0.md) - Migration guide

---

Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
