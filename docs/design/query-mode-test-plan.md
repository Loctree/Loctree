# Loctree Query Mode - Comprehensive Test Plan

**Feature**: jq-style snapshot queries
**Command**: `loct <filter>`, `loct @preset <args>`, `loct query <filter>`
**Author**: AI Agent Design Document
**Date**: 2025-12-11
**Status**: Design Phase

---

## Overview

Query mode enables jq-style filtering on loctree snapshots without re-scanning.
This document defines test cases for implementation.

### Command Syntax

```bash
# Filter syntax (jq-compatible)
loct '.metadata'                          # Simple field access
loct '.metadata.git_branch'               # Nested access
loct '.edges[]'                           # Array iteration
loct '.edges | length'                    # Pipe to function
loct '.edges[] | select(.to == "foo")'    # Filter with select

# Preset queries (shorthand)
loct @imports src/foo.ts                  # Files that import foo.ts
loct @exports src/foo.ts                  # Symbols exported by foo.ts
loct @dead                                # Dead exports list
loct @consumers src/foo.ts                # Files that consume foo.ts

# Output flags
loct '.metadata.git_branch' -r            # Raw output (no quotes)
loct '.edges' -c                          # Compact JSON
loct '.edges | length' -e                 # Exit code based on result

# Explicit snapshot path
loct '.metadata' --snapshot .loctree/snapshot.json
```

---

## 1. Filter Detection Tests

These tests verify the CLI correctly identifies filter expressions vs subcommands.

### 1.1 Dot-prefix filter detection

| Test ID | Input | Expected | Validates |
|---------|-------|----------|-----------|
| FD-001 | `loct '.metadata'` | Recognized as filter | Dot-prefix triggers filter mode |
| FD-002 | `loct '.metadata.git_branch'` | Recognized as filter | Nested dot-prefix works |
| FD-003 | `loct '.'` | Recognized as filter | Root selector works |
| FD-004 | `loct '.files[0]'` | Recognized as filter | Array index with dot-prefix |
| FD-005 | `loct '.edges[]'` | Recognized as filter | Array iteration with dot-prefix |

### 1.2 Bracket-prefix filter detection

| Test ID | Input | Expected | Validates |
|---------|-------|----------|-----------|
| FD-010 | `loct '[.metadata]'` | Recognized as filter | Bracket wrapping |
| FD-011 | `loct '[ .files[] ]'` | Recognized as filter | Bracket with spaces |
| FD-012 | `loct '[.edges[] | .from]'` | Recognized as filter | Complex bracket expression |

### 1.3 Subcommand disambiguation

| Test ID | Input | Expected | Validates |
|---------|-------|----------|-----------|
| FD-020 | `loct scan` | Recognized as subcommand | Known subcommand not treated as filter |
| FD-021 | `loct tree` | Recognized as subcommand | Another known subcommand |
| FD-022 | `loct dead` | Recognized as subcommand | Subcommand without args |
| FD-023 | `loct info` | Recognized as subcommand | Single-word subcommand |
| FD-024 | `loct query .metadata` | Explicit query subcommand | Explicit query mode |

### 1.4 Preset detection

| Test ID | Input | Expected | Validates |
|---------|-------|----------|-----------|
| FD-030 | `loct @imports foo.ts` | Recognized as preset | At-prefix triggers preset |
| FD-031 | `loct @exports foo.ts` | Recognized as preset | Known preset name |
| FD-032 | `loct @dead` | Recognized as preset | Preset without args |
| FD-033 | `loct @consumers foo.ts` | Recognized as preset | Consumer preset |
| FD-034 | `loct @unknown` | Error: unknown preset | Invalid preset rejected |

### 1.5 Edge cases in detection

| Test ID | Input | Expected | Validates |
|---------|-------|----------|-----------|
| FD-040 | `loct metadata` | Error or treated as path | Bare word not a filter |
| FD-041 | `loct .hidden-file.ts` | Ambiguous - file or filter? | Document behavior |
| FD-042 | `loct ''` | Error: empty filter | Empty string rejected |
| FD-043 | `loct ' .metadata '` | Recognized as filter | Whitespace trimmed |

