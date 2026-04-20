# Loctree Architecture

Current architecture of the public `loctree-ast` workspace.

## Workspace Layout

```text
loctree-ast/
├── Cargo.toml            # workspace manifest
├── Makefile              # build, install, release, and smoke gates
├── loctree_rs/           # core library + CLI binaries
├── loctree-mcp/          # MCP server
├── rmcp-common/          # shared MCP/common utilities
├── reports/              # Leptos-based HTML reports
├── distribution/         # install and release channel contracts
├── docs/                 # user + developer docs
├── scripts/              # release/install helpers
└── tools/                # hooks and fixtures
```

## Crate Graph

```text
          loctree (loctree_rs)
             /           \
            /             \
 loctree-mcp           report-leptos
            \
             \
          rmcp-common
```

`reports/wasm` is a sub-crate under the report renderer for WASM assets.

## High-Risk Hubs

These files carry the largest blast radius in the live tree:

- `loctree_rs/src/types.rs`
- `loctree_rs/src/snapshot.rs`
- `reports/src/types.rs`

Before heavy edits, follow the repo discipline:

```text
repo-view -> focus -> slice -> impact -> find -> follow
```

## Distribution Shape

The monorepo is the source of truth for code, CI, and release choreography.

User-facing binary distribution is split outward into thin repos and taps:

- CLI release repo: `Loctree/loct`
- MCP release repo: `Loctree/loctree-mcp`
- CLI tap: `Loctree/homebrew-cli`
- MCP tap: `Loctree/homebrew-mcp`

Those release channels are orchestrated from `.github/workflows/publish.yml` and
`distribution/`.

## What Is Not In This Workspace

Older docs sometimes referred to directories that no longer live here. The
public workspace does **not** contain:

- `landing/`
- `rmcp-mux/`
- `rmcp-memex/`
- `loctree_memex/`

Editor/LSP surfaces live in the external `loctree-suite` project rather than
this workspace.
