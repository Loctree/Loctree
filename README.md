# loctree — AI-oriented Project Analyzer

[![CI](https://github.com/Loctree/Loctree/actions/workflows/ci.yml/badge.svg)](https://github.com/Loctree/Loctree/actions/workflows/ci.yml)
[![Loctree CI](https://img.shields.io/github/actions/workflow/status/Loctree/Loctree/loctree-ci.yml?label=loctree%20ci&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://github.com/Loctree/Loctree/actions/workflows/loctree-ci.yml)
[![Crates.io](https://img.shields.io/crates/v/loctree.svg)](https://crates.io/crates/loctree)
[![Downloads](https://img.shields.io/crates/d/loctree.svg)](https://crates.io/crates/loctree)
[![docs.rs](https://docs.rs/loctree/badge.svg)](https://docs.rs/loctree)
[![Semgrep](https://img.shields.io/badge/semgrep-scanned-blue?logo=semgrep)](https://semgrep.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![loctree](https://img.shields.io/badge/analyzed_with-loctree-a8a8a8?style=flat&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxNiIgaGVpZ2h0PSIxNiIgdmlld0JveD0iMCAwIDE2IDE2Ij48cmVjdCB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIGZpbGw9IiMwMDAiLz48dGV4dCB4PSI4IiB5PSIxMiIgZm9udC1mYW1pbHk9Im1vbm9zcGFjZSIgZm9udC1zaXplPSIxMCIgZmlsbD0iI2E4YThhOCIgdGV4dC1hbmNob3I9Im1pZGRsZSI+TDwvdGV4dD48L3N2Zz4=)](https://crates.io/crates/loctree)

**loctree** is a static analysis tool designed for AI agents and developers building production-ready software. It helps overcome the common AI tendency to generate excessive artifacts that lead to re-export cascades, circular imports, and spaghetti dependencies.

**Scan once, slice many.** Run `loct` to capture your project's true structure, then use `loct slice` to extract focused context for any AI conversation.

## What's New in 0.8.4

**Security & Analysis Improvements:**
- **sys.modules Detection** - Python dead code analysis now detects monkey-patching via `sys.modules`
- **Multi-Query Search** - `loct find` now supports regex and `foo|bar` patterns for faster symbol lookup
- **Parameter Indexing** - Function parameters are now indexed for comprehensive search

**0.8.1-0.8.3 Highlights:**
- **React lazy() Fix** - Dynamic imports via `lazy(() => import('./Component'))` correctly tracked
- **Dual License** - MIT OR Apache-2.0 (standard for Rust ecosystem)
- **Auto-Snapshot** - All commands work immediately without manual scan

### What's in 0.7.0

**Artifact-First Architecture** - mental model centered around artifacts:

- `.loctree/snapshot.json` - Complete graph data (imports, exports, LOC per file)
- `.loctree/findings.json` - All detected issues (dead code, cycles, duplicates)
- `.loctree/agent.json` - AI-optimized context bundle
- `.loctree/manifest.json` - Index for tooling and AI agents

**Query First** - Use jq-style queries on artifacts:

```bash
loct '.dead_parrots'              # Show dead code
loct '.summary.health_score'      # Get health score
loct '.files | length'            # Count files
```

## Quick Start

```bash
# Install via Homebrew (macOS/Linux) - PR pending
brew install loctree  # Coming soon!

# Or install from crates.io
cargo install loctree

# Scan your project (auto-detects stack)
cd your-project
loct

# View interactive report (HTML + graph)
loct report --serve

# Extract context for AI agents
loct slice src/components/ChatPanel.tsx --consumers --json

# Get AI-optimized hierarchical output
loct --for-ai

# Find circular imports
loct cycles

# Semantic duplicate analysis (dead parrots, twins, barrel chaos)
loct twins

# Detect dead exports
loct dead --confidence high

# Quick health check (cycles + dead + twins summary)
loct health

# Full codebase audit (cycles + dead + twins + orphans + shadows + crowds)
loct audit

# Verify tree-shaking in production bundles
loct dist dist/bundle.js.map src/

# jq-style queries on snapshot data (NEW!)
loct '.metadata'                    # extract metadata
loct '.files | length'              # count files
loct '.edges[] | select(.from | contains("api"))'  # filter
```

## Recommended Commands (by Real-World Testing)

Based on extensive testing across TypeScript, Rust, Python, and Tauri codebases:

| Command | Rating | Best For |
|---------|--------|----------|
| `loct slice <file>` | ⭐⭐⭐⭐⭐ | AI context extraction - shows deps + consumers |
| `loct trace <handler>` | ⭐⭐⭐⭐⭐ | Tauri apps - FE→BE pipeline visualization |
| `loct find <symbol>` | ⭐⭐⭐⭐⭐ | Symbol search with semantic matching |
| `loct --for-ai` | ⭐⭐⭐⭐ | AI bundle with quick wins and hub files |
| `loct query who-imports <file>` | ⭐⭐⭐⭐ | Reverse dependency lookup |
| `loct health` | ⭐⭐⭐⭐ | Quick health summary (cycles + dead + twins) |
| `loct coverage` | ⭐⭐⭐⭐ | Test coverage gaps for handlers/events |
| `loct dead` | ⭐⭐⭐ | Unused exports (JS/TS best, Python limited) |

### Known Limitations

- **Python import resolution** is ~50% accurate for complex package structures
- **FastAPI/Flask routes** not yet detected as handlers
- **Python slice/impact** may return empty results for unresolved imports

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

### Handler Trace (`trace`)

Follow a Tauri command through the entire pipeline:

```bash
loct trace get_user
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
loct --for-ai
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

### Multi-Language Support

loctree supports comprehensive analysis across multiple languages:

| Language | Status | Key Features |
|----------|--------|--------------|
| **Rust** | Exceptional | 0% false positives on rust-lang/rust (35,387 files, ~787 files/sec) |
| **Go** | Perfect | ~0% false positives on golang/go (17,182 files) |
| **TypeScript/JavaScript** | Full | JSX/TSX support, React patterns, Flow annotations, WeakMap/WeakSet patterns |
| **Python** | Full | Library mode with stdlib auto-detection, `__all__` tracking |
| **Svelte** | Full | .d.ts re-export tracking, component analysis |
| **Vue** | Full | Component analysis, SFC support |
| **Dart/Flutter** | Full | Complete language support (new in v0.6.x) |

### Auto-Detect Stack

loctree automatically detects your project type:

| Marker | Stack | Auto-Ignores | Extensions |
|--------|-------|--------------|------------|
| `Cargo.toml` | Rust | `target/` | `.rs` |
| `tsconfig.json` | TypeScript | `node_modules/` | `.ts`, `.tsx`, `.jsx` |
| `pyproject.toml` | Python | `.venv/`, `__pycache__/` | `.py` |
| `src-tauri/` | Tauri | All above | `.ts`, `.tsx`, `.rs` |
| `vite.config.*` | Vite | `dist/` | Auto |
| `pubspec.yaml` | Dart/Flutter | `.dart_tool/`, `build/` | `.dart` |

### Tauri Command Coverage

For Tauri projects, loctree validates the entire command pipeline:

```bash
loct commands
```

- **Missing handlers** — Frontend invokes commands that don't exist in backend
- **Unregistered handlers** — Backend has `#[tauri::command]` but not in `generate_handler![]`
- **Unused handlers** — Registered handlers never invoked from frontend
- **React lazy detection** — Tracks `React.lazy()` dynamic imports

### Test Coverage Analysis

Structural test coverage based on import analysis (no runtime instrumentation):

```bash
loct coverage                        # All coverage gaps
loct coverage --handlers             # Only Tauri handler gaps
loct coverage --min-severity high    # Filter by severity
loct coverage --json                 # JSON output for CI
```

Identifies coverage gaps by severity:
- **CRITICAL** — Handlers called in production but not tested
- **HIGH** — Events emitted but not tested
- **MEDIUM** — Exports used in production without test imports
- **LOW** — Tested but unused code (cleanup candidates)

### Tree Mode

Fast directory tree with LOC counts:

```bash
loct tree src/

# Find build artifacts to clean
loct tree --find-artifacts
# Output: /path/to/node_modules, /path/to/target, ...

# Show gitignored files
loct tree --show-ignored
```

### Janitor Mode Tools

Find problems before they become tech debt:

```bash
# Check if similar component exists before creating
loct find ChatSurface
# Found: ChatPanel (distance: 2), ChatWindow (distance: 3)

# Find potentially unused exports (improved detection in v0.6.x)
loct dead --confidence high

# Detect circular import cycles (with visualization in reports)
loct cycles

# Analyze impact of changing a file
loct impact src/utils/api.ts

# Find a symbol across the codebase
loct find useAuth

# Twins analysis (dead parrots, exact twins, barrel chaos)
loct twins
loct twins --dead-only    # Only exports with 0 imports
```

**Enhanced Dead Code Detection (v0.6.x):**
- **Registry Pattern Support** - Detects WeakMap/WeakSet usage (React DevTools, observability tools)
- **Flow Type Annotations** - Understands Flow syntax alongside TypeScript
- **Re-export Chains** - Tracks .d.ts files and barrel exports (Svelte, library types)
- **Python `__all__`** - Respects public API declarations in Python modules
- **Library Mode Intelligence** - Auto-detects npm packages and Python stdlib to exclude public APIs

```bash
# Example: Python stdlib analysis
loct dead --library-mode
# Skips: __all__ exports, Lib/ directory public APIs

# Example: npm package analysis
loct dead
# Auto-detects: package.json "exports" field, excludes public API

## Library / Framework Mode

loctree intelligently handles libraries and frameworks to avoid false positives:

**Automatic Detection:**
- **npm packages** with `exports` field in package.json
- **Python stdlib** detection via `Lib/` directory and `__all__` exports
- **Public API exclusion** - Exports in public APIs are not flagged as dead code

**Manual Activation:**
```bash
loct --library-mode
# or in .loctree/config.toml:
library_mode = true
```

**Features:**
- Ignores example sandboxes, demos, playgrounds, kitchen-sink, docs/examples
- Tracks `__all__` for Python public API boundaries
- Customizable via `library_example_globs` in config
- Outputs `.loctree/report.html` and `.loctree/analysis.json` automatically

**Advanced Pattern Detection:**
- WeakMap/WeakSet registry patterns (e.g., React DevTools)
- Flow type annotations
- TypeScript .d.ts re-export chains

# List entry points
loct lint --entrypoints
```

### HTML Reports

Generate interactive HTML reports with dependency graphs:

```bash
loct report --graph --output report.html
```

Features:
- Interactive Cytoscape.js dependency graphs
- Tabbed navigation (Duplicates, Cascades, Commands, Graph, Cycles)
- AI Summary panel with quick wins
- Circular dependency visualization
- Dark mode support

**Smoke Test Results:**
- **rust-lang/rust**: 35,387 files analyzed in ~45s (787 files/sec), 91 circular dependencies detected
- **facebook/react**: 3,951 files in 49s (81 files/sec), 8 circular dependencies found
- **golang/go**: 17,182 files analyzed with ~0% false positives

### CI Pipeline Checks

```bash
# Fail if frontend invokes missing backend handlers
loct lint --fail

# Fail if events lack listeners/emitters or have races
loct events --json

# SARIF 2.1.0 output for GitHub/GitLab
loct lint --sarif > results.sarif
```

## CLI Reference

```
loct (Rust) - AI-oriented Project Analyzer

Modes:
  (default)                 Scan, save snapshot + reports to .loctree/
  slice <file>              Holographic slice (add --consumers, --json)
  find                      Unified search (symbols, similar, impact)
  dead                      Unused exports (alias/barrel aware)
  cycles                    Circular imports
  twins                     Semantic duplicates (dead parrots, exact twins, barrel chaos)
  commands                  Tauri FE↔BE bridges (missing/unused)
  events                    Emit/listen/races summary
  tree                      Directory tree with LOC counts
  report --graph            HTML report with graph
  lint --fail --sarif       CI guardrails / SARIF output
  diff --since <id>         Compare snapshots, show delta
  query <kind> <target>     Quick queries (who-imports, where-symbol, component-of)
  '<filter>'                jq-style query on snapshot (e.g., '.metadata', '.files | length')
  --for-ai                  AI-optimized hierarchical JSON (legacy flag)

jq Query options:
  -r, --raw                 Raw output (no JSON quotes for strings)
  -c, --compact             Compact output (one line per result)
  -e, --exit-status         Exit 1 if result is false/null
  --arg <name> <value>      Bind string variable
  --argjson <name> <json>   Bind JSON variable
  --snapshot <path>         Use specific snapshot file

Find options:
  --similar <query>         Similar components/symbols
  --symbol <name>           Symbol definitions/usages
  --impact <file>           What imports this file

Slice options:
  --consumers               Include files that import the target
  --json                    JSON output for piping to AI

Query kinds:
  who-imports <file>        Files that import target
  where-symbol <name>       Where symbol is defined/used
  component-of <file>       Which graph component contains file

Tree options:
  --find-artifacts          Find build dirs (node_modules, target, etc.)
  --show-ignored            Show only gitignored files
  --summary[=N]             Show totals + top N large files

Common:
  -g, --gitignore           Respect .gitignore
  -I, --ignore <path>       Ignore path (repeatable)
  --full-scan               Re-analyze all (ignore cache)
  --verbose                 Detailed progress
```

## Installation

### macOS (Homebrew) - pending review

```bash
brew install loctree
```

### From crates.io (Recommended)

```bash
cargo install loctree
```

### From source

```bash
git clone https://github.com/Loctree/Loctree
cd loctree/loctree_rs
cargo install --path .
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
# Setup git hooks (auto-runs on `make install`)
make git-hooks

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
[![loctree](https://raw.githubusercontent.com/Loctree/Loctree/main/assets/loctree-badge.svg)](https://crates.io/crates/loctree)
```

[![loctree](https://raw.githubusercontent.com/Loctree/Loctree/main/assets/loctree-badge.svg)](https://crates.io/crates/loctree)

## License

Dual-licensed under MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

---

**Developed with 💀 by The Loctree Team (c)2025**