---

## 2. Snapshot Discovery Tests

These tests verify correct snapshot file discovery and override behavior.

### 2.1 Auto-discovery (newest snapshot)

| Test ID | Setup | Command | Expected | Validates |
|---------|-------|---------|----------|-----------|
| SD-001 | Single `.loctree/snapshot.json` | `loct '.metadata'` | Uses that snapshot | Basic discovery |
| SD-002 | Multiple: `main@abc123/`, `develop@def456/` | `loct '.metadata'` | Uses newest by mtime | Multi-snapshot selection |
| SD-003 | Only `develop@xyz/snapshot.json` | `loct '.metadata'` | Uses branch-specific | Branch isolation works |
| SD-004 | Empty `.loctree/` directory | `loct '.metadata'` | Error: no snapshot found | Clear error on missing |
| SD-005 | No `.loctree/` directory | `loct '.metadata'` | Error: no snapshot found | Missing dir handled |

### 2.2 Explicit snapshot path

| Test ID | Setup | Command | Expected | Validates |
|---------|-------|---------|----------|-----------|
| SD-010 | Multiple snapshots exist | `loct '.metadata' --snapshot .loctree/old.json` | Uses specified file | Override works |
| SD-011 | Specified file missing | `loct '.metadata' --snapshot missing.json` | Error: file not found | Clear error |
| SD-012 | Invalid JSON file | `loct '.metadata' --snapshot broken.json` | Error: invalid snapshot | Parse error |
| SD-013 | Valid JSON, wrong schema | `loct '.metadata' --snapshot other.json` | Error or warning | Schema validation |

### 2.3 Directory traversal

| Test ID | Setup | Command | Expected | Validates |
|---------|-------|---------|----------|-----------|
| SD-020 | Run from subdirectory | `loct '.metadata'` | Finds parent's `.loctree/` | Upward search |
| SD-021 | Run from project root | `loct '.metadata'` | Finds `./.loctree/` | Direct path |
| SD-022 | Multiple `.loctree/` in ancestors | `loct '.metadata'` | Uses nearest | Closest wins |

---

## 3. Filter Execution Tests

These tests verify correct jq filter execution on snapshot data.

### 3.1 Simple field access

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-001 | `.metadata` | Full metadata object | Top-level field |
| FE-002 | `.files` | Array of file analyses | Array field |
| FE-003 | `.edges` | Array of graph edges | Another array |
| FE-004 | `.barrels` | Array of barrel files | Optional field |
| FE-005 | `.nonexistent` | `null` | Missing field returns null |

### 3.2 Nested field access

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-010 | `.metadata.git_branch` | `"develop"` (or current branch) | Nested string |
| FE-011 | `.metadata.file_count` | `44` (or current count) | Nested number |
| FE-012 | `.metadata.schema_version` | `"0.5.0-rc"` | Nested string |
| FE-013 | `.metadata.languages` | `["rs"]` or `["ts", "rs"]` | Nested array |
| FE-014 | `.metadata.nonexistent.deep` | `null` | Deep missing returns null |

### 3.3 Array iteration

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-020 | `.files[]` | Each file as separate output | Array iteration |
| FE-021 | `.edges[]` | Each edge as separate output | Edge iteration |
| FE-022 | `.files[0]` | First file analysis | Index access |
| FE-023 | `.files[-1]` | Last file analysis | Negative index |
| FE-024 | `.files[0:3]` | First 3 files | Slice notation |

### 3.4 Field extraction from arrays

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-030 | `.files[].path` | List of all file paths | Field from each |
| FE-031 | `.edges[].from` | List of all source files | Edge sources |
| FE-032 | `.edges[].to` | List of all target files | Edge targets |
| FE-033 | `.files[].exports[].name` | All export names | Nested array extraction |

