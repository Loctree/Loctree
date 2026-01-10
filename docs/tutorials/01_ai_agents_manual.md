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
| Finding existing code | Grep/search, hope for the best | `loct find Button` |
| Understanding dependencies | Read imports manually | `loct slice src/Component.tsx --consumers` (deps included) |
| Knowing what uses a file | Search for import statements | `loct slice src/utils.ts --consumers` |
| Dead code detection | Guesswork | `loct dead --confidence high` or `loct doctor` |
| Circular import detection | Runtime errors | `loct cycles` or `loct doctor` |
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
├── findings.json      # All issues: dead_parrots, cycles, twins, shadows, orphans
├── manifest.json      # Index for tooling integration
├── agent.json         # AI-ready bundle (replaces --for-ai file)
├── report.html        # Human-readable HTML report
└── report.sarif       # SARIF 2.1.0 for IDE/CI integration
```

**Creating a snapshot:**
```bash
loct              # Auto-detect stack, write snapshot.json (fast)
loct --full-scan  # Force rescan (ignore cached mtime)
```

> **Note (0.7.0+):** The `-A` flag is deprecated. Use `loct doctor` for interactive diagnostics instead.

### Slicing

Slicing extracts relevant context for a specific file or task:

```bash
# Get file's dependencies (what it imports)
loct slice src/api/client.ts

# Get file's consumers (what imports it)
loct slice src/utils/format.ts --consumers

# Full context (deps + consumers + file analysis)
loct slice src/Component.tsx --consumers --json
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

### Short Aliases (v0.7.0+)

Save keystrokes with these built-in aliases:

| Alias | Command | Description |
|-------|---------|-------------|
| `s` | `slice` | File context extraction |
| `f` | `find` | Symbol/file search |
| `d` | `dead` | Dead export detection |
| `t` | `twins` | Semantic duplicate analysis |
| `h` | `health` | Quick health check |
| `i` | `impact` | Change impact analysis |
| `c` | `cycles` | Circular import detection |
| `q` | `query` | Quick graph queries |

**Examples:**
```bash
loct h                          # Quick health check
loct d --confidence high        # Dead exports
loct s src/App.tsx --consumers  # Slice with consumers
loct f Button                   # Find Button symbol
loct t --dead-only              # Dead parrots only
```

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
  --consumers   Include consumers (files that import this) – deps included by default
  --json        Output as JSON (for piping to AI)
  --depth N     Limit dependency depth (default: 3)
```

**Examples:**
```bash
# Context for AI task
loct slice src/ChatPanel.tsx --consumers --json | claude

# Just the imports (deps are default)
loct slice src/utils.ts

# What depends on this?
loct slice src/api/types.ts --consumers
```

### `loct find`

Search for code patterns and relationships.

```bash
loct find <name>     # Find symbol definitions and uses
loct find <name>     # Fuzzy find similar names (avoid duplicates)
loct impact <file>     # Show blast radius of changes
```

**Examples:**
```bash
# Before creating Button, check if it exists
loct find Button

# Find all uses of useAuth hook
loct find useAuth

# What breaks if I change api.ts?
loct impact src/utils/api.ts
```

### `loct dead`

> **Note (0.7.0+):** This command is deprecated. Use `loct doctor` for interactive diagnostics or jq queries (`loct '.dead_parrots'`) for scripting.

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

> **Note (0.7.0+):** This command is deprecated. Use `loct doctor` for interactive diagnostics or jq queries (`loct '.cycles'`) for scripting.

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

### `loct health`

Quick health check summary — combines cycles + dead exports + twins in one command.

```bash
loct health              # Quick summary
loct health --json       # JSON output for CI
loct health src/         # Analyze specific directory
```

**Output example:**
```
Health Check Summary

Cycles:      3 total (2 hard, 1 structural)
Dead:        6 high confidence, 24 low
Twins:       2 duplicate symbol groups

Run `loct cycles`, `loct dead`, `loct twins` for details.
```

Use this for quick sanity checks before commits or in CI pipelines.

### `loct audit`

> **Note (0.7.0+):** This command is deprecated. Use `loct doctor` for interactive diagnostics with actionable recommendations.

Full codebase audit — combines ALL structural analyses into one actionable report. Perfect for getting a complete picture of codebase health on day one.

```bash
loct audit              # Full audit of current directory
loct audit --json       # JSON output for CI
loct audit src/         # Audit specific directory
```

**What it includes:**
- **Cycles** — Circular imports (hard + structural)
- **Dead exports** — Unused code with 0 imports
- **Twins** — Same symbol exported from multiple files
- **Orphan files** — Files with 0 importers (not entry points)
- **Shadow exports** — Consolidation candidates
- **Crowds** — Files with similar dependency patterns

**Output example:**
```
🔍 Full Codebase Audit

