# loctree CLI Global Options

Global options and environment variables for loctree CLI.

---

## Table of Contents

- [Global Options](#global-options)
- [Output Options](#output-options)
- [Scan Behavior Options](#scan-behavior-options)
- [Filter Options](#filter-options)
- [Environment Variables](#environment-variables)
- [Color Mode](#color-mode)
- [Examples](#examples)

---

## Global Options

These options can be used with any loctree command.

| Option | Description | Default |
|--------|-------------|---------|
| `--help`, `-h` | Show help for command | - |
| `--version` | Show version information | - |
| `--quiet` | Suppress non-essential output | Off |
| `--verbose` | Show detailed progress | Off |

**Examples:**
```bash
loct --help                # Main help
loct slice --help          # Help for slice command
loct --version             # Show version
loct scan --verbose        # Detailed progress
loct dead --quiet          # Minimal output
```

---

## Output Options

Control output format and content.

### `--json`

Output as JSON (stdout only). Works with most commands.

```bash
loct health --json
loct slice src/main.rs --json
loct dead --json
```

**Output:**
```json
{
  "summary": {
    "cycles": 3,
    "dead_exports": 12,
    "twins": 2,
    "health_score": 85
  }
}
```

---

### `--findings`

Output `findings.json` to stdout. Contains all issues (dead, cycles, twins...).

```bash
loct --findings
loct auto --findings > findings.json
```

**Use Case:**
- CI integration
- Automated checks
- Custom processing

---

### `--summary`

Output summary only to stdout. Quick health check format.

```bash
loct --summary
loct h                     # Alias for --summary
```

**Output:**
```
Health Check Summary

Cycles:      3 total (2 hard, 1 structural)
Dead:        6 high confidence, 24 low
Twins:       2 duplicate symbol groups
```

---

### `--for-ai`

Output AI context bundle (JSONL stream). Optimized for AI agents.

```bash
loct --for-ai
loct --for-ai > context.jsonl
```

**Alias:** `loct --for-agent-feed`

**Output Format:** JSONL (one JSON object per line)

---

### `--agent-json`

Emit single-shot agent JSON bundle (vs JSONL stream). Saved to `.loctree/agent.json`.

```bash
loct --agent-json
loct agent                 # Shortcut command
```

**Bundle Contains:**
- Handlers (Tauri commands)
- Duplicates (exact twins)
- Dead exports
- Dynamic imports
- Cycles
- Top files (by LOC)
- Health score

---

## Scan Behavior Options

Control how loctree scans and caches data.

### `--fresh`

Force rescan even if snapshot exists. Ignores mtime cache.

```bash
loct --fresh
loct auto --fresh
loct scan --fresh
```

**Use When:**
- Snapshot is stale
- Files changed without mtime update
- Testing after code changes

---

### `--no-scan`

Fail if no snapshot exists (don't auto-scan). Query-only mode.

```bash
loct slice src/main.rs --no-scan
loct health --no-scan
```

**Use When:**
- CI where scan happened in previous step
- You want to ensure you're using cached data
- Testing against existing snapshot

---

### `--fail-stale`

Fail if snapshot is stale (CI mode). Ensures snapshot is up-to-date.

```bash
loct health --fail-stale
loct lint --fail-stale --sarif
```

**Use When:**
- CI pipeline
- Ensuring fresh analysis
- Pre-commit hooks

**Exit Codes:**
- `0` - Snapshot is fresh
- `1` - Snapshot is stale

---

### `--full-scan`

Force full rescan ignoring mtime cache. Part of `auto` and `scan` commands.

```bash
loct auto --full-scan
loct scan --full-scan
```

**Difference from `--fresh`:**
- `--fresh`: Global option, forces rescan for any command
- `--full-scan`: Scan option, ignores mtime optimization

---

### `--scan-all`

Include normally-ignored directories (node_modules, target, .venv).

```bash
loct scan --scan-all
loct auto --scan-all
```

**Use When:**
- Analyzing vendored code
- Debugging missing imports
- Full codebase inspection

**Default Ignored:**
- `node_modules/`
- `target/`
- `.venv/`
- `dist/`
- `build/`
- `.git/`

---

## Filter Options

Options for filtering and suppressing output.

### `--no-duplicates`

Suppress duplicate export sections in CLI output. Reduces noise.

```bash
loct auto --no-duplicates
loct commands --no-duplicates
loct events --no-duplicates
loct lint --no-duplicates
```

**Affects:**
- `auto` command
- `commands` command
- `events` command
- `lint` command

---

### `--no-dynamic-imports`

Hide dynamic import sections in CLI output. Reduces noise.

```bash
loct auto --no-dynamic-imports
loct events --no-dynamic-imports
loct lint --no-dynamic-imports
```

**Affects:**
- `auto` command
- `events` command
- `lint` command

---

### `--py-root <PATH>`

Additional Python package root for imports. Helps resolve Python imports.

```bash
loct scan --py-root /path/to/python/packages
loct auto --py-root ./external-libs
```

**Use When:**
- Using external Python packages
- Custom Python package layout
- Monorepo with multiple Python roots

**Example:**
```bash
# Resolve imports from both src/ and external-libs/
loct scan --py-root ./external-libs
```

---

## Environment Variables

### `LOCT_COLOR`

Control color output. Overrides `--color` flag.

**Values:**
- `auto` - Auto-detect TTY support (default)
- `always` - Force color output
- `never` - Disable color output

**Example:**
```bash
export LOCT_COLOR=never
loct health                # No color output

LOCT_COLOR=always loct health | less -R  # Force color in pipe
```

---

### `LOCT_CACHE_DIR`

Override default cache directory. Default: `.loctree/`

**Example:**
```bash
export LOCT_CACHE_DIR=/tmp/loctree-cache
loct scan
```

---

### `LOCT_LOG_LEVEL`

Set logging verbosity. Useful for debugging.

**Values:**
- `error` - Only errors
- `warn` - Warnings and errors
- `info` - Informational messages (default)
- `debug` - Detailed debug output
- `trace` - Very detailed trace output

**Example:**
```bash
export LOCT_LOG_LEVEL=debug
loct scan --verbose
```

---

## Color Mode

Control color output with `--color` flag.

```bash
loct --color <MODE>
```

**Modes:**

| Mode | Description | Use Case |
|------|-------------|----------|
| `auto` | Auto-detect TTY support | Default, smart detection |
| `always` | Force color output | Piping to `less -R`, HTML logs |
| `never` | Disable color output | CI logs, plain text files |

**Examples:**
```bash
loct health --color auto      # Auto-detect (default)
loct health --color always    # Force color
loct health --color never     # No color
loct health --color always | less -R  # Color in less
```

**Auto-detection:**
- Checks if stdout is a TTY
- Checks `TERM` environment variable
- Checks `NO_COLOR` environment variable

---

## Examples

### CI Integration

```bash
# Fail if snapshot is stale, output SARIF
loct lint --fail-stale --sarif --fail > loctree.sarif

# JSON output for processing
loct health --json | jq '.summary.health_score'

# Ensure fresh scan, fail on issues
loct --fresh audit --json > audit.json
```

---

### AI Agent Integration

```bash
# Single-shot agent bundle
loct --agent-json > agent.json

# JSONL stream for incremental processing
loct --for-ai > context.jsonl

# Slice with JSON for AI consumption
loct slice src/main.rs --json | claude
```

---

### Development Workflow

```bash
# Quick health check
loct h                        # Alias for --summary

# Verbose scan with full rescan
loct scan --verbose --full-scan

# Quiet mode for scripts
loct dead --quiet --json > dead.json

# Watch mode for continuous analysis
loct scan --watch
```

---

### Debugging

```bash
# Enable debug logging
export LOCT_LOG_LEVEL=debug
loct scan --verbose

# Force color in pipe
loct health --color always | tee health.log

# Use custom cache directory
export LOCT_CACHE_DIR=/tmp/loctree
loct scan
```

---

### Python Projects

```bash
# Add custom Python root
loct scan --py-root ./external-libs

# Multiple Python roots (run separate scans)
loct scan --py-root ./lib1
loct scan --py-root ./lib2

# Include virtualenv in scan
loct scan --scan-all          # Scans .venv/ too
```

---

## Option Precedence

When options conflict, precedence is:

1. Command-line flags (highest)
2. Environment variables
3. Configuration files (if any)
4. Defaults (lowest)

**Example:**
```bash
# Environment variable
export LOCT_COLOR=never

# Command-line flag overrides
loct health --color always    # Uses 'always', not 'never'
```

---

## Best Practices

### For CI

```bash
# Use fail flags and JSON output
loct lint --fail-stale --fail --sarif > loctree.sarif
loct health --fail-stale --json > health.json

# Disable color in CI logs
loct --color never health
# Or set environment variable
export LOCT_COLOR=never
```

---

### For Local Development

```bash
# Use watch mode for continuous feedback
loct scan --watch

# Quick health checks
loct h                        # Alias for --summary

# Verbose for debugging
loct scan --verbose --full-scan
```

---

### For AI Integration

```bash
# Agent bundle for one-shot context
loct agent > agent.json

# JSONL stream for incremental processing
loct --for-ai > context.jsonl

# JSON output for specific commands
loct slice src/main.rs --json
loct dead --json
loct health --json
```

---

## Common Patterns

### Fresh Analysis

```bash
# Force fresh scan and analysis
loct --fresh health
loct --fresh audit
loct scan --full-scan
```

---

### Query Only (No Scan)

```bash
# Fail if no snapshot, don't auto-scan
loct slice src/main.rs --no-scan
loct health --no-scan
loct dead --no-scan
```

---

### Quiet Mode for Scripts

```bash
# Minimal output, JSON for parsing
loct dead --quiet --json > dead.json
loct health --quiet --json | jq '.summary'
```

---

### Verbose Debugging

```bash
# Detailed progress and logging
export LOCT_LOG_LEVEL=debug
loct scan --verbose --full-scan

# Trace-level logging
export LOCT_LOG_LEVEL=trace
loct auto --verbose
```

---

## Summary Table

| Option | Short | Type | Default | Commands |
|--------|-------|------|---------|----------|
| `--help` | `-h` | Flag | - | All |
| `--version` | - | Flag | - | All |
| `--json` | - | Flag | Off | Most |
| `--quiet` | - | Flag | Off | All |
| `--verbose` | - | Flag | Off | All |
| `--color <mode>` | - | Value | `auto` | All |
| `--fresh` | - | Flag | Off | All |
| `--no-scan` | - | Flag | Off | Query commands |
| `--fail-stale` | - | Flag | Off | All |
| `--findings` | - | Flag | Off | `auto` |
| `--summary` | - | Flag | Off | `auto` |
| `--for-ai` | - | Flag | Off | `auto` |
| `--agent-json` | - | Flag | Off | `auto` |
| `--py-root <path>` | - | Value | - | `scan`, `auto` |
| `--full-scan` | - | Flag | Off | `scan`, `auto` |
| `--scan-all` | - | Flag | Off | `scan`, `auto` |
| `--no-duplicates` | - | Flag | Off | `auto`, `commands`, `events`, `lint` |
| `--no-dynamic-imports` | - | Flag | Off | `auto`, `events`, `lint` |

---

Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team
Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>
