<p align="center">
  <img src="https://loct.io/assets/loctree-logo.png" width="128" alt="loctree logo"/>
</p>

<h1 align="center">loctree</h1>

<p align="center">
  <strong>Scan once, query everything.</strong><br/>
  AI-oriented static analysis for dead exports, circular imports, dependency graphs, and holographic context slices.
</p>

<p align="center">
  <a href="https://crates.io/crates/loctree"><img src="https://img.shields.io/crates/v/loctree.svg" alt="crates.io"/></a>
  <a href="https://crates.io/crates/loctree"><img src="https://img.shields.io/crates/d/loctree.svg" alt="downloads"/></a>
  <a href="https://docs.rs/loctree"><img src="https://docs.rs/loctree/badge.svg" alt="docs.rs"/></a>
  <a href="https://github.com/Loctree/Loctree/actions/workflows/ci.yml"><img src="https://github.com/Loctree/Loctree/actions/workflows/ci.yml/badge.svg" alt="CI"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg" alt="License"/></a>
</p>

---

## Install

```bash
curl -fsSL https://loct.io/install.sh | sh   # One-liner (installs from crates.io)
```

Or directly via Cargo:

```bash
cargo install loctree        # CLI: loct, loctree
cargo install loctree-mcp    # MCP server for AI agents
```

## Quick Start

Artifacts are stored in your OS cache dir by default (override via `LOCT_CACHE_DIR`).

```bash
loct                              # Scan project, write cached artifacts
loct --for-ai                     # AI-optimized overview (health, hubs, quick wins)
loct slice src/App.tsx --consumers # Context: file + deps + consumers
loct find useAuth                  # Find symbol definitions
loct find 'Snapshot FileAnalysis'  # Cross-match: where terms meet
loct impact src/utils/api.ts       # What breaks if you change this?
loct health                        # Quick summary: cycles + dead + twins
loct dead --confidence high        # Unused exports
loct cycles                        # Circular imports
loct twins                         # Dead parrots + duplicates + barrel chaos
loct audit                         # Full codebase review
```

## What It Does

loctree captures your project's real dependency graph in a single scan, then answers structural questions instantly from the snapshot. Designed for AI agents that need focused context without reading every file.

**Core capabilities:**

- **Holographic Slice** - extract file + dependencies + consumers in one call
- **Cross-Match Search** - find where multiple terms co-occur (not flat grep)
- **Dead Export Detection** - find unused exports across JS/TS, Python, Rust, Go, Dart
- **Circular Import Detection** - Tarjan's SCC algorithm catches runtime bombs
- **Handler Tracing** - follow Tauri commands through the entire FE/BE pipeline
- **Impact Analysis** - see what breaks before you delete or refactor
- **jq Queries** - query snapshot data with jq syntax (`loct '.files | length'`)

## MCP Server

loctree ships as an MCP server for seamless AI agent integration:

```bash
loctree-mcp    # Start via stdio (configure in your MCP client)
```

Tools: `repo-view`, `slice`, `find`, `impact`, `focus`, `tree`. Each tool accepts a `project` parameter - auto-scans on first use, caches snapshots in RAM.

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

## Language Support

| Language | Dead Export Accuracy | Notes |
|----------|---------------------|-------|
| **Rust** | ~0% FP | Tested on rust-lang/rust (35K files) |
| **Go** | ~0% FP | Tested on golang/go (17K files) |
| **TypeScript/JavaScript** | ~10-20% FP | JSX/TSX, React patterns, Flow, WeakMap |
| **Python** | ~20% FP | Library mode, `__all__`, stdlib detection |
| **Svelte** | <15% FP | Template analysis, .d.ts re-exports |
| **Vue** | ~15% FP | SFC support, Composition & Options API |
| **Dart/Flutter** | Full | pubspec.yaml detection |

Auto-detects stack from `Cargo.toml`, `tsconfig.json`, `pyproject.toml`, `pubspec.yaml`, `src-tauri/`.

## Holographic Slice

Extract 3-layer context for any file:

```bash
loct slice src/App.tsx --consumers
```

```
Slice for: src/App.tsx

Core (1 files, 150 LOC):
  src/App.tsx (150 LOC, ts)

Deps (3 files, 420 LOC):
  [d1] src/hooks/useAuth.ts (80 LOC)
    [d2] src/contexts/AuthContext.tsx (200 LOC)
    [d2] src/utils/api.ts (140 LOC)

Consumers (2 files, 180 LOC):
  src/main.tsx (30 LOC)
  src/routes/index.tsx (150 LOC)

Total: 6 files, 750 LOC
```

## Cross-Match Search

Multi-term queries show where terms meet, not flat OR:

```bash
loct find 'Snapshot FileAnalysis'
```

```
=== Cross-Match Files (9) ===
  src/snapshot.rs: Snapshot(6), FileAnalysis(4)
  src/slicer.rs: Snapshot(2), FileAnalysis(3)
  ...

=== Symbol Matches (222 in cross-match files) ===
  src/snapshot.rs:20 - Snapshot [struct]
  src/types.rs:15 - FileAnalysis [struct]
  ...

=== Parameter Matches (4 cross-matched) ===
  src/slicer.rs:45 - snapshot: &Snapshot in build_slice(analyses: &[FileAnalysis])
```

## jq Queries

Query snapshot data directly:

```bash
loct '.dead_parrots'                           # Dead code findings
loct '.files | length'                         # Count files
loct '.edges[] | select(.from | contains("api"))' # Filter edges
loct '.summary.health_score'                   # Health score
```

## CI Integration

```bash
loct lint --fail --sarif > results.sarif    # SARIF for GitHub/GitLab
loct --findings | jq '.dead_parrots | length'  # Check dead code count
loct doctor && echo 'Clean'                 # Health gate
```

## Crates

| Crate | Description |
|-------|-------------|
| [`loctree`](https://crates.io/crates/loctree) | Core analyzer + CLI (`loct`, `loctree`) |
| [`report-leptos`](https://crates.io/crates/report-leptos) | HTML report renderer (Leptos SSR) |
| [`loctree-mcp`](https://crates.io/crates/loctree-mcp) | MCP server for AI agents |

## Development

```bash
make precheck        # fmt + clippy + check (run before push)
make install         # Install loct + loctree-mcp
make test            # Run all workspace tests
make publish         # Cascade publish to crates.io
```

## Badge

```markdown
[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8?style=flat&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://crates.io/crates/loctree)
```

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

---

VibeCrafted with AI Agents (c)2025-2026 VetCoders
