# MCP Server Integration

`loctree-mcp` exposes Loctree's structural analysis as MCP tools that AI agents
can call directly over stdio.

For the repo's default operating model, pair this with:

- [`PERCEPTION.md`](../../PERCEPTION.md)
- [`docs/perception/adr.md`](../perception/adr.md)

## What Ships Here

The public workspace ships one MCP binary:

```bash
loctree-mcp
```

It is project-agnostic:

- first use on a project auto-scans if needed
- snapshots are cached in memory
- every tool accepts an optional `project` parameter

## Install

Choose whichever channel matches your workflow:

```bash
# Cargo
cargo install --locked loctree-mcp

# Full public install path (CLI + MCP)
curl -fsSL https://loct.io/install.sh | sh

# From source
git clone https://github.com/Loctree/loctree-ast.git
cd loctree-ast
make install-mcp
```

## Configuration

Example Claude Desktop / Claude Code config:

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

If you use an external MCP multiplexer such as `rmcp-mux`, keep that config in
your client/runtime layer. It is not part of this workspace anymore.

## Available Tools

### `repo-view(project?: string)`

Use this first. It returns repo summary, health, top hubs, quick wins, and the
recommended next loctree calls.

### `slice(project?: string, file: string, consumers?: boolean)`

Returns a file plus its dependencies and consumers. Use before modifying code.

### `find(project?: string, name: string, limit?: number)`

Symbol search with regex support. Use before creating new functions, types, or components.

### `impact(project?: string, file: string)`

Shows direct and transitive consumers. Use before deleting or major refactors.

### `focus(project?: string, directory: string)`

Module-level deep dive: files, LOC, exports, internal edges, and external dependencies.

### `tree(project?: string, depth?: number, loc_threshold?: number)`

Directory structure with LOC counts.

### `follow(project?: string, scope: string, limit?: number, handler?: string)`

Drills into repo-view signals such as dead exports, cycles, twins, hotspots,
commands, events, and pipelines.

## Recommended Agent Flow

The default Loctree sequence stays:

```text
repo-view -> focus -> slice -> impact -> find -> follow
```

That keeps AI context grounded in structure instead of grep drift.

## Verification

Quick checks after install:

```bash
loctree-mcp --version
```

If your client supports MCP inspection, confirm the server exposes these tool
names exactly:

- `repo-view`
- `slice`
- `find`
- `impact`
- `focus`
- `tree`
- `follow`

## Notes

- The public OSS workspace does not ship a `for_ai()` MCP tool anymore. The
  current overview entrypoint is `repo-view`.
- The public OSS workspace does not ship `loct lsp`. Editor/LSP integrations are
  maintained outside this repo.
