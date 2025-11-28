# loctree — AI-oriented Project Analyzer

[![CI](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml/badge.svg)](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**loctree** is a static analysis tool designed for AI agents and non-programmers building production-ready software. It helps overcome the common AI tendency to generate excessive artifacts that lead to re-export cascades, circular imports, and spaghetti dependencies.

**Scan once, slice many.** Run `loctree` to capture your project's true structure, then use `loctree slice` to extract focused context for any AI conversation.

## Why loctree?

AI agents and vibe-coders face a common problem: **context drift**. Without understanding the real dependency graph, AI generates new components instead of reusing existing ones, creates barrel files that re-export everything, and builds circular dependencies that compile but break at runtime.

loctree solves this by:

1. **Detecting what you already have** — Find existing components before creating duplicates
2. **Exposing hidden dependencies** — See circular imports and orphaned code
3. **Slicing relevant context** — Extract just what an AI needs for a specific task
4. **CI-friendly checks** — Fail builds on missing handlers, ghost events, or dependency cycles

1. **Detecting what you already have** — Find existing components before creating duplicates
2. **Exposing hidden dependencies** — See circular imports and orphaned code
3. **Slicing relevant context** — Extract just what an AI needs for a specific task
4. **CI-friendly checks** — Fail builds on missing handlers, ghost events, or dependency cycles

## Quick Start


# Extract context for AI agents
loctree slice src/components/ChatPanel.tsx --consumers --json | claude

# Find circular imports
loctree -A --circular

# Check for dead exports
loctree -A --dead --confidence high

# CI check: fail if FE invokes missing BE handlers
loctree -A --fail-on-missing-handlers

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

## License

MIT — use it, fork it, improve it. See [LICENSE](LICENSE).

---

**Created by M&K (c)2025 The LibraxisAI Team**
