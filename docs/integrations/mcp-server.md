# MCP Server Integration

## What is MCP?

**Model Context Protocol (MCP)** is Anthropic's open protocol for connecting AI assistants to external data sources and tools. Think of it as a standard API that lets Claude (and other AI assistants) interact with your codebase through well-defined tools.

loctree-mcp exposes loctree's analysis capabilities as MCP tools that Claude can invoke directly during conversations.

## Architecture

```
Claude Desktop/Code
    │
    ▼ (JSON-RPC over stdio)
loctree-mcp (MCP Server)
    │
    ├── Auto-scans project on first use
    ├── Caches snapshots in memory
    └── Provides tools: for_ai(), slice(), find(), etc.
```

**Key Features:**
- **Project-agnostic**: Each tool accepts a `project` parameter (defaults to current directory)
- **Auto-scan**: First use on a project creates snapshot automatically
- **Multi-project cache**: Snapshots kept in RAM for instant responses
- **Zero config**: Just start the server, no --project needed

## Installation

### Option 1: Install from crates.io (Recommended)

```bash
cargo install loctree-mcp
```

### Option 2: Install from source

```bash
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite
cargo install --path loctree-mcp
```

### Option 3: Install with MCP stack (all-in-one)

```bash
git clone https://github.com/Loctree/loctree-suite.git
cd loctree-suite
make mcp-install  # Installs loctree-mcp + rmcp-mux + rmcp_memex
```

## Configuration

### Standalone (Direct Connection)