CYCLES (3 total)
  2 hard cycles (breaking)
  1 structural cycle

DEAD EXPORTS (12 total)
  6 high confidence
  6 low confidence

TWINS (2 groups)
  useAuth exported from 2 files
  formatDate exported from 3 files

ORPHAN FILES (4 files, 1,200 LOC)
  src/legacy/old-utils.ts (450 LOC)
  ...

SHADOW EXPORTS (1)
  store exported by 2 files, 1 dead

CROWDS (2 clusters)
  API handlers: 5 similar files
  Form components: 3 similar files

─────────────────────────────────
Total: 22 findings to review
```

Use `loct audit` when onboarding to a new codebase or for comprehensive CI checks. Use `loct health` for quick daily checks.

### `loct doctor` (v0.7.0+)

Interactive diagnostics with actionable recommendations. This is the successor to `loct audit` with intelligent categorization and auto-fix suggestions.

```bash
loct doctor                     # Full diagnostics
loct doctor --apply-suppressions  # Auto-add patterns to .loctignore
```

**Categories findings into:**
- **Auto-fixable**: High confidence, safe to remove
- **Needs review**: Low confidence, verify manually
- **Suggested suppressions**: Patterns for `.loctignore`

**Example output:**
```
=== Doctor Diagnostics ===

Found 65 issues: 60 auto-fixable, 5 need review

Dead Exports (12 total):
  10 high confidence (safe to remove)
  2 low confidence (needs review)

Cycles (3 total):
  2 hard cycles (breaking)
  1 structural cycle

Twins (8 groups):
  Button exported from 2 files
  formatDate exported from 3 files

