# loctree — AI-oriented Project Analyzer

[![CI](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml/badge.svg)](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/loctree.svg)](https://crates.io/crates/loctree)
[![Downloads](https://img.shields.io/crates/d/loctree.svg)](https://crates.io/crates/loctree)
[![docs.rs](https://docs.rs/loctree/badge.svg)](https://docs.rs/loctree)
[![Semgrep](https://img.shields.io/badge/semgrep-scanned-blue?logo=semgrep)](https://semgrep.dev)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8?style=flat&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://loctree.io)

**loctree** is a static analysis tool designed for AI agents and non-programmers building production-ready software. It helps overcome the common AI tendency to generate excessive artifacts that lead to re-export cascades, circular imports, and spaghetti dependencies.

**Scan once, slice many.** Run `loctree` to capture your project's true structure, then use `loctree slice` to extract focused context for any AI conversation.

## Why loctree?

AI agents and vibe-coders face a common problem: **context drift**. Without understanding the real dependency graph, AI generates new components instead of reusing existing ones, creates barrel files that re-export everything, and builds circular dependencies that compile but break at runtime.

loctree solves this by:

1. **Detecting what you already have** — Find existing components before creating duplicates
2. **Exposing hidden dependencies** — See circular imports and orphaned code
3. **Slicing relevant context** — Extract just what an AI needs for a specific task
4. **CI-friendly checks** — Fail builds on missing handlers, ghost events, or dependency cycles

## Quick Start

```bash
# Install (Rust binary)
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh

# Scan your project (creates .loctree/snapshot.json)
cd your-project
loctree

# Extract context for AI agents
loctree slice src/components/ChatPanel.tsx --consumers --json | claude

# Find circular imports
loctree -A --circular

# Check for dead exports
loctree -A --dead --confidence high

# CI check: fail if FE invokes missing BE handlers
loctree -A --fail-on-missing-handlers
```

## Core Features

### Holographic Slice (`slice` command)

Extract 3-layer context for any file, perfect for AI conversations:

```bash
loctree slice src/App.tsx --consumers
```

Output:
```
Slice for: src/App.tsx

Core (1 files, 150 LOC):
  src/App.tsx (150 LOC, ts)

Deps (3 files, 420 LOC):
  [d1] src/hooks/useAuth.ts (80 LOC, ts)
    [d2] src/contexts/AuthContext.tsx (200 LOC, ts)
    [d2] src/utils/api.ts (140 LOC, ts)

Consumers (2 files, 180 LOC):
  src/main.tsx (30 LOC, ts)
  src/routes/index.tsx (150 LOC, ts)

Total: 6 files, 750 LOC
```

Pipe directly to AI:
```bash
loctree slice src/features/chat/ChatPanel.tsx --json | claude "refactor this to use React Query"
```

### Auto-Detect Stack

loctree automatically detects your project type and configures sensible defaults:

| Marker | Detected As | Auto-Ignores |
|--------|-------------|--------------|
| `Cargo.toml` | Rust | `target/` |
| `tsconfig.json` | TypeScript | `node_modules/` |
| `pyproject.toml` | Python | `.venv/`, `__pycache__/` |
| `src-tauri/` | Tauri | All of the above |

### Janitor Mode Tools

Find problems before they become tech debt:

```bash
# Before creating a new component, check if similar exists
loctree -A --check ChatSurface
# Found: ChatPanel (distance: 2), ChatWindow (distance: 3)

# Find potentially unused exports
loctree -A --dead

# Detect circular import cycles
loctree -A --circular

# List entry points (main functions)
loctree -A --entrypoints

# Analyze impact of changing a file
loctree -A --impact src/utils/api.ts

# Find a symbol across the codebase
loctree -A --symbol useAuth
```

### Incremental Scanning

After the first scan, loctree uses file modification times to skip unchanged files:

```
$ loctree
[loctree][detect] Detected: Tauri + Vite
[loctree][progress] 32 cached, 1 fresh (touched: src/App.tsx)
```

Use `--full-scan` to force re-analysis of everything.

### CI Pipeline Checks

```bash
# Fail if frontend invokes backend handlers that don't exist
loctree -A --fail-on-missing-handlers

# Fail if events are emitted but never listened to (or vice versa)
loctree -A --fail-on-ghost-events

# Fail if potential race conditions detected in event listeners
loctree -A --fail-on-races

# Output in SARIF 2.1.0 format for GitHub/GitLab integration
loctree -A --sarif > results.sarif
```

## CLI Reference

```
loctree (Rust) - AI-oriented Project Analyzer

Modes:
  init (default)            Scan and save snapshot to .loctree/snapshot.json
  slice <file>              Holographic slice: extract context for AI agents
  -A, --analyze-imports     Import/export analyzer

Slice options:
  --consumers               Include files that import the target
  --json                    Output as JSON (for piping to AI)

Analyzer options (-A):
  --circular                Find circular imports
  --entrypoints             List entry points
  --dead                    List potentially unused exports
  --sarif                   SARIF 2.1.0 output for CI
  --check <query>           Find similar existing components
  --impact <file>           Show what imports the target
  --symbol <name>           Search for symbol

Pipeline checks:
  --fail-on-missing-handlers
  --fail-on-ghost-events
  --fail-on-races

Common:
  --gitignore, -g           Respect .gitignore
  --full-scan               Ignore mtime cache, re-analyze all
  --verbose                 Show detailed progress
```

Full reference: `loctree --help-full`

## Installation

### Rust (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh
```

### From source

```bash
git clone https://github.com/LibraxisAI/loctree
cd loctree/loctree_rs
cargo install --path .
```

### Alternative runtimes

Node.js and Python wrappers are available for environments where Rust isn't practical:

```bash
# Node.js
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_node.sh | sh

# Python
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_py.sh | sh
```

## Project Structure

```
.
├── loctree_rs/          # Rust crate (primary)
│   ├── src/
│   │   ├── main.rs      # CLI entry point
│   │   ├── slicer.rs    # Holographic slice implementation
│   │   ├── detect.rs    # Auto-detect stack
│   │   ├── snapshot.rs  # Snapshot persistence
│   │   └── analyzer/    # Import/export analysis
│   └── Cargo.toml
├── loctree.mjs          # Node.js wrapper
├── loctree.py           # Python wrapper
├── tools/               # Installers and test runners
└── .github/workflows/   # CI configuration
```

## Development

```bash
# Setup git hooks
./tools/setup_hooks.sh

# Run tests
cd loctree_rs && cargo test

# Run all checks (fmt, clippy, tests, semgrep)
cargo fmt && cargo clippy -- -D warnings && cargo test
cd .. && semgrep scan --config auto --config .semgrep.yaml loctree_rs/src/
```

## Philosophy

> "The goal isn't 'make it work'. The goal is: we know WHY it works (or doesn't)."

loctree embodies this principle by making the invisible visible:
- **Import graphs** show real dependencies, not assumed ones
- **Dead code detection** finds what you forgot you wrote
- **Circular import detection** catches runtime bombs before they explode
- **Context slicing** gives AI agents exactly what they need, no more

## Badge for Your Project

Show that your project is analyzed with loctree:

```markdown
[![loctree](https://raw.githubusercontent.com/LibraxisAI/loctree/main/assets/loctree-badge.svg)](https://loctree.io)
```

[![loctree](https://raw.githubusercontent.com/LibraxisAI/loctree/main/assets/loctree-badge.svg)](https://loctree.io)

## License

MIT — use it, fork it, improve it. See [LICENSE](LICENSE).

---

**Created by M&K (c)2025 The LibraxisAI Team**
