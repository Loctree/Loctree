# Query Mode - jq-like Codebase Queries

`loct` provides a jq-like interface for querying loctree snapshots. Instead of parsing JSON manually, use familiar filter syntax to extract insights from your codebase analysis.

## Quick Start

### What is Query Mode?

Query mode lets you interrogate loctree snapshots using jq-style filters. Unlike raw jq, `loct` automatically discovers the latest snapshot and understands the codebase schema.

```bash
# Basic filter - get snapshot metadata
loct '.metadata'

# Get all file paths
loct '.files[].path'

# Count dead exports
loct '.files | map(.exports | map(select(.dead == true))) | flatten | length'
```

### How it Differs from jq

| Feature | jq | loct query |
|---------|-----|------------|
| Snapshot discovery | Manual path | Auto-discovers `.loctree/*/snapshot.json` |
| Schema awareness | None | Validates against snapshot schema |
| Preset queries | None | `@imports`, `@exports`, `@dead`, etc. |
| Error messages | Generic JSON errors | Codebase-specific hints |

## Usage

```
loct [OPTIONS] <FILTER>
loct [OPTIONS] @<PRESET> [ARGS]
```

### Arguments

- `<FILTER>` - jq-compatible filter expression
- `@<PRESET>` - Named preset query (see Preset Queries)

### Examples

```bash
# Raw filter
loct '.metadata.languages'

# Preset query
loct @imports src/utils/auth.ts

# Preset with options
loct @dead --confidence high
```

## Filter Syntax