Suggested .loctignore entries:
  **/index.*
  **/*test*

Next steps:
  1. Review high-confidence dead exports and remove if safe
  2. Run tests after each removal
  3. Break hard cycles (structural cycles are often harmless)
```

**Workflow:**
```bash
# 1. Run diagnostics
loct doctor

# 2. Apply suppressions for known false positives
loct doctor --apply-suppressions

# 3. Fix high-confidence issues one by one
# 4. Verify with tests after each fix
# 5. Re-run doctor to track progress
```

### `loct twins`

> **Note (0.7.0+):** This command is deprecated. Use `loct doctor` for interactive diagnostics or jq queries (`loct '.twins'`) for scripting.

Semantic duplicate analysis — finds dead parrots, exact twins, and barrel chaos.

```bash
loct twins           # Full analysis: dead parrots + exact twins + barrel chaos
loct twins --dead-only    # Only exports with 0 imports
loct twins --path src/    # Analyze specific path
```

**What it detects:**

1. **Dead Parrots** — exports with zero imports (Monty Python reference: code that's "just resting")
   ```
   DEAD PARROTS (75 symbols with 0 imports)
     ├─ ChatPanelTabs (reexport:6) - 0 imports
     ├─ update_profile (reexport:0) - 0 imports
     └─ ...
   ```

2. **Exact Twins** — same symbol name exported from multiple files
   ```
   EXACT TWINS (150 duplicates)
     ├─ "Button" exported from:
     │   src/components/Button.tsx
     │   src/ui/Button.tsx
     └─ ...
   ```

3. **Barrel Chaos** — barrel file issues
   - Missing `index.ts` in directories with many external imports
   - Deep re-export chains (A → B → C → D)
   - Inconsistent import paths (same symbol imported via different paths)

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

### jq-style queries

Query snapshot data directly using jq syntax (powered by jaq, a Rust-native jq implementation).

```bash
loct '<filter>'                     # Query current snapshot
loct '<filter>' --snapshot <path>   # Query specific snapshot
```

**Flags:**
- `-r` — Raw output (no JSON quotes)
- `-c` — Compact output (single line)
- `-e` — Exit code based on empty result
- `--arg NAME VALUE` — Pass string variable
- `--argjson NAME JSON` — Pass JSON variable

**Examples:**
```bash
# Metadata
loct '.metadata'

# Count files
loct '.files | length'

# Find large files (>500 LOC)
loct '.files[] | select(.loc > 500)' -c

# Filter edges by pattern
loct '.edges[] | select(.from | contains("api"))'

# List Tauri commands
loct '.command_bridges | map(.name)'

# Find top export duplicates
loct '.export_index | to_entries | map(select(.value | length > 1)) | sort_by(.value | length) | reverse | .[0:5]'

# Query findings (0.7.0+)
loct '.dead_parrots'              # Dead exports from findings
loct '.cycles'                    # Circular imports
loct '.twins[:5]'                 # First 5 twin groups
loct '.orphans'                   # Orphan files
loct '.shadows'                   # Shadow exports
```

**Use cases:**
- Quick codebase statistics
- Custom filtering and aggregation
- Integration with shell pipelines
- Extracting specific data for analysis

**Output includes:**
- Files added, removed, modified
- New/resolved circular imports
- New/removed dead exports
- Changed graph edges

### `loct dist`

Bundle distribution analysis — verify tree-shaking by comparing source exports against production bundles using source maps.

```bash
loct dist dist/bundle.js.map src/   # Analyze bundle vs source
```

**Output:**
```
✓ Found 4 dead export(s) (67%)
Bundle Analysis:
  Source exports:  6
  Bundled exports: 2
  Dead exports:    4
  Reduction:       67%
  Analysis level:  symbol

Dead Exports (not in bundle):
  deadFunction (function) in index.ts:5
  DEAD_CONST (var) in index.ts:10
```

**Features:**
- Symbol-level detection via VLQ Base64 decoding of source map mappings
- File-level fallback when source maps lack `names` array
- Verifies bundler (Vite/Webpack/esbuild) actually eliminated dead code

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

### jq-style Queries (v0.6.15+)

Query snapshot data directly using jq syntax. Uses jaq (Rust-native jq implementation) for zero external dependencies.

```bash
loct '<filter>' [options]
```

**Basic Usage:**
```bash
loct '.metadata'                    # Extract snapshot metadata
loct '.files | length'              # Count files in codebase
loct '.edges | length'              # Count import edges
loct '.command_bridges | length'    # Count Tauri commands
```

**Filtering:**
```bash
# Find all edges from api/ directory
loct '.edges[] | select(.from | contains("api"))'

# Find large files (>500 LOC)
loct '.files[] | select(.loc > 500)'

# Get all file paths
loct '.files[].path' -r

# List Tauri command names
loct '.command_bridges | map(.name)'
```

**Options:**
| Flag | Description |
|------|-------------|
| `-r`, `--raw` | Raw output (no JSON quotes for strings) |
| `-c`, `--compact` | Compact output (one line per result) |
| `-e`, `--exit-status` | Exit 1 if result is false/null |
| `--arg <name> <val>` | Bind string variable |
| `--argjson <name> <json>` | Bind JSON variable |
| `--snapshot <path>` | Use specific snapshot file |

**Variable Binding:**
```bash
# Find edges from specific file
loct '.edges[] | select(.from == $file)' --arg file 'src/api.ts'

# Files with LOC above threshold
loct '.files[] | select(.loc > $min)' --argjson min 300
```

**Important:** Filter must come before flags:
```bash
# ✅ Correct
loct '.edges[]' --arg file 'foo.ts'

# ❌ Won't work
loct --arg file 'foo.ts' '.edges[]'
```

**Snapshot Discovery:**
- Auto-discovers newest `.loctree/*/snapshot.json` by modification time
- Use `--snapshot path/to/snapshot.json` to specify explicitly

---

## Agent Bundle for CI

The agent bundle is a complete analysis package for CI pipelines:

```
.loctree/
├── snapshot.json      # Code graph (files, imports, exports)
├── findings.json      # All issues: dead_parrots, cycles, twins, shadows, orphans
├── manifest.json      # Index for tooling integration
├── agent.json         # AI-ready bundle (replaces --for-ai file)
├── report.sarif       # SARIF 2.1.0 for GitHub/GitLab
├── report.html        # Human review
└── py_races.json      # Python concurrency (if applicable)
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

**Using findings.json in CI:**
```yaml
- name: Check for issues
  run: |
    loct
    dead_count=$(loct '.dead_parrots | length')
    cycle_count=$(loct '.cycles | length')
    echo "Dead exports: $dead_count"
    echo "Cycles: $cycle_count"
    if [ "$dead_count" -gt 0 ] || [ "$cycle_count" -gt 0 ]; then
      exit 1
    fi
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
loct find FeatureName    # Check for existing code
loct slice src/related.ts --consumers --json

# 3. Understand impact
loct impact src/file-to-modify.ts
```

### Before Creating a New Component

```bash
# Check if similar exists
loct find Button
loct find ButtonComponent
loct find useButton

# If creating, check where to place it
loct slice src/components/index.ts --consumers
```