### 3.5 Select/filter expressions

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-040 | `.edges[] \| select(.to == "src/types.rs")` | Edges pointing to types.rs | Equality select |
| FE-041 | `.files[] \| select(.loc > 500)` | Files with >500 LOC | Numeric comparison |
| FE-042 | `.files[] \| select(.language == "ts")` | TypeScript files only | String match |
| FE-043 | `.files[] \| select(.is_test == true)` | Test files only | Boolean match |
| FE-044 | `.edges[] \| select(.label == "reexport")` | Re-export edges only | Label filter |
| FE-045 | `.files[] \| select(.path \| contains("utils"))` | Files with utils in path | String contains |
| FE-046 | `.files[] \| select(.path \| test(".*\\.rs$"))` | Rust files (regex) | Regex match |

### 3.6 Variable binding (--arg)

| Test ID | Command | Expected Output | Validates |
|---------|---------|-----------------|-----------|
| FE-050 | `loct '.edges[] \| select(.to == $file)' --arg file src/types.rs` | Edges to types.rs | String variable |
| FE-051 | `loct '.files[] \| select(.loc > $min)' --argjson min 100` | Files >100 LOC | JSON number variable |
| FE-052 | `loct '.edges[] \| select(.from == $f and .to == $t)' --arg f a.ts --arg t b.ts` | Specific edge | Multiple variables |

### 3.7 Aggregate functions

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-060 | `.edges \| length` | Number of edges | Array length |
| FE-061 | `.files \| length` | Number of files | File count |
| FE-062 | `[.files[].loc] \| add` | Total LOC | Sum |
| FE-063 | `[.files[].loc] \| max` | Largest file LOC | Max |
| FE-064 | `[.files[].loc] \| min` | Smallest file LOC | Min |
| FE-065 | `.files \| group_by(.language) \| length` | Language count | Group by |
| FE-066 | `[.files[] \| select(.loc > 500)] \| length` | Count of large files | Filtered count |

### 3.8 Object construction

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-070 | `.files[] \| {path, loc}` | Objects with path and loc | Field subset |
| FE-071 | `.edges[] \| {source: .from, target: .to}` | Renamed fields | Field renaming |
| FE-072 | `.files[] \| {path, exports: (.exports \| length)}` | Computed field | Nested computation |

### 3.9 Sorting and limiting

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| FE-080 | `.files \| sort_by(.loc) \| reverse` | Files by LOC descending | Sort reverse |
| FE-081 | `.files \| sort_by(.loc) \| .[:10]` | Top 10 by LOC | Sort + limit |
| FE-082 | `.edges \| unique_by(.to)` | Unique targets | Deduplication |
| FE-083 | `[.files[] \| .path] \| sort` | Sorted file paths | Simple sort |

---

## 4. Output Flag Tests

These tests verify output formatting flags work correctly.

### 4.1 Raw output (-r)

| Test ID | Command | Expected Output | Validates |
|---------|---------|-----------------|-----------|
| OF-001 | `loct '.metadata.git_branch' -r` | `develop` (no quotes) | String unquoted |
| OF-002 | `loct '.metadata.file_count' -r` | `44` | Number unchanged |
| OF-003 | `loct '.files[0].path' -r` | `src/analyzer/assets.rs` | Path unquoted |
| OF-004 | `loct '.metadata' -r` | JSON object (formatted) | Objects unchanged |
| OF-005 | `loct '.files[].path' -r` | One path per line | Array items unquoted |

### 4.2 Compact output (-c)

| Test ID | Command | Expected Output | Validates |
|---------|---------|-----------------|-----------|
| OF-010 | `loct '.metadata' -c` | Single-line JSON | No pretty-print |
| OF-011 | `loct '.files[0]' -c` | Compact file JSON | Object compacted |
| OF-012 | `loct '.edges[:3]' -c` | Compact array | Array compacted |

### 4.3 Exit code mode (-e)

