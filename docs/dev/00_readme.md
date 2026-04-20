# Loctree Developer Docs

Technical notes for contributors working inside the public `loctree-ast`
workspace.

## Primary Docs

| Document | Purpose |
|----------|---------|
| [01_installation.md](01_installation.md) | Install, verify, and update the workspace |
| [02_architecture.md](02_architecture.md) | Current workspace shape and blast-radius hubs |
| [03_cli_reference.md](03_cli_reference.md) | CLI contract and developer reference |

## Quick Start

```bash
git clone https://github.com/Loctree/loctree-ast.git
cd loctree-ast
make install
make precheck
```

## Workspace Crates

The public workspace currently ships these members:

| Crate | Version | Purpose |
|-------|---------|---------|
| `loctree` | 0.8.17 | Core analyzer + CLI (`loct`, `loctree`) |
| `loctree-mcp` | 0.8.17 | MCP server |
| `report-leptos` | 0.8.17 | HTML report renderer |
| `rmcp-common` | 0.8.17 | Shared MCP/common utilities |

## Binaries

| Binary | Purpose |
|--------|---------|
| `loct` | Canonical CLI |
| `loctree` | Quiet compatibility alias for `loct` |
| `loctree-mcp` | MCP server |

## External Surfaces

These are part of the broader Loctree ecosystem, but not members of this
workspace:

- `loctree-suite` for editor/LSP surfaces
- thin release repos: `Loctree/loct` and `Loctree/loctree-mcp`
- Homebrew taps: `Loctree/homebrew-cli` and `Loctree/homebrew-mcp`

## Core Gates

```bash
make precheck   # fast repo-wide check
make check      # fmt + clippy + cargo check + semgrep
make test       # workspace tests
```