### Debugging Import Issues

```bash
# Find circular imports
loct cycles

# Check what imports what
loct slice problematic-file.ts --consumers
```

### Cleaning Up Dead Code

```bash
# Use doctor for interactive diagnostics (0.7.0+)
loct doctor

# Or find candidates manually
loct dead --confidence high --json

# Verify each before deletion
loct find suspectedDead
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
- **Flow types** - Flow annotation support (v0.6.x)
- **WeakMap/WeakSet patterns** - Registry pattern detection (v0.6.x)
- **.d.ts re-exports** - Proper type-only re-export tracking (v0.6.x)

### SvelteKit

- **Virtual modules** - Resolves `$app/navigation`, `$app/stores`, `$app/environment`, `$app/paths`
- **`$lib` alias** - Maps `$lib/*` to configured library path
- **Runtime modules** - Correctly resolves SvelteKit internal runtime paths
- **Server/client split** - Understands `.server.ts` and `+page.server.ts` patterns
- **.d.ts re-exports** - Tracks Svelte component type exports (v0.6.x)

### Python

- **Namespace packages** - PEP 420 support (no `__init__.py` required)
- **Typed packages** - PEP 561 `py.typed` marker detection
- **Test detection** - Distinguishes test files from production
- **Concurrency patterns** - Detects threading/asyncio/multiprocessing
- **`__all__` tracking** - Respects public API declarations (v0.6.x)
- **Library mode** - Auto-detects Python stdlib (Lib/ directory) (v0.6.x)

### Rust

- **Crate structure** - Understands `mod` declarations and module hierarchy
- **Crate-internal imports** - Resolves `use crate::foo::Bar`, `use super::Bar`, `use self::foo::Bar`
- **Same-file usage** - Detects when exported symbols are used locally (e.g., `BUFFER_SIZE` in generics like `fn foo::<BUFFER_SIZE>()`)
- **Nested brace imports** - Handles complex imports like `use crate::{foo::{A, B}, bar::C}`
- **Tauri integration** - `#[tauri::command]` detection
- **Symbol-level blame** - Git blame for fn/struct/enum/impl

### Go

- **Package structure** - Understands Go package imports
- **Cross-package references** - Accurate dead code detection across packages
- **Standard library** - Stdlib imports tracked correctly

### Dart/Flutter (v0.6.x)

- **Package imports** - Resolves `package:` imports
- **Auto-detection** - Recognizes `pubspec.yaml`, ignores `.dart_tool/`, `build/`
- **Full language support** - Imports, exports, dead code detection

### Vue

- **Single File Components (SFC)** - `<script setup>` and `<script>` support
- **Component analysis** - Tracks component usage and exports

---

## Integration Patterns

### Piping to Claude/AI

```bash
# Full context for a task
loct slice src/ChatPanel.tsx --consumers --json | claude "refactor this"

# Analysis summary (0.7.0+)
loct '.dead_parrots' | claude "what should I clean up?"
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
# Use doctor for intelligent categorization (0.7.0+)
loct doctor

# Or use high confidence only
loct dead --confidence high

# Check if it's used dynamically
loct find suspectedDead
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
| File context | `loct slice <file> --consumers --json` (deps included by default) |
| Find similar | `loct find <name>` or `loct f <name>` |
| Find symbol | `loct find <name>` |
| Impact analysis | `loct impact <file>` or `loct i <file>` |
| Dead code | `loct dead --confidence high` or `loct d` |
| Circular imports | `loct cycles` or `loct c` |
| Twins analysis | `loct twins` or `loct t` |
| Health check | `loct health` or `loct h` |
| Doctor diagnostics | `loct doctor` (0.7.0+) |
| FE↔BE gaps | `loct commands --missing` |
| Who imports file | `loct query who-imports <file>` or `loct q who-imports <file>` |
| Where is symbol | `loct query where-symbol <name>` |
| jq query (snapshot) | `loct '.files \| length'`, `loct '.metadata'` |
| jq query (findings) | `loct '.dead_parrots'`, `loct '.cycles'`, `loct '.twins'` (0.7.0+) |
| Delta since snapshot | `loct diff --since <id>` |
| CI lint | `loct lint --fail --sarif` |
| Git blame | `loct git blame <file>` |

Keep artifacts together: snapshots and agent metadata (e.g., `AI_META.json` or `for-ai.json`) should live under `.loctree/<branch@sha>/` alongside `snapshot.json` so diffs and queries stay aligned with the scan.

---

Developed with care by The Loctree Team (c)2025.