| Test ID | Command | Expected Exit Code | Validates |
|---------|---------|-------------------|-----------|
| OF-020 | `loct '.metadata' -e` | 0 | Non-null result |
| OF-021 | `loct '.nonexistent' -e` | 1 | Null result |
| OF-022 | `loct 'false' -e` | 1 | False result |
| OF-023 | `loct '""' -e` | 1 | Empty string |
| OF-024 | `loct '0' -e` | 0 | Zero is truthy |
| OF-025 | `loct '[]' -e` | 1 | Empty array is falsy |
| OF-026 | `loct '.edges \| length > 0' -e` | 0 | True condition |

### 4.4 Combined flags

| Test ID | Command | Expected | Validates |
|---------|---------|----------|-----------|
| OF-030 | `loct '.files[].path' -rc` | Compact paths, unquoted | Raw + compact |
| OF-031 | `loct '.edges \| length' -re` | Count, exit 0 if >0 | Raw + exit |

---

## 5. Preset Query Tests

These tests verify preset queries work correctly.

### 5.1 @imports preset

| Test ID | Command | Expected Output | Validates |
|---------|---------|-----------------|-----------|
| PQ-001 | `loct @imports src/types.rs` | Files importing types.rs | Basic imports |
| PQ-002 | `loct @imports src/nonexistent.ts` | `[]` (empty array) | Missing file |
| PQ-003 | `loct @imports types.rs` | Same as full path | Path normalization |
| PQ-004 | `loct @imports src/analyzer/` | Files importing from dir | Directory import |

### 5.2 @exports preset

| Test ID | Command | Expected Output | Validates |
|---------|---------|-----------------|-----------|
| PQ-010 | `loct @exports src/types.rs` | Export symbols | File exports |
| PQ-011 | `loct @exports src/nonexistent.ts` | Error or empty | Missing file |
| PQ-012 | `loct @exports src/analyzer/mod.rs` | Module exports | Rust mod exports |

### 5.3 @dead preset

| Test ID | Command | Expected Output | Validates |
|---------|---------|-----------------|-----------|
| PQ-020 | `loct @dead` | Dead export list | All dead exports |
| PQ-021 | `loct @dead --json` | JSON format dead | JSON output |
| PQ-022 | `loct @dead --top 5` | Top 5 dead exports | Limit works |

### 5.4 @consumers preset

| Test ID | Command | Expected Output | Validates |
|---------|---------|-----------------|-----------|
| PQ-030 | `loct @consumers src/types.rs` | Files that use types.rs | Transitive consumers |
| PQ-031 | `loct @consumers src/main.rs` | Entry point consumers | Entry file |

### 5.5 Unknown preset

| Test ID | Command | Expected | Validates |
|---------|---------|----------|-----------|
| PQ-040 | `loct @unknown` | Error: unknown preset '@unknown' | Clear error |
| PQ-041 | `loct @` | Error: empty preset name | Empty preset |

---

## 6. Edge Cases

### 6.1 Empty/null results

| Test ID | Filter | Expected Output | Validates |
|---------|--------|-----------------|-----------|
| EC-001 | `.edges[] \| select(.to == "nonexistent")` | Empty output | No matches |
| EC-002 | `.barrels` (when empty) | `[]` | Empty array |
| EC-003 | `.files[999]` | `null` | Out of bounds |
| EC-004 | `.metadata.resolver_config` (when null) | `null` | Null field |

### 6.2 Invalid filter syntax

| Test ID | Filter | Expected | Validates |
|---------|--------|----------|-----------|
| EC-010 | `.metadata[` | Error: parse error | Unclosed bracket |
| EC-011 | `.edges \| select(` | Error: parse error | Unclosed paren |
| EC-012 | `.files \| unknownfn()` | Error: unknown function | Bad function |
| EC-013 | `. \| . \| . \| .` | Success (identity chain) | Valid but weird |

### 6.3 Snapshot edge cases