Add to `~/.config/claude/claude_desktop_config.json` (Linux/macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

**For Claude Code**, add to `~/.claude.json`:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

### With rmcp-mux (Recommended for Multiple Servers)

If you're running multiple MCP servers, use [rmcp-mux](https://crates.io/crates/rmcp-mux) to manage them:

1. Install rmcp-mux:
```bash
cargo install rmcp-mux
```

2. Create `~/.rmcp_servers/config/mux.toml`:
```toml
[servers.loctree]
socket = "~/.rmcp_servers/sockets/loctree.sock"
cmd = "/Users/youruser/.cargo/bin/loctree-mcp"
args = []
auto_start = true
max_connections = 5
restart_policy = "on_failure"
```

3. Configure Claude to use the proxy:
```json
{
  "mcpServers": {
    "loctree": {
      "command": "rmcp-mux",
      "args": ["proxy", "--socket", "/Users/youruser/.rmcp_servers/sockets/loctree.sock"]
    }
  }
}
```

4. Start rmcp-mux:
```bash
rmcp-mux --config ~/.rmcp_servers/config/mux.toml
```

See [MCP Quick Start](../dev/.TL_DR/00_mcp_quickstart.md) for full setup instructions.

## Available Tools

### Core Tools (Use These First)

#### `for_ai(project?: string)`

Get AI-optimized project overview. Shows file count, LOC, health issues (dead code, cycles, twins), top hubs, quick wins.

**USE THIS FIRST** at the start of any AI session.

**Parameters:**
- `project` (optional): Project directory (default: current directory)

**Example:**
```typescript
// Claude will invoke:
for_ai({ project: "." })

// Response:
{
  "project": "/Users/me/my-app",
  "summary": {
    "files": 127,
    "total_loc": 15234,
    "edges": 389,
    "languages": ["TypeScript", "Rust"]
  },
  "health": {
    "dead_exports": { "total": 12, "high_confidence": 3 },
    "cycles": 2,
    "twins": 1
  },
  "top_hubs": [
    { "file": "src/api/index.ts", "importers": 23 }
  ],
  "quick_wins": [
    { "file": "src/utils/old.ts", "symbol": "formatDate" }
  ]
}
```

#### `slice(project?: string, file: string, consumers?: boolean)`

Get file context: the file + all its imports + all files that depend on it.

**USE THIS BEFORE** modifying any file. One call = complete understanding of a file's role.

**Parameters:**
- `project` (optional): Project directory
- `file` (required): File path relative to project root (e.g., 'src/App.tsx')
- `consumers` (optional): Include consumer files (default: true)

**Example:**
```typescript
slice({
  project: ".",
  file: "src/components/ChatPanel.tsx",
  consumers: true
})

// Response:
{
  "target": "src/components/ChatPanel.tsx",
  "core_loc": 150,
  "dependencies": 3,
  "consumers": 2,
  "files": [
    { "path": "src/components/ChatPanel.tsx", "layer": "core", "loc": 150 },
    { "path": "src/hooks/useAuth.ts", "layer": "dependency", "loc": 80 },
    { "path": "src/contexts/AuthContext.tsx", "layer": "dependency", "loc": 200 },
    { "path": "src/main.tsx", "layer": "consumer", "loc": 30 }
  ]
}
```

#### `find(project?: string, name: string, limit?: number)`

Find where a function/class/type is defined.

**USE THIS BEFORE** creating anything new - avoid duplicates. Shows all matches with file and line number.

**Parameters:**
- `project` (optional): Project directory
- `name` (required): Symbol name or regex pattern to search for
- `limit` (optional): Maximum results to return (default: 50)

**Example:**
```typescript
find({ project: ".", name: "useAuth", limit: 10 })

// Response:
{
  "query": "useAuth",
  "count": 2,
  "matches": [
    { "file": "src/hooks/useAuth.ts", "symbol": "useAuth", "kind": "function", "line": 12 },
    { "file": "src/hooks/index.ts", "symbol": "useAuth", "kind": "re-export", "line": 5 }
  ]
}
```

#### `impact(project?: string, file: string)`

What breaks if you change or delete this file? Shows direct and transitive consumers.

**USE THIS BEFORE** deleting or major refactor.

**Parameters:**
- `project` (optional): Project directory
- `file` (required): File path to analyze impact for

**Example:**
```typescript
impact({ project: ".", file: "src/utils/api.ts" })

// Response:
{
  "file": "src/utils/api.ts",
  "risk_level": "high",
  "direct_consumers": {
    "count": 12,
    "files": ["src/hooks/useAuth.ts", "src/hooks/useUser.ts", ...]
  },
  "transitive_consumers": {
    "count": 35,
    "files": ["src/components/App.tsx", ...]
  },
  "safe_to_delete": false
}
```

#### `health(project?: string)`

Quick health summary: cycles + dead code + twins.

**USE THIS** as sanity check before commits.

**Parameters:**
- `project` (optional): Project directory

**Example:**
```typescript
health({ project: "." })

// Response:
{
  "status": "minor_issues",
  "cycles": { "count": 2, "details": [...] },
  "dead_exports": { "total": 8, "high_confidence": 2 },
  "twins": { "count": 1, "examples": ["formatDate"] }
}
```

### Analysis Tools

#### `findings(project?: string)`

Get ALL codebase issues in one call: dead exports, circular imports, duplicate files (twins), barrel chaos, etc.

Use this for comprehensive health check or CI integration.

**Parameters:**
- `project` (optional): Project directory

#### `dead(project?: string, confidence?: string, limit?: number)`

Find unused exports (dead code). Shows exports that are defined but never imported anywhere.

**Parameters:**
- `project` (optional): Project directory
- `confidence` (optional): "normal" or "high" (default: normal)
- `limit` (optional): Maximum results (default: 20)

**Example:**
```typescript
dead({ project: ".", confidence: "high", limit: 10 })
```

#### `cycles(project?: string, classify?: boolean)`

Find circular import chains. These can cause runtime issues and make code hard to reason about.

**Parameters:**
- `project` (optional): Project directory
- `classify` (optional): Include classification (lazy, structural, etc.)

**Example:**
```typescript
cycles({ project: ".", classify: true })

// Response:
{
  "count": 2,
  "cycles": [
    {
      "classification": "Structural",
      "risk": "high",
      "pattern": "Direct circular dependency",
      "nodes": ["src/a.ts", "src/b.ts", "src/a.ts"],
      "suggestion": "Extract shared logic to a third file"
    }
  ]
}
```

#### `twins(project?: string, include_tests?: boolean, limit?: number)`

Find files with identical content (exact duplicates). These are candidates for refactoring into shared modules.

**Parameters:**
- `project` (optional): Project directory
- `include_tests` (optional): Include test files (default: false)
- `limit` (optional): Maximum results (default: 20)

#### `crowds(project?: string, limit?: number)`

Find 'crowd' patterns - files that are imported by many others (hubs) or import too much (god objects). These are refactoring hotspots.

**Parameters:**
- `project` (optional): Project directory
- `limit` (optional): Maximum results (default: 10)

#### `trace(project?: string, handler: string)`

Trace a Tauri command/handler from frontend invoke() to backend handler. Shows the complete pipeline: FE calls → BE definition → events.

**Essential for Tauri projects.**

**Parameters:**
- `project` (optional): Project directory
- `handler` (required): Handler/command name to trace (e.g., "get_user", "save_settings")

**Example:**
```typescript
trace({ project: ".", handler: "get_user" })

// Response:
{
  "handler": "get_user",
  "verdict": "connected",
  "has_handler": true,
  "is_called": true,
  "backend": { "file": "src-tauri/src/commands/user.rs", "line": 12 },
  "frontend_calls": [
    { "file": "src/hooks/useUser.ts", "line": 8 },
    { "file": "src/components/Profile.tsx", "line": 23 }
  ]
}
```

#### `duplicates(project?: string, limit?: number)`

Find symbols exported from multiple files. This can cause confusion about which import to use.

**Parameters:**
- `project` (optional): Project directory
- `limit` (optional): Maximum groups to return (default: 10)

### Structure Tools

#### `tree(project?: string, depth?: number, loc_threshold?: number)`

Get directory structure with LOC (lines of code) counts. Helps understand project layout and find large files/directories.

**Parameters:**
- `project` (optional): Project directory
- `depth` (optional): Maximum depth (default: 3)
- `loc_threshold` (optional): LOC threshold for highlighting (default: 500)

#### `query(project?: string, kind: string, target: string)`

Fast graph queries: who-imports (files importing target), where-symbol (where is symbol defined).

**Parameters:**
- `project` (optional): Project directory
- `kind` (required): Query kind: 'who-imports', 'where-symbol'
- `target` (required): Query target (file path or symbol name)

**Example:**
```typescript
query({ project: ".", kind: "who-imports", target: "src/utils/api.ts" })

// Response:
{
  "query": "who-imports",
  "target": "src/utils/api.ts",
  "count": 12,
  "importers": [
    { "file": "src/hooks/useAuth.ts", "line": 3 },
    { "file": "src/hooks/useUser.ts", "line": 5 }
  ]
}
```

### Administrative Tools

#### `scan(project?: string, force?: boolean)`

Scan a project directory and create/update snapshot.

Run this first on any new project, or after major changes. Creates `.loctree/` with all analysis artifacts.

**Parameters:**
- `project` (optional): Project directory
- `force` (optional): Force rescan even if snapshot exists (default: false)

**Note:** Most tools auto-scan if no snapshot exists, so you rarely need to call this manually.

## Usage Examples with Claude

### Example 1: Before Modifying a File

```
You: I need to refactor src/hooks/useAuth.ts

Claude: Let me check the context for that file first.
[Invokes: slice({ file: "src/hooks/useAuth.ts", consumers: true })]

Based on the slice, this file:
- Has 2 dependencies (AuthContext.tsx, api.ts)
- Is imported by 5 consumers
- Total blast radius: 8 files, 1,200 LOC

The main consumers are:
- src/components/Login.tsx
- src/components/Profile.tsx
- src/routes/ProtectedRoute.tsx

Any changes will affect these files. What specifically do you want to refactor?
```

### Example 2: Before Creating a New Component

```
You: I need to create a new DatePicker component

Claude: Let me check if something similar already exists.
[Invokes: find({ name: ".*[Dd]ate.*[Pp]icker.*" })]

Found 2 similar components:
- src/components/DateInput.tsx (line 15)
- src/ui/DateSelector.tsx (line 8)

Do you want to use one of these or create a new one?
```

### Example 3: Health Check Before Commit

```
You: Can you check the codebase health before I commit?

Claude: Running health check...
[Invokes: health({ project: "." })]

Health Status: minor_issues

Issues found:
- 2 circular imports (src/a.ts ↔ src/b.ts)
- 8 dead exports (2 high confidence)
- 1 duplicate file (formatDate implemented twice)

High-confidence dead code to remove:
- src/utils/old.ts:formatDate (line 12)
- src/helpers/unused.ts:processData (line 45)

Would you like me to help fix these?
```

### Example 4: Tauri Command Validation

```
You: Debug why my get_user command isn't working

Claude: Let me trace that command...
[Invokes: trace({ handler: "get_user" })]

Found the issue:
- Backend handler exists: src-tauri/src/commands/user.rs:12
- But it's NOT registered in generate_handler![] in main.rs

You need to add "get_user" to the generate_handler![] macro.
```

## Troubleshooting

### Server won't start

**Check the binary is installed:**
```bash
which loctree-mcp
# Should show: /Users/youruser/.cargo/bin/loctree-mcp
```

**Check logs:**
```bash
# Claude Desktop logs (macOS)
tail -f ~/Library/Logs/Claude/mcp*.log

# Claude Code logs
# Check the Output panel in Claude Code, select "MCP: loctree"
```

**Common issue**: Wrong path in config
```json
{
  "mcpServers": {
    "loctree": {
      "command": "/Users/youruser/.cargo/bin/loctree-mcp",  // Use full path
      "args": []
    }
  }
}
```

### Tools return "Project directory not found"

The `project` parameter must be:
- An absolute path, OR
- A relative path from Claude's current working directory

**Fix**: Use absolute paths or ensure Claude is in the right directory.

### "Failed to load snapshot" error

**Cause**: No `.loctree/snapshot.json` exists yet.

**Fix**: The server will auto-create it on first use. If it fails:
```bash
cd your-project
loct scan  # Create snapshot manually
```

### Snapshot is stale

**Symptom**: Tools return outdated data after git commits.

**Cause**: Snapshot is cached and linked to a git commit hash.

**Fix**: The server auto-detects this and rescans. To force rescan:
```typescript
scan({ project: ".", force: true })
```

### "loct scan failed" error

**Cause**: The `loct` CLI is not in PATH or the project is not supported.

**Check loct is installed:**
```bash
which loct
# Should show: /Users/youruser/.cargo/bin/loct
```

**Install loct:**
```bash
cargo install loctree
```

### High memory usage

**Cause**: Multiple large project snapshots cached in RAM.

**Fix**: Restart loctree-mcp to clear the cache:
```bash
# With rmcp-mux
rmcp-mux --restart-service loctree

# Standalone: restart Claude Desktop/Code
```

### Tools are slow

**First use**: The initial scan can take 10-60 seconds for large projects.

**Subsequent uses**: Should be instant (sub-second) thanks to caching.

**For very large projects** (>10,000 files):
- Consider using `--ignore` patterns in `.loctree/config.toml`
- Exclude build artifacts, node_modules, etc.

## Advanced Configuration

### Custom Log Level

Set via environment variable in your MCP config:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": ["--log-level", "debug"],
      "env": {}
    }
  }
}
```

Log levels: `trace`, `debug`, `info`, `warn`, `error`

### Multiple Project Instances

If you work on multiple projects simultaneously, loctree-mcp caches each project separately:

```typescript
// Project A
for_ai({ project: "/Users/me/project-a" })

// Project B
for_ai({ project: "/Users/me/project-b" })

// Both snapshots stay in memory for fast access
```

## Performance Notes

- **Scan time**: ~100-1000 files/sec depending on language and file complexity
- **Memory usage**: ~50-200 MB per cached project snapshot
- **Query time**: Sub-second for most operations after initial scan

**Benchmark results:**
- rust-lang/rust: 35,387 files in ~45s (787 files/sec)
- facebook/react: 3,951 files in ~49s (81 files/sec)
- golang/go: 17,182 files with ~0% false positives

## See Also

- [MCP Quick Start](../dev/.TL_DR/00_mcp_quickstart.md) - Full setup guide with rmcp-mux
- [loctree CLI Reference](../dev/03_cli_reference.md) - Standalone loctree usage
- [CI/CD Integration](./ci-cd.md) - Use loctree in GitHub Actions

---

Created by M&K (c)2025 The LibraxisAI Team
Co-Authored-By: Maciej <void@div0.space> & Klaudiusz <the1st@whoai.am>
