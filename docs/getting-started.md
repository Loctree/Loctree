# Getting Started with loctree

5-minute quickstart to scanning a codebase, reading the artifacts, and wiring
Loctree into an AI workflow.

## Install

Choose one channel per machine:

```bash
# Fastest public path: CLI + MCP server
curl -fsSL https://loct.io/install.sh | sh

# Cargo, reproducible lockfile build
cargo install --locked loctree loctree-mcp

# npm (CLI only; macOS arm64/x64, Linux x64 glibc, Windows x64)
npm install -g loctree

# From source
git clone https://github.com/Loctree/loctree-ast.git
cd loctree-ast
make install
```

Verify the install:

```bash
loct --version
loctree --version
loctree-mcp --version
```

## First Scan

Run Loctree inside any project directory:

```bash
cd your-project
loct
```

Artifacts are written into your OS cache directory by default. In CI or when
you want repo-local output, set `LOCT_CACHE_DIR=.loctree`.

## Essential Commands

```bash
loct                              # Scan or refresh the cached snapshot
loct --for-ai                     # AI-optimized overview
loct slice src/App.tsx --consumers # File + deps + consumers
loct find useAuth                 # Find symbol definitions/usages
loct impact src/utils/api.ts      # Blast radius before refactor/delete
loct health                       # Quick health summary
loct dead --confidence high       # High-confidence dead exports
loct cycles                       # Circular imports
loct twins                        # Duplicate exports / dead parrots / barrel drift
loct audit                        # Full structural review
```

## Artifacts

After a scan, Loctree keeps four core files per project snapshot:

- `snapshot.json` — the dependency graph and file metadata
- `findings.json` — structural findings such as dead exports and cycles
- `agent.json` — AI-optimized overview with quick wins and health data
- `manifest.json` — artifact index for tooling

Query artifacts directly:

```bash
loct '.metadata'
loct '.files | length'
loct --findings | jq '.dead_exports[] | select(.confidence == "high")'
loct --agent-json | jq '.summary'
```

## MCP Server

`loctree-mcp` is the production MCP surface for AI agents. Start it via stdio:

```bash
loctree-mcp
```

Example Claude Desktop / Claude Code config:

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

The server exposes seven tools:

- `repo-view`
- `slice`
- `find`
- `impact`
- `focus`
- `tree`
- `follow`

See [integrations/mcp-server.md](integrations/mcp-server.md) for the full MCP contract.

## IDE / LSP

The public OSS workspace does not ship `loct lsp`. Editor integrations live in
the external `loctree-suite` project. The docs in [`docs/ide/`](ide/) are kept
as compatibility notes and setup pointers for that external surface.

## Next Steps

- Read [CLI Commands](cli/commands.md) for the full command surface
- Read [Use Cases](use-cases/README.md) for real-world analysis flows
- Read [dev/01_installation.md](dev/01_installation.md) if you are contributing to the workspace itself