| Test ID | Scenario | Command | Expected | Validates |
|---------|----------|---------|----------|-----------|
| EC-020 | Snapshot with 0 files | `.files \| length` | `0` | Empty snapshot |
| EC-021 | Snapshot with 0 edges | `.edges \| length` | `0` | No edges |
| EC-022 | Corrupted JSON | `.metadata` | Error: invalid snapshot | Parse failure |
| EC-023 | Schema v0.4 snapshot | `.metadata` | Warning or error | Schema mismatch |

### 6.4 Special characters

| Test ID | Filter | Expected | Validates |
|---------|--------|----------|-----------|
| EC-030 | `.files[] \| select(.path \| contains("$"))` | Files with $ in path | Special char in string |
| EC-031 | `.edges[] \| select(.to == "src/types.d.ts")` | Edge to .d.ts | Dots in value |
| EC-032 | Filter with Unicode | Appropriate handling | Unicode support |

---

## 7. Integration Tests

### 7.1 Workflow: Scan then Query

```bash
# Setup: Fresh project
loct scan                              # Create snapshot
loct '.metadata.file_count'            # Query: should show count
loct '.edges | length'                 # Query: should show edge count
```

| Test ID | Step | Expected | Validates |
|---------|------|----------|-----------|
| IT-001 | After scan | Snapshot exists | Scan creates snapshot |
| IT-002 | Query after scan | Returns data | Query uses snapshot |
| IT-003 | Multiple queries | Consistent results | Snapshot stable |

### 7.2 Workflow: Branch switching

```bash
git checkout feature-branch
loct scan                              # Branch snapshot
loct '.metadata.git_branch'            # Should show feature-branch
git checkout main
loct '.metadata.git_branch'            # Should find main snapshot or error
```

| Test ID | Scenario | Expected | Validates |
|---------|----------|----------|-----------|
| IT-010 | Query on branch | Branch snapshot used | Branch isolation |
| IT-011 | Switch branch, no snapshot | Error: no snapshot | Clear error |

### 7.3 Comparison with dead command

```bash
# These should produce equivalent results
loct dead --json
loct @dead --json
loct '.files[] | select(.exports[] | select(.import_count == 0)) | .path'
```

| Test ID | Comparison | Expected | Validates |
|---------|------------|----------|-----------|
| IT-020 | @dead vs dead command | Same results | Preset matches command |
| IT-021 | Filter vs preset | Same results | Filter equivalent |

---

## 8. Performance Tests

### 8.1 Large snapshot handling

| Test ID | Scenario | Metric | Threshold | Validates |
|---------|----------|--------|-----------|-----------|
| PT-001 | 10k files snapshot | Query time | < 100ms | Fast queries |
| PT-002 | 100k edges snapshot | Filter time | < 500ms | Edge handling |
| PT-003 | Deep nesting filter | Memory | < 100MB | Memory bound |

### 8.2 Repeated queries

| Test ID | Scenario | Expected | Validates |
|---------|----------|----------|-----------|
| PT-010 | 100 sequential queries | Consistent time | No degradation |
| PT-011 | Parallel queries | All succeed | Thread safety |

---

## 9. Test Fixtures Required

### 9.1 Fixture: simple_ts_for_query

```
simple_ts_for_query/
  .loctree/
    snapshot.json          # Pre-generated snapshot
  src/
    index.ts               # Main entry
    utils/
      helper.ts            # Used by index
      unused.ts            # Dead export
    types.ts               # Shared types
```

Snapshot should contain:
- 4 files
- At least 3 edges
- 1 dead export in unused.ts
- metadata with git info

### 9.2 Fixture: multi_snapshot

```
multi_snapshot/
  .loctree/
    snapshot.json                    # Current
    main@abc1234/snapshot.json       # Old main
    develop@def5678/snapshot.json    # Old develop
```

### 9.3 Fixture: empty_snapshot

```
empty_snapshot/
  .loctree/
    snapshot.json          # Valid but empty: {metadata: {...}, files: [], edges: []}
```

