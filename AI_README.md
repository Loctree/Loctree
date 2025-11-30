# loctree - AI Agent Quick Reference (v0.5.4)

A static analysis tool designed for AI agents working on production codebases. Solves the "context drift" problem where AI generates duplicates, barrel files, and circular dependencies instead of reusing existing components.

## Core Principle: Scan Once, Slice Many

```bash
# 1. Scan project (creates .loctree/snapshot.json)
loctree

# 2. Extract focused context for any task
loctree slice src/components/ChatPanel.tsx --consumers --json | claude
```

## Installation

```bash
cargo install --force --path loctree_rs
loctree --version   # expected: 0.5.4
```

## Primary Workflow for AI Agents

### Step 1: Understand What Exists

Before generating any new component, always check:

```bash
# Find similar existing components
loctree -A --check ChatSurface
# Output: Found: ChatPanel (distance: 2), ChatWindow (distance: 3)

# Search for symbol usage
loctree -A --symbol useAuth
```

### Step 2: Extract Relevant Context

Use Holographic Slice to get 3-layer context (Core/Deps/Consumers):

```bash
# Human-readable slice
loctree slice src/features/chat/ChatPanel.tsx --consumers

# JSON for piping to AI
loctree slice src/features/chat/ChatPanel.tsx --consumers --json | claude "refactor to React Query"
```

Output structure:
```
Slice for: src/features/chat/ChatPanel.tsx

Core (1 files, 150 LOC):
  src/features/chat/ChatPanel.tsx (150 LOC, tsx)

Deps (3 files, 420 LOC):
  [d1] src/hooks/useAuth.ts (80 LOC, ts)
    [d2] src/contexts/AuthContext.tsx (200 LOC, tsx)
    [d2] src/utils/api.ts (140 LOC, ts)

Consumers (2 files, 180 LOC):
  src/pages/Chat.tsx (100 LOC, tsx)
  src/App.tsx (80 LOC, tsx)

Total: 6 files, 750 LOC
```

### Step 3: Detect Problems

```bash
# Find circular imports (causes runtime issues)
loctree -A --circular

# Find dead exports (cleanup candidates)
loctree -A --dead --confidence high

# Analyze impact of changes
loctree -A --impact src/utils/api.ts
```

## Modes

| Mode | Command | Purpose |
|------|---------|---------|
| Init (default) | `loctree` | Scan and save snapshot to .loctree/snapshot.json |
| Slice | `loctree slice <file>` | Extract 3-layer context for AI |
| Analyzer | `loctree -A` | Import/export analysis, duplicates, coverage |

## Auto-Detect Stack

loctree automatically detects project type and configures ignores:

| Marker | Detected As | Auto-Ignores |
|--------|-------------|--------------|
| Cargo.toml | Rust | target/ |
| tsconfig.json | TypeScript | node_modules/, dist/ |
| pyproject.toml | Python | .venv/, __pycache__/ |
| src-tauri/ | Tauri | All of the above |
| vite.config.* | Vite | dist/, build/ |

## Key Flags

### Slice Mode
- `--consumers` - Include files that import the target
- `--json` - Output as JSON for piping to AI

### Analyzer Mode (-A)
- `--check <query>` - Find similar components (avoid duplicates)
- `--dead` - Find unused exports
- `--symbol <name>` - Find symbol definitions and usage
- `--impact <file>` - What imports this file (what breaks if removed)
- `--circular` - Detect circular import cycles
- `--entrypoints` - List entry points (main functions)
- `--sarif` - SARIF 2.1.0 output for CI integration

### CI Pipeline Checks
- `--fail-on-missing-handlers` - Exit 1 if FE calls missing BE handlers
- `--fail-on-ghost-events` - Exit 1 if events emitted but never listened
- `--fail-on-races` - Exit 1 if potential race conditions detected

### Common
- `-g, --gitignore` - Respect .gitignore
- `--full-scan` - Ignore mtime cache, re-analyze all files
- `--verbose` - Show detailed progress
- `--json` - JSON output

## Incremental Scanning

After first scan, loctree uses file modification times to skip unchanged files:

```
$ loctree
[loctree][detect] Detected: Tauri + Vite
[loctree][progress] 32 cached, 1 fresh (touched: src/App.tsx)
```

Use `--full-scan` to force complete re-analysis.

## JSON Output (Schema 1.3.0)

Key fields for AI agents:
- `files[*].imports` - Resolution kind (local|stdlib|dynamic|unknown)
- `aiViews.commands2` - FE to BE command coverage (ok|missing_handler|unused_handler)
- `symbols/clusters` - Duplicate groups with canonical, score, reasons
- `graphs` - Dependency graph when `--graph` enabled

## Example Workflows

### Before Creating New Component
```bash
# Check if similar exists
loctree -A --check UserProfile
# If found similar: read and extend existing
# If not found: proceed with new component
```

### Refactoring a Module
```bash
# Get full context
loctree slice src/services/auth.ts --consumers --json > context.json

# Understand impact
loctree -A --impact src/services/auth.ts

# After changes, verify no circular imports
loctree -A --circular
```

### CI Integration
```bash
# In CI pipeline
loctree -A --fail-on-missing-handlers --fail-on-ghost-events
loctree -A --sarif > results.sarif  # Upload to GitHub/GitLab
```

### Tauri Project (FE + BE)
```bash
loctree -A --preset-tauri src src-tauri/src \
  --fail-on-missing-handlers \
  --json > analysis.json
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Root is not a directory" | Use paths relative to cwd or absolute paths |
| No graph output | Check limits (--max-graph-nodes/edges), narrow scope |
| Tauri coverage noise | Commands via wrappers (safeInvoke) have aliased impl in commands2 |
| Stale results | Use --full-scan to bypass mtime cache |

## Philosophy

The goal is not "make it work". The goal is: we know WHY it works (or doesn't).

- Import graphs show real dependencies, not assumed ones
- Dead code detection finds what you forgot you wrote
- Circular import detection catches runtime bombs before they explode
- Context slicing gives AI agents exactly what they need, no more

---

Created by M&K (c)2025 The LibraxisAI Team
