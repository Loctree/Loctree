# Search Tools Comparison Map

## Tools Analyzed

| Tool | Type | Scope | Speed |
|------|------|-------|-------|
| `rg` (ripgrep) | Exact text | Respects `.gitignore` | Fast |
| `grep -rn` | Exact text | All files (incl. .venv) | Medium |
| `loct find` | Symbol + Semantic | Indexed codebase | Fast |
| `loct query` | Graph traversal | Dependency graph | Fast |
| `loct` commands | Static analysis | Full codebase | Varies |
| Claude `Grep` tool | Exact text (rg) | Respects `.gitignore` | Fast |

---

## Quick Reference: When to Use What

| Szukam... | Użyj | Dlaczego |
|-----------|------|----------|
| Dokładnego stringa | `rg` | Exact match, fast |
| Stringa w dependencies | `grep -rn` lub `rg --no-ignore` | Searches .venv |
| Eksportowanego symbolu | `loct find` lub `loct query where-symbol` | Symbol + semantic |
| Czegoś z literówką | `loct find` | Semantic recovery |
| Lokalnych zmiennych | `rg` | loct nie indeksuje locals |
| Komentarzy/docstringów | `rg` | Text search |
| Kto importuje plik? | `loct query who-imports` | Reverse deps |
| Co się zepsuje jak zmienię? | `loct impact` | Transitive analysis |
| Wszystkie FastAPI routes | `loct routes` | Framework-aware |
| Circular imports | `loct cycles` | Graph analysis |
| Unused code | `loct dead` / `loct zombie` | Static analysis |
| Duplicate symbols | `loct twins` | Cross-file detection |
| Directory overview | `loct focus` | LOC + deps tree |
| Health check | `loct health` / `loct audit` | Full report |

---

## Loct Command Reference

### Instant Commands (<100ms)

| Command | Description | JSON? | Use Case |
|---------|-------------|-------|----------|
| `loct find <pattern>` | Symbol + params + semantic search | ✅ | Find functions, classes, parameters by name |
| `loct query who-imports <file>` | Reverse dependencies | ✅ | "What files import this?" |
| `loct query where-symbol <sym>` | Symbol definition + re-exports | ✅ | "Where is X defined?" |
| `loct query component-of <file>` | Module ownership | ✅ | "What module owns this file?" |
| `loct slice <file>` | File dependencies + LOC | ✅ | "What does this file depend on?" |
| `loct impact <file>` | Transitive consumers | ✅ | "What breaks if I change this?" |
| `loct focus <dir>` | Directory context | ✅ | "Overview of this module" |
| `loct hotspots` | Import frequency heatmap | ✅ | "What are the core files?" |
| `loct health` | Quick health summary | ✅ | "Any obvious issues?" |

> **Note (v0.8.4)**: All commands now support `--json` output. `loct find` also searches function parameters!

### Analysis Commands

| Command | Description | JSON? | Use Case |
|---------|-------------|-------|----------|
| `loct dead` | Unused exports | ✅ | Find dead code |
| `loct cycles` | Circular imports | ✅ | Detect import cycles |
| `loct twins` | Duplicate symbol names | ✅ | Find naming conflicts |
| `loct zombie` | Dead + orphan + shadows | ✅ | Combined cleanup report |
| `loct coverage` | Test gaps (structural) | ✅ | "What's not tested?" |
| `loct audit` | Full codebase report | ❌ | Markdown health report |
| `loct sniff` | Code smells aggregate | ❌ | twins + dead + crowds |
| `loct crowd <kw>` | Functional clustering | ❌ | "Files related to X" |
| `loct tagmap <kw>` | Unified search | ❌ | files + crowd + dead |

### Framework-Specific

| Command | Description | JSON? | Use Case |
|---------|-------------|-------|----------|
| `loct routes` | FastAPI/Flask routes | ✅ | List all API endpoints |
| `loct commands` | Tauri FE↔BE handlers | ❌ | Frontend-backend bridges |
| `loct events` | Event emit/listen flow | ❌ | Event analysis |

### JQ Queries (on .loctree/snapshot.json)

```bash
loct '.metadata'              # Scan metadata
loct '.files | length'        # Count files
loct '.dead_parrots[]'        # List dead exports
loct '.cycles[]'              # List circular imports
```

---

## Test Results

### 1. Exported Symbol: `ResponsesAdapter`

| Tool | Result |
|------|--------|
| **rg** | ✅ 3 files, exact matches (usage count) |
| **loct find** | ✅ Symbol match + 19 semantic matches |
| **loct query where-symbol** | ✅ Exact definition + line number |

**Verdict**: OVERLAP - all find it, loct adds semantic context

---

### 2. Typo: `streeming` (meant: streaming)

| Tool | Result |
|------|--------|
| **rg** | ❌ No results |
| **loct find** | ⚠️ Semantic recovery: StreamOptions (0.46) |

**Verdict**: AUGMENT - loct recovers from typos, rg cannot

---

### 3. Local Variable: `request_model`

| Tool | Result |
|------|--------|
| **rg** | ✅ 14 hits in adapter.py |
| **loct find** | ⚠️ No symbol match (not exported) |
| **loct query where-symbol** | ❌ Not found |