### 9.4 Fixture: invalid_snapshots

```
invalid_snapshots/
  broken.json              # Invalid JSON
  old_schema.json          # Valid JSON, wrong schema version
  missing_fields.json      # Missing required fields
```

---

## 10. Implementation Notes

### 10.1 Recommended jq library

For Rust implementation, consider:
- `jaq` crate (Rust-native jq implementation)
- `serde_json` + custom evaluator
- Shell out to `jq` binary (simpler but requires jq installed)

### 10.2 Filter detection regex

```rust
fn is_filter_expression(arg: &str) -> bool {
    let arg = arg.trim();
    // Dot-prefix: .metadata, .files[], etc.
    if arg.starts_with('.') {
        return true;
    }
    // Bracket-prefix: [.metadata], [.files[]]
    if arg.starts_with('[') && arg.ends_with(']') {
        return true;
    }
    // Pipe expression without leading dot: length, keys, etc.
    // These would need explicit `loct query` prefix
    false
}

fn is_preset(arg: &str) -> bool {
    arg.starts_with('@')
}
```

### 10.3 Snapshot discovery algorithm

```rust
fn find_snapshot(explicit: Option<&Path>) -> Result<PathBuf> {
    // 1. Explicit path takes precedence
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }

    // 2. Search upward for .loctree directory
    let mut current = std::env::current_dir()?;
    loop {
        let loctree_dir = current.join(".loctree");
        if loctree_dir.exists() {
            // 3. Find newest snapshot in directory
            return find_newest_snapshot(&loctree_dir);
        }
        if !current.pop() {
            break;
        }
    }

    Err(anyhow!("No snapshot found. Run `loct scan` first."))
}
```

---

## Appendix A: Expected CLI Help Text

```
loct query - Query snapshot data with jq-style filters

USAGE:
    loct <filter>                    # Direct filter (starts with . or [)
    loct @<preset> [args]            # Preset query
    loct query <filter> [OPTIONS]    # Explicit query command

FILTERS:
    .metadata                        Access metadata object
    .files[]                         Iterate over files
    .edges[] | select(.to == "x")    Filter edges
    .files | length                  Count files

PRESETS:
    @imports <file>                  Files that import <file>
    @exports <file>                  Symbols exported by <file>
    @consumers <file>                Files consuming <file>
    @dead                            Dead exports list

OUTPUT OPTIONS:
    -r, --raw-output                 Output strings without quotes
    -c, --compact-output             Compact JSON (no pretty-print)
    -e, --exit-status                Set exit code based on result

SNAPSHOT OPTIONS:
    --snapshot <path>                Use specific snapshot file

EXAMPLES:
    loct '.metadata.git_branch'
    loct '.files[] | select(.loc > 500) | .path' -r
    loct @imports src/utils.ts
    loct '.edges | length' -e
```

---

## Appendix B: Snapshot Schema Reference

```json
{
  "metadata": {
    "schema_version": "0.5.0-rc",
    "generated_at": "2025-12-11T10:30:00Z",
    "roots": ["/path/to/project"],
    "languages": ["ts", "rs"],
    "file_count": 100,
    "total_loc": 5000,
    "scan_duration_ms": 500,
    "git_repo": "Loctree",
    "git_branch": "develop",
    "git_commit": "abc1234"
  },
  "files": [
    {
      "path": "src/index.ts",
      "loc": 50,
      "language": "ts",
      "kind": "code",
      "is_test": false,
      "is_generated": false,
      "imports": [...],
      "exports": [
        {"name": "main", "kind": "function", "export_type": "named", "line": 10}
      ]
    }
  ],
  "edges": [
    {"from": "src/app.ts", "to": "src/utils.ts", "label": "import"},
    {"from": "src/index.ts", "to": "src/types.ts", "label": "reexport"}
  ],
  "barrels": [...],
  "export_index": {...},
  "command_bridges": [...],
  "event_bridges": [...]
}
```

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-12-11 | AI Agent | Initial test plan design |
