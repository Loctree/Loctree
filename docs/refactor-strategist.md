# Refactor Strategist

> **loct plan** - Generate architectural refactoring plans with risk-ordered execution phases

The Refactor Strategist analyzes module coupling and generates safe refactoring plans. It detects architectural layers, identifies misplaced files, and produces phased migration scripts ordered by risk level.

---

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Layer Detection](#layer-detection)
- [Risk Assessment](#risk-assessment)
- [Output Formats](#output-formats)
- [Shimming Strategy](#shimming-strategy)
- [Cyclic Dependency Handling](#cyclic-dependency-handling)
- [Configuration](#configuration)
- [Examples](#examples)
- [Integration with Other Commands](#integration-with-other-commands)

---

## Overview

Large codebases accumulate architectural debt over time. Files get placed in wrong directories, layers become blurred, and refactoring becomes risky due to complex dependencies.

**Refactor Strategist** solves this by:

1. **Detecting Layers** - Classifies files into UI, App, Kernel, Infra, or Test layers using path heuristics
2. **Analyzing Impact** - Uses dependency graph to calculate consumer counts and identify high-risk moves
3. **Detecting Cycles** - Identifies files involved in circular dependencies (via Tarjan's SCC)
4. **Ordering by Risk** - Sorts moves from LOW → MEDIUM → HIGH risk for safe incremental execution
5. **Generating Shims** - Creates re-export stubs for heavily-imported files to maintain backward compatibility

---

## Quick Start

```bash
# Generate markdown plan for current directory
loct plan

# Generate plan for specific directory
loct plan src/features

# Generate all formats (markdown, JSON, shell script)
loct plan --all -o refactor-2026

# Generate executable migration script
loct plan --script > migrate.sh
chmod +x migrate.sh
./migrate.sh --dry  # Preview changes
./migrate.sh        # Execute migration
```

---

## Layer Detection

Files are classified into architectural layers based on path patterns:

| Layer | Directory Patterns | File Patterns |
|-------|-------------------|---------------|
| **UI** | `components/`, `views/`, `pages/`, `ui/`, `widgets/`, `screens/` | `.tsx`, `.vue`, `.svelte` |
| **App** | `hooks/`, `services/`, `stores/`, `state/`, `context/`, `providers/` | `use*.ts` (React hooks) |
| **Kernel** | `core/`, `domain/`, `models/`, `entities/`, `business/` | - |
| **Infra** | `utils/`, `helpers/`, `lib/`, `adapters/`, `api/`, `clients/` | - |
| **Test** | `tests/`, `__tests__/`, `spec/` | `.test.*`, `.spec.*`, `*_test.*` |

### Detection Priority

1. Test patterns are checked first (highest priority)
2. UI patterns next (but excludes hooks/stores)
3. App patterns (hooks, services, stores)
4. Kernel patterns (core business logic)
5. Infra patterns (utilities, infrastructure)
6. If no match, `Unknown` layer is assigned

### Custom Layer Mapping

Override default layer detection with `--target-layout`:

```bash
loct plan --target-layout "core=src/kernel,ui=src/views,infra=src/shared"
```

---

## Risk Assessment

Each file move is assigned a risk level based on impact analysis:

### Risk Levels

| Risk | Icon | Thresholds | Description |
|------|------|------------|-------------|
| **LOW** | 🟢 | <5 consumers, <200 LOC, not in cycle | Safe to move with minimal impact |
| **MEDIUM** | 🟡 | 5-10 consumers, 200-500 LOC | Moderate impact, review recommended |
| **HIGH** | 🔴 | >10 consumers, >500 LOC, or in cycle | Significant impact, proceed carefully |

### Risk Calculation Formula

```
if in_cycle → HIGH
if direct_consumers >= 10 OR transitive_consumers >= 50 → HIGH
if loc >= 500 → HIGH
if direct_consumers >= 5 OR transitive_consumers >= 20 → MEDIUM
if loc >= 200 → MEDIUM
else → LOW
```

### Phased Execution

Moves are grouped into phases by risk level:

```
Phase 1: LOW Risk   (10 files)  ← Execute first
Phase 2: MEDIUM Risk (6 files)  ← Execute second
Phase 3: HIGH Risk  (10 files)  ← Execute last, with extra care
```

This ordering minimizes disruption: if Phase 1 breaks something, you haven't touched the critical files yet.

---

## Output Formats

### Markdown (Default)

Human-readable report with tables, git commands, and shim instructions:

```bash
loct plan                    # Print to stdout
loct plan -o refactor.md     # Write to file
```

**Sections:**
- Summary (file counts, risk breakdown)
- Layer distribution (before/after)
- Phased execution plan with tables
- Git commands for each phase
- Shimming strategy

### JSON

Machine-readable format for tooling integration:

```bash
loct plan --json            # Print to stdout
loct plan --json -o plan.json
```

**Structure:**
```json
{
  "target": "src/features",
  "moves": [...],
  "shims": [...],
  "cyclic_groups": [...],
  "phases": [...],
  "stats": {
    "total_files": 47,
    "files_to_move": 23,
    "shims_needed": 8,
    "layer_before": {"Unknown": 23, "UI": 10, ...},
    "layer_after": {"Infra": 15, "UI": 10, ...},
    "by_risk": {"LOW": 12, "MEDIUM": 8, "HIGH": 3}
  }
}
```

### Shell Script

Executable bash script with phase functions:

```bash
loct plan --script > migrate.sh
chmod +x migrate.sh

# Usage options:
./migrate.sh           # Execute all phases
./migrate.sh --dry     # Preview (no changes)
./migrate.sh 1         # Execute only Phase 1
./migrate.sh 2         # Execute only Phase 2
```

**Script Features:**
- `set -e` for fail-fast execution
- Color-coded output (green/yellow/red by risk)
- Dry-run mode (`--dry`)
- Phase selection (`./migrate.sh 1`)
- Automatic `mkdir -p` for target directories
- Git mv commands with proper quoting

### All Formats

Generate all three formats at once:

```bash
loct plan --all -o refactor-2026
# Creates:
#   refactor-2026.md
#   refactor-2026.json
#   refactor-2026.sh (executable)
```

---

## Shimming Strategy

When a file has many importers (>3), moving it directly would require updating all import statements. Instead, a **shim** can be created at the old location that re-exports from the new location.

### When Shims Are Suggested

```
direct_consumers > 3 → suggest shim
```

### Shim Examples

**TypeScript/JavaScript:**
```typescript
// Old location: src/utils/format.ts (shim)
export { formatDate, formatCurrency } from '../infra/format';
// or
export * from '../infra/format';
```

**Rust:**
```rust
// Old location: src/utils.rs (shim)
pub use crate::infra::utils::*;
```

**Python:**
```python
# Old location: src/utils.py (shim)
from .infra.utils import *
```

### Gradual Migration

1. Move file to new location
2. Create shim at old location
3. Update importers incrementally
4. Remove shim when no importers remain

---

## Cyclic Dependency Handling

Files involved in circular imports are flagged as **HIGH risk** and grouped together.

### Detection

Uses Tarjan's Strongly Connected Components (SCC) algorithm to detect cycles in the dependency graph.

### Output

```markdown
## ⚠️ Cyclic Dependencies

The following groups of files have circular imports. Move these together or break the cycle first:

**Cycle 1:**
- `src/a.ts`
- `src/b.ts`

**Cycle 2:**
- `src/models/patient.ts`
- `src/services/patientService.ts`
- `src/hooks/usePatient.ts`
```

### Recommendations

1. **Break the cycle first** - Extract shared types to a third module
2. **Move together** - If the cycle is intentional, move all files in the group together
3. **Review architecture** - Cycles often indicate design issues

---

## Configuration

### Command Options

| Option | Short | Description |
|--------|-------|-------------|
| `--target-layout <SPEC>` | | Custom layer mapping (e.g., `"core=src/kernel"`) |
| `--markdown` | `--md` | Output as markdown (default) |
| `--json` | | Output as JSON |
| `--script` | `--sh` | Output as executable shell script |
| `--all` | | Generate all formats (.md, .json, .sh) |
| `--output <PATH>` | `-o` | Output file path (without extension for --all) |
| `--no-open` | | Don't auto-open the generated report |
| `--include-tests` | | Include test files in analysis |
| `--min-coupling <N>` | | Minimum coupling score to include (0.0-1.0) |
| `--max-module-size <N>` | | Maximum module LOC before suggesting split |

### Examples

```bash
# Custom layer mapping
loct plan --target-layout "core=src/kernel,ui=src/components,infra=src/shared"

# Include test files (normally excluded)
loct plan --include-tests

# Generate all formats to specific directory
loct plan --all -o reports/refactor-$(date +%Y%m%d)

# Pipe JSON to other tools
loct plan --json | jq '.moves | length'  # Count moves
loct plan --json | jq '.stats.by_risk'   # Risk breakdown
```

---

## Examples

### Example 1: React Feature Module

**Directory:** `src/features/patients/`

**Before:**
```
src/features/patients/
├── PatientList.tsx      → UI (correct)
├── PatientCard.tsx      → UI (correct)
├── utils.ts             → Unknown (should be Infra)
├── types.ts             → Unknown (should be Kernel)
├── usePatients.ts       → App (correct, hook pattern)
└── api.ts               → Unknown (should be Infra)
```

**Command:**
```bash
loct plan src/features/patients
```

**Output:**
```markdown
# Refactor Plan: src/features/patients

## Summary
- Files analyzed: 6
- Files to move: 3
- Risk: 2 LOW, 1 MEDIUM

## 🟢 Phase 1: LOW Risk (2 files)
| File | From | To | Reason |
|------|------|-------|--------|
| utils.ts | Unknown | Infra | Utility functions |
| api.ts | Unknown | Infra | API client |

## 🟡 Phase 2: MEDIUM Risk (1 file)
| File | From | To | Reason |
|------|------|-------|--------|
| types.ts | Unknown | Kernel | 5 consumers |
```

### Example 2: Rust Crate Reorganization

**Directory:** `src/cli/`

**Command:**
```bash
loct plan src/cli --script > reorganize-cli.sh
./reorganize-cli.sh --dry
```

**Output (excerpt):**
```bash
phase_1 () {
    echo -e "${GREEN}=== Phase 1: LOW Risk ===${NC}"
    echo "Moving 10 files..."

    run mkdir -p "src/cli/infra"
    run git mv "src/cli/helpers.rs" "src/cli/infra/helpers.rs"
    run git mv "src/cli/utils.rs" "src/cli/infra/utils.rs"
    # ...
}
```

### Example 3: CI Integration

**GitHub Actions:**
```yaml
- name: Architectural Review
  run: |
    loct plan --json > plan.json
    FILES_TO_MOVE=$(jq '.stats.files_to_move' plan.json)
    if [ "$FILES_TO_MOVE" -gt 0 ]; then
      echo "::warning::$FILES_TO_MOVE files may need reorganization"
      jq '.stats' plan.json
    fi
```

---

## Integration with Other Commands

### Before Planning

```bash
# Understand current state
loct health                    # Quick health check
loct focus <dir>               # Directory context
loct hotspots                  # Find hub files
```

### During Execution

```bash
# Verify each move
loct impact <moved-file>       # What breaks?
loct cycles                    # Check for new cycles
```

### After Migration

```bash
# Validate results
loct health                    # Re-check health
loct audit                     # Full audit
loct report --html             # Visual report
```

---

## Related Commands

| Command | Purpose |
|---------|---------|
| `loct impact <file>` | Analyze what breaks if a file is changed |
| `loct focus <dir>` | Extract directory context (deps + consumers) |
| `loct cycles` | Detect and classify circular imports |
| `loct audit` | Full codebase audit (dead + cycles + twins) |
| `loct health` | Quick health summary |

---

## Architecture

The Refactor Strategist uses these loctree building blocks:

1. **HolographicFocus** - Extracts files in target directory with their dependencies
2. **ImpactAnalysis** - Calculates direct and transitive consumers for risk scoring
3. **Tarjan SCC** - Detects cyclic dependency groups
4. **LayerDetection** - Classifies files by architectural layer via path heuristics
5. **ShimGeneration** - Creates re-export code for backward compatibility

**Performance:** <3s for 500-file directories (uses cached snapshot, no re-scanning)

---

*Vibecrafted with AI Agents by VetCoders (c)2026 VetCoders*
