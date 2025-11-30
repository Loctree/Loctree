# loctree — AI-oriented Project Analyzer

[![CI](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml/badge.svg)](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml)
[![Loctree CI](https://img.shields.io/github/actions/workflow/status/LibraxisAI/loctree/loctree-ci.yml?label=loctree%20ci&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://github.com/LibraxisAI/loctree/actions/workflows/loctree-ci.yml)
[![Crates.io](https://img.shields.io/crates/v/loctree.svg)](https://crates.io/crates/loctree)
[![Downloads](https://img.shields.io/crates/d/loctree.svg)](https://crates.io/crates/loctree)
[![docs.rs](https://docs.rs/loctree/badge.svg)](https://docs.rs/loctree)
[![Semgrep](https://img.shields.io/badge/semgrep-scanned-blue?logo=semgrep)](https://semgrep.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8?style=flat&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://crates.io/crates/loctree)

**loctree** is a static analysis tool designed for AI agents and developers building production-ready software. It helps overcome the common AI tendency to generate excessive artifacts that lead to re-export cascades, circular imports, and spaghetti dependencies.

**Scan once, slice many.** Run `loctree` to capture your project's true structure, then use `loctree slice` to extract focused context for any AI conversation.

## Quick Start

```bash
# Install from crates.io
cargo install loctree

# Or via install script
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh

# Scan your project (auto-detects stack)
cd your-project
loctree

# Extract context for AI agents
loctree slice src/components/ChatPanel.tsx --consumers --json

# Get AI-optimized hierarchical output
loctree --for-ai

# Find circular imports
loctree -A --circular

# Trace a Tauri handler through the entire pipeline
loctree trace get_user
```

## Why loctree?

AI agents face **context drift** — without understanding the real dependency graph, they generate new components instead of reusing existing ones, create barrel files that re-export everything, and build circular dependencies that compile but break at runtime.

loctree solves this by:

- **Detecting what you already have** — Find existing components before creating duplicates
- **Exposing hidden dependencies** — See circular imports and orphaned code
- **Slicing relevant context** — Extract just what an AI needs for a specific task
- **Tracing handler pipelines** — Follow Tauri commands from frontend invoke to backend handler
- **CI-friendly checks** — Fail builds on missing handlers, ghost events, or dependency cycles

## Core Features

### Holographic Slice (`slice`)

Extract 3-layer context for any file — perfect for AI conversations:

```bash
loctree slice src/App.tsx --consumers
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

### Handler Trace (`trace`)

Follow a Tauri command through the entire pipeline:

```bash
loctree trace get_user
```

```
Handler: get_user

Registration:
  src-tauri/src/main.rs:45  generate_handler![get_user, ...]

Implementation:
  src-tauri/src/commands/user.rs:12  #[tauri::command] pub async fn get_user(...)

Frontend Invocations:
  src/hooks/useUser.ts:8     invoke('get_user', { id })
  src/components/Profile.tsx:23  invoke('get_user', { id: props.userId })
```

### AI-Optimized Output (`--for-ai`)

Hierarchical JSON designed for AI agents with quick wins and hub files:

```bash
loctree --for-ai
```

```json
{
  "summary": {
    "files_analyzed": 127,
    "missing_handlers": 2,
    "unregistered_handlers": 1,
    "circular_imports": 0
  },
  "quick_wins": [
    {
      "priority": 1,
      "action": "Add missing handler",
      "target": "delete_session",
      "location": "src/api/auth.ts:45",
      "impact": "Fixes runtime error"
    }
  ],
  "hub_files": [
    { "path": "src/api/index.ts", "connections": 23 }
  ]
}
```

### Auto-Detect Stack

loctree automatically detects your project type:

| Marker | Stack | Auto-Ignores | Extensions |
|--------|-------|--------------|------------|
| `Cargo.toml` | Rust | `target/` | `.rs` |
| `tsconfig.json` | TypeScript | `node_modules/` | `.ts`, `.tsx` |
| `pyproject.toml` | Python | `.venv/`, `__pycache__/` | `.py` |
| `src-tauri/` | Tauri | All above | `.ts`, `.tsx`, `.rs` |
| `vite.config.*` | Vite | `dist/` | Auto |

### Tauri Command Coverage

For Tauri projects, loctree validates the entire command pipeline:

```bash
loctree -A --preset-tauri
```

- **Missing handlers** — Frontend invokes commands that don't exist in backend
- **Unregistered handlers** — Backend has `#[tauri::command]` but not in `generate_handler![]`
- **Unused handlers** — Registered handlers never invoked from frontend
- **React lazy detection** — Tracks `React.lazy()` dynamic imports

### Tree Mode

Fast directory tree with LOC counts:

```bash
loctree --tree src/

# Find build artifacts to clean
loctree --tree --find-artifacts
# Output: /path/to/node_modules, /path/to/target, ...

# Show gitignored files
loctree --tree --show-ignored
```

### Janitor Mode Tools

Find problems before they become tech debt:

```bash
# Check if similar component exists before creating
loctree -A --check ChatSurface
# Found: ChatPanel (distance: 2), ChatWindow (distance: 3)

# Find potentially unused exports
loctree -A --dead --confidence high

# Detect circular import cycles
loctree -A --circular

# Analyze impact of changing a file
loctree -A --impact src/utils/api.ts

# Find a symbol across the codebase
loctree -A --symbol useAuth

# List entry points
loctree -A --entrypoints
```

### HTML Reports

Generate interactive HTML reports with dependency graphs:

```bash
loctree -A --html-report report.html --graph
```

Features:
- Interactive Cytoscape.js dependency graphs
- Tabbed navigation (Duplicates, Cascades, Commands, Graph)
- AI Summary panel with quick wins
- Dark mode support

### CI Pipeline Checks

```bash
# Fail if frontend invokes missing backend handlers
loctree -A --fail-on-missing-handlers

# Fail if events lack listeners/emitters
loctree -A --fail-on-ghost-events

# Fail if listener/await races detected
loctree -A --fail-on-races

# SARIF 2.1.0 output for GitHub/GitLab
loctree -A --sarif > results.sarif
```

## CLI Reference

```
loctree (Rust) - AI-oriented Project Analyzer

Modes:
  (default)                 Scan and save snapshot to .loctree/snapshot.json
  slice <file>              Holographic slice for AI agents
  trace <handler>           Trace handler through pipeline (Tauri)
  --tree                    Directory tree with LOC counts
  -A, --analyze-imports     Full import/export analysis
  --for-ai                  AI-optimized hierarchical JSON

Slice options:
  --consumers               Include files that import the target
  --json                    JSON output for piping to AI

Trace options:
  trace <name>              Handler name to trace

Tree options:
  --find-artifacts          Find build dirs (node_modules, target, etc.)
  --show-ignored            Show only gitignored files
  --summary[=N]             Show totals + top N large files

Analyzer options (-A):
  --circular                Find circular imports
  --dead                    Find unused exports
  --entrypoints             List entry points
  --impact <file>           What imports this file
  --symbol <name>           Search for symbol
  --check <query>           Find similar components
  --sarif                   SARIF 2.1.0 output
  --html-report <file>      Generate HTML report
  --graph                   Include dependency graph in HTML

Presets:
  --preset-tauri            Tauri defaults (ts,tsx,rs + handler checks)
  --preset-styles           CSS/Tailwind defaults

Pipeline checks:
  --fail-on-missing-handlers
  --fail-on-ghost-events
  --fail-on-races

Common:
  -g, --gitignore           Respect .gitignore
  -I, --ignore <path>       Ignore path (repeatable)
  --full-scan               Re-analyze all (ignore cache)
  --verbose                 Detailed progress
  --help-full               Full reference
```

## Installation

### From crates.io (Recommended)

```bash
cargo install loctree
```

### From source

```bash
git clone https://github.com/LibraxisAI/loctree
cd loctree/loctree_rs
cargo install --path .
```

### Install script

```bash
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh
```

## Project Structure

```
.
├── loctree_rs/           # Main Rust crate (crates.io: loctree)
│   ├── src/
│   │   ├── main.rs       # CLI entry point
│   │   ├── slicer.rs     # Holographic slice
│   │   ├── detect.rs     # Auto-detect stack
│   │   ├── tree.rs       # Tree mode
│   │   ├── snapshot.rs   # Incremental scanning
│   │   └── analyzer/     # Import/export analysis
│   │       ├── trace.rs  # Handler tracing
│   │       ├── coverage.rs # Tauri command coverage
│   │       ├── for_ai.rs # AI-optimized output
│   │       └── ...
│   └── Cargo.toml
├── reports/              # HTML report renderer (crates.io: report-leptos)
│   └── src/
│       ├── lib.rs        # Leptos SSR
│       └── components/   # Report UI components
├── landing/              # Landing page (Leptos SPA)
├── tools/                # Install scripts, git hooks
└── .github/workflows/    # CI configuration
```

## Development

```bash
# Setup git hooks
./tools/githooks/setup.sh

# Run tests
cd loctree_rs && cargo test

# Run all checks
cargo fmt && cargo clippy -- -D warnings && cargo test
```

## Philosophy

> "The goal isn't 'make it work'. The goal is: we know WHY it works (or doesn't)."

loctree makes the invisible visible:
- **Import graphs** show real dependencies, not assumed ones
- **Dead code detection** finds what you forgot you wrote
- **Handler tracing** shows the full pipeline, not just one side
- **Context slicing** gives AI agents exactly what they need

## Badge for Your Project

Show that your project uses loctree:

```markdown
[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8?style=flat&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://crates.io/crates/loctree)
```

[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8?style=flat&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://crates.io/crates/loctree)

Or use the SVG badge:

```markdown
[![loctree](https://raw.githubusercontent.com/LibraxisAI/loctree/main/assets/loctree-badge.svg)](https://crates.io/crates/loctree)
```

[![loctree](https://raw.githubusercontent.com/LibraxisAI/loctree/main/assets/loctree-badge.svg)](https://crates.io/crates/loctree)

## License

MIT — use it, fork it, improve it. See [LICENSE](LICENSE).

---

**Created by M&K (c)2025 The LibraxisAI Team**
