# loctree Documentation

AI-oriented codebase analyzer for detecting dead code, circular imports, and generating dependency graphs.

**Current version:** 0.8.3
**CLI command:** `loct` (old `loctree` is deprecated)

---

## Quick Links

- [Getting Started](getting-started.md)
- [CLI Commands](cli/commands.md)
- [CLI Options](cli/options.md)
- [IDE Integration](#ide-integration)
- [AI Agent Integration](#ai-agent-integration)
- [CI/CD Integration](integrations/ci-cd.md)
- [Use Cases](use-cases/README.md)
- [Advanced Topics](#advanced)

---

## Getting Started

### Installation

```bash
# From crates.io (recommended)
cargo install loctree

# From source
git clone https://github.com/Loctree/Loctree
cd loctree/loctree_rs
cargo install --path .
```

### First Scan

```bash
cd your-project
loct                          # Auto-detects stack, creates .loctree/
loct report --serve           # Interactive HTML report
loct --for-ai                 # AI-optimized hierarchical output
```

### Essential Commands

```bash
loct slice <file>             # Extract context for AI (deps + consumers)
loct health                   # Quick summary: cycles + dead + twins
loct dead --confidence high   # Find unused exports
loct cycles                   # Detect circular imports
loct twins                    # Semantic duplicates analysis
```

---

## Core Concepts

### Snapshot-Based Analysis

loctree operates on snapshots stored in `.loctree/`:

- **snapshot.json** - Complete graph data (imports, exports, LOC per file)
- **findings.json** - All detected issues (dead code, cycles, duplicates)
- **agent.json** - AI-optimized context bundle
- **manifest.json** - Index for tooling and AI agents

Scan once with `loct`, then query multiple times without re-parsing.

### Findings Categories

| Finding | Description | Command |
|---------|-------------|---------|
| **Dead Parrots** | Exports with 0 imports | `loct dead` |
| **Cycles** | Circular import chains | `loct cycles` |
| **Twins** | Semantic duplicates | `loct twins` |
| **Orphans** | Files with no imports/exports | `loct audit` |
| **Shadows** | Duplicate symbol definitions | `loct audit` |
| **Crowds** | Files with excessive connections | `loct audit` |

### Artifacts

All outputs are stored as artifacts in `.loctree/`:

```bash
loct                          # Creates snapshot + findings
loct report                   # Generates report.html
loct jq '.metadata'           # Query snapshot.json directly
```

---

## IDE Integration

Full LSP support for real-time dead code detection, cycle warnings, and code navigation.

| Editor | Documentation | Status |
|--------|---------------|--------|
| VSCode | [ide/vscode.md](ide/vscode.md) | ✅ Ready |
| Neovim | [ide/neovim.md](ide/neovim.md) | ✅ Ready |
| Any LSP client | [ide/lsp-protocol.md](ide/lsp-protocol.md) | ✅ Ready |

### Quick Setup

```bash
# Build LSP server
cd loctree_lsp
cargo build --release

# Binary at target/release/loctree-lsp
```

### Features

- **Diagnostics** - Dead exports, cycles, twins as warnings
- **Hover** - Import counts, consumer files
- **Go to Definition** - Resolve re-export chains
- **References** - Find all importers
- **Code Actions** - Quick fixes for dead code

---

## AI Agent Integration

### MCP Server

loctree provides an MCP (Model Context Protocol) server for AI agents.

**Full documentation:** [integrations/mcp-server.md](integrations/mcp-server.md)

**Location:** `loctree-mcp/`
**Status:** Production-ready

#### Setup

Add to your MCP config (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "loctree": {
      "command": "path/to/loctree-mcp",
      "args": ["--project", "/path/to/your/project"]
    }
  }
}
```

#### Available Tools

- `loctree_scan` - Full codebase scan
- `loctree_slice` - Extract focused context
- `loctree_query` - jq-style queries on snapshot
- `loctree_health` - Health summary

#### Use Cases

- **Context extraction** - Get relevant code for AI conversations
- **Duplicate detection** - Find existing components before creating new ones
- **Impact analysis** - Understand downstream effects of changes
- **Handler tracing** - Follow Tauri command pipelines

### AI-Optimized Output

```bash
loct --for-ai                 # Hierarchical JSON with quick wins
loct slice <file> --json      # Context bundle for AI agents
```

Output includes:
- Health score
- Quick wins (prioritized actions)
- Hub files (high-connectivity nodes)
- Dependency chains

---

## Advanced

### Architecture

**Core components:**
- `loctree_rs/` - Main analyzer (Rust)
- `loctree_lsp/` - LSP server
- `loctree-mcp/` - MCP server for AI agents
- `reports/` - HTML report renderer (Leptos SSR)

**Analysis flow:**
1. Auto-detect stack (Rust/TS/Python/Dart)
2. Parse imports/exports (language-specific)
3. Build dependency graph
4. Run detectors (dead code, cycles, twins)
5. Generate artifacts (snapshot, findings, reports)

### Multi-Language Support

| Language | Support | Parser |
|----------|---------|--------|
| Rust | Exceptional | Tree-sitter |
| TypeScript/JavaScript | Full | OXC |
| Python | Full | Custom |
| Go | Perfect | Tree-sitter |
| Dart/Flutter | Full | Tree-sitter |
| Svelte/Vue | Full | SFC + OXC |

### Query Mode

jq-compatible queries on snapshot data:

```bash
loct '.metadata'                              # Extract metadata
loct '.files | length'                        # Count files
loct '.edges[] | select(.from | contains("api"))' # Filter edges
loct -r '.summary.health_score'               # Raw output (no quotes)
loct -c '.dead_parrots'                       # Compact JSON
```

**Options:**
- `-r, --raw` - Raw output (no JSON quotes)
- `-c, --compact` - Compact one-line JSON
- `-e, --exit-status` - Exit 1 if result is false/null
- `--arg <name> <value>` - Bind string variable
- `--argjson <name> <json>` - Bind JSON variable

### Tauri Integration

Full command pipeline validation:

```bash
loct commands                 # Missing/unregistered/unused handlers
loct trace <handler>          # Follow FE invoke → BE handler
loct coverage --handlers      # Test coverage for handlers
```

**Detects:**
- Missing handlers (frontend invokes non-existent backend)
- Unregistered handlers (`#[tauri::command]` not in `generate_handler![]`)
- Unused handlers (backend defined but never called)
- React.lazy() dynamic imports

### Library Mode

For libraries/frameworks with public APIs:

```bash
loct --library-mode
```

**Features:**
- Auto-detects npm `exports` field
- Respects Python `__all__` declarations
- Ignores example/demo directories
- Excludes public APIs from dead code detection

**Customization:**
```toml
# .loctree/config.toml
library_mode = true
library_example_globs = ["examples/*", "demos/*", "playground/*"]
```

### CI Integration

Full documentation: [integrations/ci-cd.md](integrations/ci-cd.md)

```bash
# Fail build on critical issues
loct lint --fail

# SARIF output for GitHub/GitLab
loct lint --sarif > results.sarif

# JSON output for custom processing
loct health --json
loct coverage --json
```

Supports: GitHub Actions, GitLab CI, CircleCI, pre-commit hooks.

### Watch Mode

```bash
loct watch                    # Auto-refresh on file changes
loct watch --serve            # Live reload HTML report
```

### Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for:
- Development setup
- Testing guidelines
- Adding language support
- Release process

---

## Additional Resources

- **Changelog:** [CHANGELOG.md](../CHANGELOG.md)
- **Main README:** [../README.md](../README.md)
- **Crates.io:** [loctree](https://crates.io/crates/loctree)
- **Repository:** [github.com/Loctree/Loctree](https://github.com/Loctree/Loctree)

---

Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
Co-Authored-By: [Maciej](mailto:void@div0.space) & [Klaudiusz](mailto:the1st@whoai.am)
