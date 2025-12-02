# AI Agent's Manual for Loctree

> Complete guide for AI agents working with codebases using loctree.
> For quick reference, see [AI_README.md](../../AI_README.md).

## Table of Contents

1. [Why Loctree?](#why-loctree)
2. [Installation](#installation)
3. [Core Concepts](#core-concepts)
4. [Command Reference](#command-reference)
5. [Agent Bundle for CI](#agent-bundle-for-ci)
6. [Workflows](#workflows)
7. [Language-Specific Features](#language-specific-features)
8. [Integration Patterns](#integration-patterns)
9. [Troubleshooting](#troubleshooting)

---

## Why Loctree?

Loctree is a static analysis tool designed for AI agents. It solves the fundamental problem of **context** - knowing what code exists, how it connects, and what's actually used.

### Problems Loctree Solves

| Problem | Without Loctree | With Loctree |
|---------|-----------------|--------------|
| Finding existing code | Grep/search, hope for the best | `loct find --similar Button` |
| Understanding dependencies | Read imports manually | `loct slice src/Component.tsx --deps` |
| Knowing what uses a file | Search for import statements | `loct slice src/utils.ts --consumers` |
| Dead code detection | Guesswork | `loct dead --confidence high` |
| Circular import detection | Runtime errors | `loct cycles` |
| Tauri FE↔BE coverage | Manual audit | `loct commands --missing` |

### Key Philosophy

1. **Scan once, slice many** - Initial scan builds a snapshot; subsequent queries are instant
2. **Know why it works** - Graph-based analysis over assumptions
3. **Reduce false positives** - Alias-aware, barrel-aware dead code detection
4. **Holographic slices** - Extract exactly the context needed for a task

---

## Installation

```bash
# Install from crates.io
cargo install loctree

# Verify installation
loct --version
```

### Requirements

- Rust toolchain (for installation)
- Git repository (optional, enables git-aware features)

---

## Core Concepts

### Snapshot

A snapshot is a cached representation of your codebase's structure:

```
.loctree/
├── snapshot.json      # Code graph (files, imports, exports)
├── analysis.json      # Analysis results (dead code, gaps, cycles)
├── report.html        # Human-readable HTML report
├── report.sarif       # SARIF 2.1.0 for IDE/CI integration
├── circular.json      # Circular import details
└── py_races.json      # Python concurrency patterns
```

**Creating a snapshot:**
```bash
loct              # Auto-detect stack, create snapshot + all artifacts
loct --full-scan  # Force rescan (ignore cached mtime)
```

### Slicing

Slicing extracts relevant context for a specific file or task:

```bash
# Get file's dependencies (what it imports)
loct slice src/api/client.ts --deps

# Get file's consumers (what imports it)
loct slice src/utils/format.ts --consumers

# Full context (deps + consumers + file analysis)
loct slice src/Component.tsx --deps --consumers --json
```

### Graph

The import graph maps relationships between files:

```
src/App.tsx
  └─imports→ src/components/Header.tsx
  └─imports→ src/components/Footer.tsx
  └─imports→ src/hooks/useAuth.ts
       └─imports→ src/api/auth.ts
```

---

## Command Reference

### `loct` (default scan)

Scans the project and generates all artifacts.

```bash
loct                    # Scan from current directory
loct src src-tauri      # Scan specific roots
loct --full-scan        # Force rescan
loct --scan-all         # Include node_modules, .venv, target
```

**Output:** Creates `.loctree/` with snapshot and all analysis artifacts.

### `loct slice`

Extract context for a file.

```bash
loct slice <file> [options]

Options:
  --deps        Include dependencies (files this imports)
  --consumers   Include consumers (files that import this)
  --json        Output as JSON (for piping to AI)
  --depth N     Limit dependency depth (default: 3)
```

**Examples:**
```bash
# Context for AI task
loct slice src/ChatPanel.tsx --deps --consumers --json | claude

# Just the imports
loct slice src/utils.ts --deps

# What depends on this?
loct slice src/api/types.ts --consumers
```

### `loct find`

Search for code patterns and relationships.

```bash
loct find --symbol <name>     # Find symbol definitions and uses
loct find --similar <name>    # Find similar names (avoid duplicates)
loct find --impact <file>     # Show blast radius of changes
```

**Examples:**
```bash
# Before creating Button, check if it exists
loct find --similar Button

# Find all uses of useAuth hook
loct find --symbol useAuth

# What breaks if I change api.ts?
loct find --impact src/utils/api.ts
```

### `loct dead`

Detect unused exports (dead code).

```bash
loct dead                      # All dead exports
loct dead --confidence high    # Only high-confidence (no test files)
loct dead --json               # JSON output
```

**Confidence levels:**
- `high` - Export not imported anywhere in production code
- `medium` - Export only used in tests
- `low` - Complex re-export patterns, may be false positive

### `loct cycles`

Detect circular imports.

```bash
loct cycles          # List all cycles
loct cycles --json   # JSON output with full paths
```

**Output example:**
```
Circular import detected:
  src/a.ts → src/b.ts → src/c.ts → src/a.ts
```

### `loct commands`

Tauri FE↔BE command coverage.

```bash
loct commands --missing   # FE invokes without BE handlers
loct commands --unused    # BE handlers without FE invokes
loct commands --json      # Full command mapping
```

### `loct events`

Tauri event coverage (emit/listen pairs).

```bash
loct events              # Summary
loct events --json       # Full event mapping
loct events --ghosts     # Emits without listeners
loct events --orphans    # Listeners without emitters
```

### `loct lint`

CI-friendly linting with policy enforcement.

```bash
loct lint --fail                    # Exit 1 if issues found
loct lint --sarif > results.sarif   # SARIF output for IDE
loct lint --max-dead 0              # Fail if any dead exports
loct lint --max-cycles 0            # Fail if any circular imports
```

### `loct git`

Git-aware analysis.

```bash
loct git compare HEAD~5..HEAD       # Semantic diff between commits
loct git blame src/lib.rs           # Symbol-level blame (Rust)
loct git history --symbol foo       # Track symbol evolution
```

### `loct diff`

Compare snapshots to see what changed.

```bash
loct diff --since <snapshot_id>     # Compare current vs old snapshot
loct diff --since main              # Compare against main branch snapshot
```

**Output includes:**
- Files added, removed, modified
- New/resolved circular imports
- New/removed dead exports
- Changed graph edges

### `loct query`

Quick graph queries without full analysis.

```bash
loct query who-imports <file>       # Files that import target
loct query where-symbol <name>      # Where symbol is defined/used
loct query component-of <file>      # Graph component containing file
```

**Examples:**
```bash
# What imports my utils?
loct query who-imports src/utils/helpers.ts

# Where is useAuth defined?
loct query where-symbol useAuth

# Is this file isolated or connected?
loct query component-of src/orphan.ts
```

---

## Agent Bundle for CI

The agent bundle is a complete analysis package for CI pipelines:

```
.loctree/
├── snapshot.json      # Code graph
├── analysis.json      # All findings
├── report.sarif       # SARIF 2.1.0 for GitHub/GitLab
├── report.html        # Human review
├── circular.json      # Cycle details
└── py_races.json      # Python concurrency
```

### CI Integration

**GitHub Actions:**
```yaml
- name: Run loctree analysis
  run: |
    cargo install loctree
    loct

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v2
  with:
    sarif_file: .loctree/report.sarif
```

**Policy enforcement:**
```yaml
- name: Check code quality
  run: |
    loct lint --max-dead 0 --max-cycles 0 --fail
```

### SARIF Contents

The `report.sarif` includes:
- `duplicate-export` - Same symbol exported from multiple files
- `missing-handler` - Frontend command without backend handler
- `unused-handler` - Backend handler without frontend usage
- `dead-export` - Export never imported
- `circular-import` - Circular dependency chain
- `ghost-event` - Event emitted but never listened
- `orphan-listener` - Listener for non-existent event

### IDE Integration URLs

SARIF results include `loctree://open?f=<file>&l=<line>` URLs in `properties.openUrl` for direct IDE navigation. Compatible with:
- VS Code (via URL handler extension)
- JetBrains IDEs (built-in URL handling)
- Custom editor integrations

---

## Workflows

### Starting a New Task

```bash
# 1. Scan the project (or use cached snapshot)
loct

# 2. Find relevant context
loct find --similar FeatureName    # Check for existing code
loct slice src/related.ts --consumers --deps --json

# 3. Understand impact
loct find --impact src/file-to-modify.ts
```

### Before Creating a New Component

```bash
# Check if similar exists
loct find --similar Button
loct find --similar ButtonComponent
loct find --symbol useButton

# If creating, check where to place it
loct slice src/components/index.ts --consumers
```

### Debugging Import Issues

```bash
# Find circular imports
loct cycles

# Check what imports what
loct slice problematic-file.ts --deps --consumers
```

### Cleaning Up Dead Code

```bash
# Find candidates
loct dead --confidence high --json

# Verify each before deletion
loct find --symbol suspectedDead
loct slice file-with-dead-code.ts --consumers
```

### Tauri Development

```bash
# Check FE↔BE contract
loct commands --missing   # What's called but not implemented?
loct commands --unused    # What's implemented but never called?

# Event flow
loct events --json        # Full emit/listen mapping
loct events --ghosts      # Emits going nowhere
```

---

## Language-Specific Features

### TypeScript/JavaScript

- **Path aliases** - Respects `tsconfig.json` `paths` and `baseUrl`
- **Barrel files** - Understands `index.ts` re-exports
- **Dynamic imports** - Tracks `import()` expressions
- **JSX/TSX** - Full support

### Python

- **Namespace packages** - PEP 420 support (no `__init__.py` required)
- **Typed packages** - PEP 561 `py.typed` marker detection
- **Test detection** - Distinguishes test files from production
- **Concurrency patterns** - Detects threading/asyncio/multiprocessing

### Rust

- **Crate structure** - Understands `mod` declarations
- **Tauri integration** - `#[tauri::command]` detection
- **Symbol-level blame** - Git blame for fn/struct/enum/impl

---

## Integration Patterns

### Piping to Claude/AI

```bash
# Full context for a task
loct slice src/ChatPanel.tsx --deps --consumers --json | claude "refactor this"

# Analysis summary
cat .loctree/analysis.json | claude "what should I clean up?"
```

### IDE Integration

```bash
# Generate SARIF for IDE warnings
loct lint --sarif > .loctree/report.sarif

# Most IDEs auto-detect SARIF files
# VS Code: SARIF Viewer extension
# JetBrains: built-in SARIF support
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

loct lint --max-cycles 0 --fail || {
    echo "Circular imports detected!"
    exit 1
}
```

---

## Troubleshooting

### "Snapshot not found"

```bash
# Create initial snapshot
loct

# Or force rescan
loct --full-scan
```

### "File not in snapshot"

The file might be in an ignored directory:

```bash
# Check what's being scanned
loct --scan-all  # Include everything

# Or add specific roots
loct src lib
```

### False Positives in Dead Code

```bash
# Use high confidence only
loct dead --confidence high

# Check if it's used dynamically
loct find --symbol suspectedDead
```

### Slow Initial Scan

```bash
# Exclude heavy directories
loct --ignore node_modules --ignore target

# Or use incremental scanning (default)
loct  # Uses mtime cache after first scan
```

### Circular Import False Positive

Some cycles are intentional (type-only imports). Check:

```bash
loct cycles --json | jq '.[] | select(.files | length > 2)'
```

---

## Quick Reference Card

| Goal | Command |
|------|---------|
| Scan project | `loct` |
| Force rescan | `loct --full-scan` |
| File context | `loct slice <file> --deps --consumers --json` |
| Find similar | `loct find --similar <name>` |
| Find symbol | `loct find --symbol <name>` |
| Impact analysis | `loct find --impact <file>` |
| Dead code | `loct dead --confidence high` |
| Circular imports | `loct cycles` |
| FE↔BE gaps | `loct commands --missing` |
| Who imports file | `loct query who-imports <file>` |
| Where is symbol | `loct query where-symbol <name>` |
| Delta since snapshot | `loct diff --since <id>` |
| CI lint | `loct lint --fail --sarif` |
| Git blame | `loct git blame <file>` |

---

Developed with care by The Loctree Team (c)2025.