Query mode uses jq filter syntax. For comprehensive jq documentation, see [jq Manual](https://jqlang.github.io/jq/manual/).

### Common Patterns

#### Object Access
```bash
# Single field
loct '.metadata'

# Nested field
loct '.metadata.git_branch'

# Optional field (no error if missing)
loct '.metadata.git_repo?'
```

#### Array Operations
```bash
# All elements
loct '.files[]'

# First element
loct '.files[0]'

# Slice
loct '.files[0:10]'

# Length
loct '.files | length'
```

#### Filtering
```bash
# Select by condition
loct '.files[] | select(.loc > 500)'

# Select by path pattern
loct '.files[] | select(.path | contains("components"))'

# Multiple conditions
loct '.files[] | select(.loc > 100 and .exports | length > 5)'
```

#### Transformation
```bash
# Extract specific fields
loct '.files[] | {path, loc, language}'

# Map over array
loct '.files | map(.path)'

# Flatten nested arrays
loct '.files | map(.exports) | flatten'
```

#### Aggregation
```bash
# Count
loct '.files | length'

# Sum
loct '[.files[].loc] | add'

# Group by
loct '.files | group_by(.language)'

# Sort
loct '.files | sort_by(.loc) | reverse | .[0:10]'
```

## Options

### Output Format

| Option | Description |
|--------|-------------|
| `-r, --raw-output` | Output strings without JSON quotes |
| `-c, --compact` | Compact JSON output (single line) |

```bash
# Get paths as raw strings (one per line)
loct -r '.files[].path'

# Compact JSON for piping
loct -c '.metadata' | other-tool
```

### Variables

| Option | Description |
|--------|-------------|
| `--arg NAME VALUE` | Bind `$NAME` to string `VALUE` |
| `--argjson NAME JSON` | Bind `$NAME` to parsed JSON |

```bash
# Filter by variable
loct --arg file "auth.ts" '.files[] | select(.path | endswith($file))'

# Numeric threshold
loct --argjson threshold 500 '.files[] | select(.loc > $threshold)'
```

### Snapshot Selection

| Option | Description |
|--------|-------------|
| `--snapshot PATH` | Use specific snapshot file instead of auto-discovery |

```bash
# Use specific snapshot
loct --snapshot .loctree/main@abc123/snapshot.json '.metadata'

# Compare two snapshots
diff <(loct --snapshot old.json '.files | length') \
     <(loct --snapshot new.json '.files | length')
```

### Exit Status

| Option | Description |
|--------|-------------|
| `-e, --exit-status` | Set exit code based on result |

```bash
# Exit 1 if no dead exports found (for CI)
loct -e '.files | map(.exports) | flatten | map(select(.dead)) | length > 0'
```

## Preset Queries

Presets are optimized queries for common operations. They handle edge cases and provide better output formatting.

### @imports - Find Importers

Find all files that import a given file (follows re-export chains).

```bash
loct @imports <FILE>
```

**Examples:**
```bash
# Who imports this component?
loct @imports src/components/Button.tsx

# Raw output for scripting
loct @imports src/utils/auth.ts -r

# JSON for AI agents
loct @imports src/hooks/usePatient.ts --json
```

**Output:**
```json
{
  "target": "src/components/Button.tsx",
  "importers": [
    { "file": "src/App.tsx", "line": 5, "via": "import" },
    { "file": "src/pages/Home.tsx", "line": 12, "via": "reexport" }
  ],
  "count": 2
}
```

### @exports - List Exports

List all symbols exported by a file.

```bash
loct @exports <FILE>
```

**Examples:**
```bash
# What does this file export?
loct @exports src/utils/helpers.ts

# Just the names
loct @exports src/api/index.ts -r | sort
```

**Output:**
```json
{
  "file": "src/utils/helpers.ts",
  "exports": [
    { "name": "formatDate", "kind": "function", "line": 10, "dead": false },
    { "name": "parseId", "kind": "function", "line": 25, "dead": true }
  ],
  "count": 2
}
```

### @dead - Unused Exports

Find exports with no importers (dead code candidates).

```bash
loct @dead [OPTIONS]
```

**Options:**
- `--confidence <LEVEL>` - Filter by confidence: `high`, `normal` (default), `low`
- `--path <PATTERN>` - Filter by file path pattern
- `--limit <N>` - Limit results (default: 20)

**Examples:**
```bash
# All dead exports
loct @dead

# High confidence only (safer to remove)
loct @dead --confidence high

# Dead exports in specific directory
loct @dead --path "components/"

# JSON output with full details
loct @dead --json
```

**Output:**
```json
{
  "dead_exports": [
    {
      "file": "src/utils/legacy.ts",
      "name": "oldHelper",
      "line": 42,
      "confidence": "high",
      "reason": "No imports found, not in entry point"
    }
  ],
  "count": 1
}
```

### @cycles - Circular Imports

Find circular import chains.

```bash
loct @cycles [OPTIONS]
```

**Options:**
- `--path <PATTERN>` - Filter cycles involving this path
- `--min-length <N>` - Minimum cycle length to report

**Examples:**
```bash
# All cycles
loct @cycles

# Cycles involving auth
loct @cycles --path "auth"

# Only complex cycles (3+ files)
loct @cycles --min-length 3
```

**Output:**
```json
{
  "cycles": [
    {
      "files": ["src/a.ts", "src/b.ts", "src/c.ts"],
      "length": 3,
      "severity": "warning"
    }
  ],
  "count": 1
}
```

### @who-imports - Transitive Importers

Find all files that depend on a file (transitive closure).

```bash
loct @who-imports <FILE>
```

**Examples:**
```bash
# Blast radius of a change
loct @who-imports src/core/types.ts

# Count affected files
loct @who-imports src/api/client.ts | jq '.count'
```

### @where-symbol - Symbol Lookup

Find where a symbol is defined.

```bash
loct @where-symbol <SYMBOL>
```

**Examples:**
```bash
# Find definition of a function
loct @where-symbol useAuth

# Find all files exporting a name
loct @where-symbol Button --all
```

### @barrels - Barrel File Analysis

Analyze barrel files (index.ts re-exports).

```bash
loct @barrels [OPTIONS]
```

**Options:**
- `--deep` - Show deep re-export chains
- `--missing` - Show directories without barrels

**Examples:**
```bash
# List all barrels
loct @barrels

# Find barrel chains
loct @barrels --deep
```

### @commands - Tauri Command Bridges

List Tauri FE/BE command mappings.

```bash
loct @commands [OPTIONS]
```

**Options:**
- `--missing` - Only show missing handlers
- `--unused` - Only show unused handlers
- `--name <PATTERN>` - Filter by command name

**Examples:**
```bash
# All commands
loct @commands

# Missing handlers (FE calls without BE impl)
loct @commands --missing

# Filter by name
loct @commands --name "auth"
```

### @events - Event Bridge Analysis

List event emit/listen pairs.

```bash
loct @events [OPTIONS]
```

**Options:**
- `--orphan` - Show events without listeners
- `--ghost` - Show listeners without emitters

**Examples:**
```bash
# All events
loct @events

# Orphan events (emitted but never handled)
loct @events --orphan
```

## Examples

### Common Workflows

#### Find All Files Importing a Component

```bash
# Using preset
loct @imports src/components/Button.tsx

# Using raw filter
loct '.edges[] | select(.to | endswith("Button.tsx")) | .from'
```

#### Count Dead Exports by Directory

```bash
loct '.files | group_by(.path | split("/")[1]) |
  map({
    dir: .[0].path | split("/")[1],
    dead: [.[].exports[] | select(.dead)] | length
  }) |
  sort_by(.dead) | reverse'
```

#### Find Circular Dependencies Involving a File

```bash
# Preset
loct @cycles --path "auth"

# Raw filter
loct '.cycles[] | select(any(. | contains("auth")))'
```

#### Get Blast Radius of a File

```bash
# How many files would be affected by changing this?
loct @who-imports src/core/types.ts -c | jq '.count'

# List affected files
loct @who-imports src/api/client.ts -r
```

#### Export Data for External Tools

```bash
# CSV of large files
loct -r '.files[] | select(.loc > 500) | [.path, .loc, .language] | @csv'

# TSV for spreadsheet
loct -r '.files[] | [.path, .loc] | @tsv' > files.tsv

# Markdown table
loct '.files | sort_by(.loc) | reverse | .[0:10] |
  ["| File | LOC |", "|------|-----|"] +
  map("| \(.path) | \(.loc) |") | .[]' -r
```

#### CI Integration

```bash
# Fail if new dead exports
loct -e '@dead --confidence high | .count == 0'

# Fail if cycles exist
loct -e '@cycles | .count == 0'

# Check specific file isn't orphaned
loct -e '@imports src/utils/helper.ts | .count > 0'
```

## Snapshot Structure Reference

The snapshot JSON has the following structure:

```json
{
  "metadata": {
    "schema_version": "0.5.0-rc",
    "generated_at": "2025-01-15T10:30:00Z",
    "roots": ["src", "lib"],
    "languages": ["typescript", "rust"],
    "file_count": 150,
    "total_loc": 25000,
    "scan_duration_ms": 1234,
    "git_repo": "my-app",
    "git_branch": "main",
    "git_commit": "abc1234"
  },
  "files": [
    {
      "path": "src/main.ts",
      "language": "typescript",
      "loc": 150,
      "exports": [
        {
          "name": "main",
          "kind": "function",
          "export_type": "named",
          "line": 10,
          "dead": false
        }
      ],
      "imports": ["./utils", "./config"],
      "tauri_commands": [...],
      "event_emits": [...],
      "event_listens": [...]
    }
  ],
  "edges": [
    {
      "from": "src/app.ts",
      "to": "src/utils.ts",
      "label": "import"
    }
  ],
  "export_index": {
    "Button": ["src/components/Button.tsx"],
    "useAuth": ["src/hooks/useAuth.ts", "src/legacy/auth.ts"]
  },
  "command_bridges": [
    {
      "name": "get_user",
      "frontend_calls": [["src/api.ts", 42]],
      "backend_handler": ["src-tauri/commands.rs", 100],
      "has_handler": true,
      "is_called": true
    }
  ],
  "event_bridges": [
    {
      "name": "user_updated",
      "emits": [["src/user.rs", 50, "emit"]],
      "listens": [["src/App.tsx", 30]]
    }
  ],
  "barrels": [
    {
      "path": "src/components/index.ts",
      "module_id": "src/components",
      "reexport_count": 15,
      "targets": ["src/components/Button.tsx", "..."]
    }
  ]
}
```

### Key Fields

| Path | Type | Description |
|------|------|-------------|
| `.metadata` | object | Scan metadata (version, git info, stats) |
| `.files` | array | All analyzed files with exports/imports |
| `.files[].exports` | array | Symbols exported by the file |
| `.files[].exports[].dead` | bool | Whether export has no importers |
| `.edges` | array | Import graph edges (from -> to) |
| `.export_index` | object | Symbol name -> files mapping |
| `.command_bridges` | array | Tauri FE/BE command mappings |
| `.event_bridges` | array | Event emit/listen pairs |
| `.barrels` | array | Detected barrel files (index.ts) |

## Tips and Tricks

### Combine with Other Tools

```bash
# Pipe to rg for text search in results
loct '.files[].path' -r | rg "component"

# Use fzf for interactive selection
loct '.files[].path' -r | fzf | xargs loct @imports

# Generate AI context
loct @dead --json | claude "Review these dead exports"
```

### Performance

```bash
# Use --snapshot for repeated queries (avoids re-discovery)
SNAP=$(loct '.metadata.git_scan_id' -r)
loct --snapshot ".loctree/$SNAP/snapshot.json" '.files | length'
loct --snapshot ".loctree/$SNAP/snapshot.json" '@dead'
```

### Debugging

```bash
# See raw snapshot structure
loct '.' | head -100

# Check schema version
loct '.metadata.schema_version'

# Validate snapshot exists
loct '.metadata' || echo "No snapshot found - run 'loct scan' first"
```

---

Developed by The Loctree Team (c)2025.