**Verdict**: AUGMENT - rg finds locals, loct only indexes exports

---

### 4. Reverse Dependencies

| Tool | Result |
|------|--------|
| **rg** | ❌ Cannot do reverse deps |
| **loct query who-imports** | ✅ Full list with import type |

```
who-imports 'src/mlx_omni_server/responses/store.py':
  src/mlx_omni_server/responses/__init__.py - imports via import
  src/mlx_omni_server/responses/context_builder.py - imports via import
  src/mlx_omni_server/responses/router.py - imports via import
```

**Verdict**: UNIQUE to loct - graph traversal

---

### 5. Impact Analysis

| Tool | Result |
|------|--------|
| **rg** | ❌ Cannot analyze |
| **loct impact** | ✅ Direct + transitive consumers with depth |

```
Impact analysis for: src/mlx_omni_server/responses/schema.py
  Direct consumers (4 files)
  Transitive impact (13 files)
  [!] Removing would affect 17 files (max depth: 4)
```

**Verdict**: UNIQUE to loct - refactoring blast radius

---

### 6. API Routes Discovery

| Tool | Result |
|------|--------|
| **rg '@router'** | ⚠️ Raw decorators, needs parsing |
| **loct routes** | ✅ Parsed: method, path, handler, file, line |

```json
{
  "method": "POST",
  "path": "/v1/responses",
  "handler": "create_response",
  "file": "src/mlx_omni_server/responses/router.py",
  "line": 59
}
```

**Verdict**: AUGMENT - loct parses, rg just finds text

---

## Known Limitations

### Loct False Positives

1. ~~**Monkey-patching patterns**: compat.py exports symbols injected into `sys.modules` - loct sees them as dead~~ **FIXED in v0.8.4!** sys.modules injection now detected
2. **Aliased imports**: `from x import y as _y` with noqa may not be tracked
3. **Dynamic imports**: `importlib.import_module()` not detected

### Loct Requires Exact Names

```bash
loct query where-symbol BatchCoordinator      # ❌ Not found
loct query where-symbol BatchRequestCoordinator  # ✅ Found
```

Use `loct find` for fuzzy matching, `loct query` for exact.

---

## Overlap vs Augment Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         SEARCH SPACE                                 │
│                                                                      │
│  ┌────────────────────┐                                              │
│  │    rg / grep       │ ← Exact text, comments, locals, strings     │
│  │                    │                                              │
│  │  ┌─────────────────┼─────────────────┐                           │
│  │  │    OVERLAP      │                 │                           │
│  │  │  (exported      │   loct find     │ ← Semantic search,        │
│  │  │   symbols)      │                 │   typo recovery,          │
│  │  └─────────────────┼─────────────────┘   related symbols         │
│  │                    │                                              │
│  └────────────────────┘                                              │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    loct (UNIQUE)                              │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │   │
│  │  │ who-     │ │ impact   │ │ routes   │ │ cycles/dead/     │ │   │
│  │  │ imports  │ │ analysis │ │ parsing  │ │ twins/zombie     │ │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────────┘ │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  GAP: Abstract concepts, natural language queries, runtime behavior │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Recommended Workflows

### 1. Finding a Symbol
```bash
loct find MyClass              # Semantic + exact
rg 'class MyClass'             # If loct misses (local class)
```

### 2. Before Refactoring
```bash
loct impact src/module/file.py  # What breaks?
loct slice src/module/file.py   # What does it depend on?
loct query who-imports src/module/file.py  # Direct consumers
```

### 3. Code Cleanup
```bash
loct health                    # Quick overview
loct dead                      # Unused exports
loct twins                     # Duplicate names
loct zombie                    # Combined report
```

### 4. Understanding a Directory
```bash
loct focus src/responses/      # Overview with LOC + deps
loct hotspots                  # Core files in project
```

### 5. API Discovery
```bash
loct routes                    # All endpoints
loct routes --json | jq '.routes[] | select(.method=="POST")'
```

---

## AI Integration (Claude Code Hooks)

### Hook Augmentation (v10)

Claude Code's Grep tool is automatically augmented with loctree context via PostToolUse hooks:

```bash
# ~/.claude/hooks/loct-grep-augment.sh
# Every grep gets semantic context from loct find
```

**What Claude receives for each grep:**
- `symbol_matches`: Exact symbol definitions with file + line
- `param_matches`: Function parameters matching the pattern (NEW in 0.8.4)
- `semantic_matches`: Similar symbols with similarity scores
- `dead_status`: Whether the symbol is exported and/or dead

**Example:** Grep for `template` → Hook augments with:
- Symbol: `extract_vue_template`, `parse_svelte_template_usages`, `DynamicExecTemplate`
- Params: `template: &str` in `parse_vue_template_usages()`
- Semantic: Related symbols

### Best Practices for AI

1. **Use grep + hook augmentation** for exploratory searches
2. **Use `loct find` directly** when you need semantic matching (typo recovery)
3. **Use `loct impact`** before refactoring to know blast radius
4. **Use `loct slice`** to understand a file's dependencies

---

*Created by M&K (c)2026 VetCoders*
