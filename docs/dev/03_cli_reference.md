# Loctree - CLI Reference

Truthful reference for the public Loctree command surface in this repo.

## loct

`loct` is the canonical CLI. It auto-scans by default, writes artifacts to the
user cache dir (override with `LOCT_CACHE_DIR`), and lets you query the same
snapshot through focused commands.

### Everyday usage

```bash
loct                              # Default auto-scan for current directory
loct /path/to/project             # Auto-scan a specific project
loct --fresh                      # Force rescan even if snapshot exists
loct --for-ai                     # AI context bundle (JSONL)
loct --agent-json                 # Single agent bundle JSON
```

### Query mode

```bash
loct '.metadata'
loct '.files | length'
loct '.summary.health_score'
loct '.dead_parrots[]'
loct '.cycles[]'
```

### Core commands

```bash
loct findings                     # Canonical findings JSON
loct findings --summary           # Summary JSON for CI / status checks
loct slice src/App.tsx --consumers
loct find useAuth
loct impact src/utils/api.ts
loct health
loct dead --confidence high
loct cycles
loct twins
loct audit
loct doctor
loct lint --fail --sarif > loctree.sarif
loct report --serve --port 4173
```

### Common flags

| Flag | Description |
|------|-------------|
| `--fresh` | Force rescan even if snapshot exists |
| `--json` | JSON output on commands that support it |
| `--for-ai` | AI-optimized JSONL stream |
| `--agent-json` | One-shot agent bundle JSON |
| `--quiet`, `-q` | Suppress non-essential output |
| `--verbose` | Show detailed progress |
| `--fail-stale` | Fail if cached snapshot is stale |

Notes:
- Prefer `loct findings` over the legacy bare `loct --findings` shortcut in new docs and scripts.
- Use `loct findings --summary` for summary JSON. The `--summary` flag still belongs on commands like `loct tree`.

## loctree

`loctree` is the quiet compatibility alias for `loct`. Keep using `loct` in new
examples; use `loctree` only when preserving older scripts or operator muscle
memory.

## loctree-mcp

`loctree-mcp` is the production MCP server for this workspace. It runs over
stdio and auto-scans a project on first use when no snapshot exists.

### Usage

```bash
loctree-mcp
loctree-mcp --version
```

### MCP tools

| Tool | Description |
|------|-------------|
| `repo-view` | Repo overview: files, LOC, languages, health, top hubs |
| `slice` | File + dependencies + consumers in one call |
| `find` | Symbol search and reverse-import lookups |
| `impact` | Direct + transitive consumer blast radius |
| `focus` | Module/directory deep-dive |
| `tree` | Directory tree with LOC counts |
| `follow` | Structural signals: dead, cycles, twins, hotspots, trace, pipelines |

### Configuration

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

### Recommended agent flow

```text
repo-view -> focus -> slice -> impact -> find -> follow
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Command reported findings or general runtime failure |
| `2` | Invalid arguments / usage error |
| `101` | Rust panic |

## Cross-check sources

When this page and the runtime ever disagree, trust these repo-owned sources:

- `loctree_rs/src/bin/loct.rs`
- `loctree_rs/src/cli/command/help.rs`
- `loctree_rs/src/cli/parser/core.rs`
- `loctree-mcp/src/main.rs`

---

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Loctree Team
