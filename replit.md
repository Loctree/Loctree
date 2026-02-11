# Overview

Loctree is a static analysis tool designed for AI agents and developers to understand code structure, detect issues, and provide intelligent context for code generation. It scans codebases (TypeScript, Rust, Python, JavaScript) to build dependency graphs, detect dead code, circular imports, and other architectural issues. The tool outputs JSON snapshots, HTML reports, and SARIF files for both human and AI consumption.

The project consists of:
- **loctree_rs**: Core Rust CLI analyzer that performs static analysis
- **reports**: Leptos-based (WASM) HTML report generator
- **landing**: Marketing website built with Leptos CSR
- **rmcp_memex**: MCP-based RAG server for code memory/search (separate but related)
- **rmcp_mux**: MCP multiplexer library and daemon (separate but related)

# User Preferences

Preferred communication style: Simple, everyday language.

# System Architecture

## Core Analysis Engine (loctree_rs)

**Language**: Rust
**Parser**: OxC for JavaScript/TypeScript AST parsing
**Output Formats**: JSON snapshot, SARIF, HTML reports

### Key Design Decisions

1. **Snapshot-Based Architecture**: "Scan once, slice many" - initial scan writes cached artifacts (snapshot/findings/agent/manifest) into the OS cache dir by default (override via `LOCT_CACHE_DIR`). Subsequent queries (slice, find, dead, cycles) operate on this cached snapshot for speed.

2. **Multi-Language Support**: Dedicated analyzers per language (TypeScript/JavaScript via OxC, Python via custom parser, Rust via syn). Each analyzer extracts imports, exports, symbols, and metadata.

3. **Alias-Aware Resolution**: Supports TypeScript path aliases (`@core/*`), Windows case-insensitive paths, and dynamic imports. Uses `tsconfig.json` parsing to resolve module paths correctly.

4. **Atomic Writes**: All artifact generation (snapshot, reports, SARIF) uses atomic write patterns (`write_atomic`) to prevent corruption on crashes.

5. **Graph-Based Analysis**: 
   - Strongly connected components (SCC) for circular import detection
   - Reachability analysis for dead export detection
   - Jaccard similarity for duplicate/crowd detection

## Report Generation (reports/)

**Framework**: Leptos (Rust → WASM)
**Rendering**: Server-side rendering (SSR) to static HTML, no client-side hydration required
**Visualization**: Cytoscape.js for interactive dependency graphs

### Design Rationale

- **Zero JavaScript Runtime**: Reports are pure SSR HTML with embedded Cytoscape for graphs. This ensures reports are viewable without build tooling.
- **Component Architecture**: Modular UI components (`FileTable`, `GraphView`, `QuickWinPanel`) for maintainability.
- **Dark Mode**: CSS variable-based theming with graph-specific dark mode toggle.

## Landing Page (landing/)

**Framework**: Leptos CSR (client-side rendering)
**Build Tool**: Trunk
**Styling**: Monochromatic "CRT terminal" aesthetic with CSS modules

## Tauri Integration

Loctree provides specialized analysis for Tauri applications:
- **Command Coverage**: Detects frontend `invoke()` calls without backend handlers and vice versa
- **Event Analysis**: Tracks `emit()`/`listen()` pairs, detecting orphaned listeners and race conditions

## CLI Design

```bash
loct                    # Default: scan + save snapshot
loct slice <file>       # Extract context slice (3-layer: core, deps, consumers)
loct find <pattern>     # Fuzzy search across files and symbols
loct dead               # Unused export detection
loct cycles             # Circular import detection
loct commands           # Tauri FE↔BE coverage
loct report --serve     # Launch HTML report server
```

### ID Rewriting Strategy

- CLI binary is `loct` (short form)
- Package name is `loctree` on crates.io
- Library name is `loctree_rs` (Rust identifier)

## Testing Strategy

1. **Integration Tests**: End-to-end CLI tests using `assert_cmd` crate
2. **Fixture-Based Tests**: Test fixtures in `tests/fixtures/` for circular imports, dead exports, etc.
3. **Unit Tests**: Per-analyzer unit tests for language-specific parsing

## Error Handling

- **Production Code**: No `unwrap()` in production paths; uses `anyhow::Result` or `thiserror`
- **Mutex Poison Recovery**: Non-panicking recovery in `root_scan` shared state
- **Graceful Degradation**: Analysis failures log warnings rather than crashing entire scan

## Performance Optimizations

- **Incremental Scanning**: mtime-based change detection to skip unchanged files
- **Parallel Processing**: Tokio async for concurrent file analysis
- **Snapshot Reuse**: Same commit/branch skips rewrites; provides hint when worktree is dirty

# External Dependencies

## Core Analysis Dependencies

- **oxc_parser** / **oxc_ast**: JavaScript/TypeScript AST parsing (chosen for speed and spec compliance)
- **syn**: Rust code parsing
- **serde** / **serde_json**: Serialization for JSON snapshots and SARIF output
- **tokio**: Async runtime for parallel file scanning
- **anyhow**: Error handling
- **clap**: CLI argument parsing

## Report Generation Dependencies

- **leptos**: SSR framework for HTML report generation (v0.6+)
- **cytoscape.js**: Graph visualization (loaded from CDN in reports)
- **dagre** / **cose-bilkent**: Graph layout algorithms for Cytoscape

## Python Analysis

- Custom Python parser (not using external AST library)
- Planned improvements for `TYPE_CHECKING` blocks, `importlib` dynamic imports, and `__all__` expansion

## Related Projects

- **rmcp_memex**: MCP server providing RAG/memory storage using LanceDB + FastEmbed
- **rmcp_mux**: MCP multiplexer library for sharing MCP servers across multiple clients via Unix socket

## CI/CD

- GitHub Actions for fmt, clippy, tests, Semgrep security scanning
- Pre-push hooks enforce quality gates (clippy, tests, formatting)
- Tarpaulin for code coverage tracking

## Configuration Files

- **tsconfig.json**: TypeScript path alias resolution
- **pyproject.toml**: Python package root detection
- **.gitignore**: Respected by default for excluding files from analysis
- **replit.nix** / **.replit**: Replit environment configuration (minimal, not actively maintained)
