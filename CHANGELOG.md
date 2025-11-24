# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [0.3.8] - 2025-11-22

### Added
- Report UI reorganized into tabs (Overview / Duplicates / Dynamic imports / Tauri coverage / Graph anchor) with a dedicated bottom drawer for the graph and controls.
- Help text split per mode (Tree / Analyzer / Common) and expanded examples; graph/drawer behavior documented.
- Python analyzer refinements: `--py-root` (repeatable) for extra roots, `resolutionKind` + `isTypeChecking` on imports, dynamic import tagging, `__all__` expansion for star imports.

### Fixed
- Dark-mode toggle in the graph drawer no longer panics when Cytoscape style is not ready.
- Resolved stray brace/formatting issues in CLI help output.

## [0.3.5] - 2025-11-24

### Added
- TS/JS resolver now honors `tsconfig.json` (`baseUrl` + `paths` with `*` patterns) for imports and re-exports, improving FE↔BE linkage and graph accuracy when aliasing is heavy.

### Changed
- Graph/import resolution for non-relative specs prefers tsconfig aliases before falling back to relative heuristics; reduces “unresolved” noise in JSON/HTML/CLI reports.

## [0.3.4] - 2025-11-24

### Added
- FE↔BE coverage view now captures generic `invoke`/`safeInvoke` call sites and renamed Tauri handlers; surfaced in `aiViews.coverage`.
- `aiViews.tsconfig` summarizes `baseUrl`/aliases and highlights unresolved aliases plus `include`/`exclude` drift.
- Public-surface exports (barrels/index/mod.rs) are flagged in `symbols`/`clusters`/`deadSymbols` to prioritize cleanup.

### Changed
- Patch release bump for the above analyzer JSON improvements; no CLI-breaking changes.

## [0.3.3] - 2025-11-24

### Added
- JSON schema metadata (`schema`, `schemaVersion`, `generatedAt`, `rootDir`, `languages`) plus deterministic ordering for easier machine use.
- Richer per-file records: stable `id`, `language`, `kind` (code/test/story/config/generated), `isTest`, `isGenerated`, import symbol lists with `resolvedPath`, export `exportType` + `line`.
- Derived AI views in JSON: `commands2` (canonical handler + call-sites + status), `symbols`/`clusters`, and `aiViews` (default export chains, suspicious barrels, dead symbols, CI summary, coverage stats with renamed handlers + generic call sites, tsconfig summary with aliases/include|exclude drift).
- `--verbose` flag and auto-creation of parent directories for `--html-report` (matching `--json-out`).

### Changed
- JSON output remains backward-compatible while exposing the new fields for agents/LLMs; dynamic imports, duplicate metadata, and commands are now sorted deterministically.

## [0.3.2] - 2025-11-23

### Added
- Component graph metadata in reports (component id/size, isolates, LOC sum, Tauri FE/BE counts) with UI controls for highlighting disconnected components.
- Import graph data builder extracted to `graph.rs` with safer node/edge caps and deterministic layout; HTML graph bootstrap served from a dedicated asset.
- AI insights collected from analyzer output (dup/export cascades, missing handlers, huge files) and shown in reports.

### Changed
- Analyzer runner split into focused modules (`graph.rs`, `coverage.rs`, `insights.rs`, `graph_bootstrap.js`), shrinking `runner.rs` and `html.rs`.
- Tauri command matching now normalizes names via `heck::ToSnakeCase` and respects focus/exclude globs.
- Generic invoke regexes hardened to handle type parameters without excessive backtracking risk.

## [0.3.1] - 2025-11-22

### Added
- Tauri command coverage view (missing vs unused handlers) grouped by module with linkable locations.
- Import graph drawer and safety limits; buttons for fit/reset/fullscreen/dark-mode and JSON/PNG export.
- Self-hosted Cytoscape asset for CSP/offline friendliness.

### Changed
- Duplicate export filtering honors `--focus` / `--exclude-report` globs; canonical picks non-dev files first.
- `--serve` links url-encoded and open-server startup made more robust.

## [0.3.0] - 2025-11-22

### Added
- **Import Graph Drawer**: When analyzing a single root, the graph is pinned to a collapsible bottom drawer, keeping tables readable.
- **Easier-to-hit Tooltips**: Nodes now have a larger hitbox, and tooltips appear near the cursor within the viewport boundaries.

### Changed
- The import graph is now attached to a collapsible drawer when analyzing a single root to improve table visibility.

## [0.2.9] - 2025-11-22

### Added
- Graph toolbar upgrades: fit, reset, graph-only fullscreen, dark mode toggle, and tooltips with full path + LOC; node size now scales with LOC and uses stable preset layout computed in Rust.
- Graph safety/perf guards: caps at 8k nodes / 12k edges, skips overflow with warnings, and prevents rendering when filters empty; legend/hints updated.
- Graph assets self-hosted (CSP-friendly) + buttons to export PNG/JSON snapshots.
- Tauri coverage: FE↔BE matching normalizes camelCase/snake_case aliases; coverage respects `--focus/--exclude-report` globs and groups rows by module for readability.

### Changed
- Cleaner import graph (edge labels removed, deduped CSS, more defensive `buildElements`/filter handling).
- Tauri command coverage table restyled for readability (pill rows, clearer columns).
- FE↔BE Tauri matching now normalizes camelCase/snake_case aliases (e.g., `loginWithPin` ↔ `login_with_pin`) to trim false missing/unused reports.

## [0.2.8] - 2025-11-22

### Added
- `--focus <glob>` filters the report to show only duplicates where at least one file matches the glob patterns (analysis still covers the entire tree).
- `--exclude-report <glob>` allows filtering out noise (e.g., `**/__tests__/**`, `**/*.stories.tsx`) only from the duplicate report.

### Changed
- The number of duplicates in CLI/JSON/HTML reflects the above filters; canonical file and score are calculated after filtering paths.

## [0.2.7] - 2025-11-22

### Added
- `--graph` optionally appends an interactive import/re-export graph to the HTML report (Cytoscape.js from CDN).
- `--ignore-symbols-preset <name>` (currently `common` → `main,run,setup,test_*`) and support for `foo*` prefixes in `--ignore-symbols`.

### Changed
- Help/README/Monika guide updated with new flags; duplicate analysis now considers prefix patterns.

## [0.2.6] - 2025-11-22

### Added
- The `--ignore-symbols` flag for the analyzer – allows omitting specified symbols (e.g., `main,run`) when detecting duplicate exports.

### Changed
- Documentation and help updated with the new flag.

## [0.2.5] - 2025-11-22

### Added
- The import/export analyzer now covers Python: `import`/`from`/`__all__`, detects dynamic `importlib.import_module` and `__import__`, and reports re-exports via `from x import *`.
- Default analyzer extensions now include `py`.

### Changed
- README and Monika's guide updated with Python support.

## [0.2.4] - 2025-11-22

### Added
- Optional `--serve` mini HTTP server: HTML reports contain clickable `file:line` links that open in an editor/OS (`code -g` by default, configurable with `--editor-cmd`). Safe: paths are canonicalized and restricted to provided roots.
- Reports and JSON now include locations of Tauri command calls/handlers, which speeds up FE↔BE diagnosis.

### Changed
- `--serve`/`--editor-cmd` described in help/README; auto-opening the report in the browser remains.

## [0.2.3] - 2025-11-22

### Added
- The analyzer reports Tauri command coverage: FE calls (`safeInvoke`/`invokeSnake`) vs. handlers with `#[tauri::command]` in Rust; also shows missing and unused handlers in HTML/JSON/CLI reports.

### Changed
- Hardening auto-open HTML (path canonicalization, no control character checks).
- Unified dependencies: `regex = 1.12` in manifest.
- Hidden files recognized solely by a leading dot (no special-case `.DS_Store`).

## [0.2.2] - 2025-11-22

### Added
- The analyzer now understands CSS `@import` and Rust `use`/`pub use`/public items; default analyzer extensions expanded to include `rs` and `css`.
- HTML report auto-open remains; help/README updated to note new language coverage.

### Changed
- Hidden-file detection no longer special-cases `.DS_Store`; relies on leading dot + `--show-hidden`.

## [0.2.0] - 2025-11-21

### Added
- Unified CLI features and JSON output across all runtimes (Node.js, Python, Rust): extension filters, ignore patterns, gitignore support, max depth, color modes, JSON output, and summary reporting (commit [`8962e39`](https://github.com/LibraxisAI/loctree/commit/8962e39)).
- Installation scripts for fast setup: `install.sh`, `install_node.sh`, and `install_py.sh` (commit [`b6824f4`](https://github.com/LibraxisAI/loctree/commit/b6824f4)).
- `--show-hidden` (`-H`) option to include dotfiles and other hidden entries in output in Rust and Python CLIs (commit [`12310b4`](https://github.com/LibraxisAI/loctree/commit/12310b4)).

### Changed
- Standardized the project name from `loc-tree` to `loctree` across runtimes, binaries, installers, and documentation; improved CLI UX and argument parsing, and enhanced error messages (commit [`e31d3a4`](https://github.com/LibraxisAI/loctree/commit/e31d3a4)).
- Usage/help output refined and examples clarified across Rust, Node, and Python CLIs (commit [`b6824f4`](https://github.com/LibraxisAI/loctree/commit/b6824f4) and [`8962e39`](https://github.com/LibraxisAI/loctree/commit/8962e39)).

### Documentation
- Expanded and clarified README with installation instructions, usage details, examples, and project structure overview (commits [`e31d3a4`](https://github.com/LibraxisAI/loctree/commit/e31d3a4), [`b6824f4`](https://github.com/LibraxisAI/loctree/commit/b6824f4), [`8962e39`](https://github.com/LibraxisAI/loctree/commit/8962e39)).

### Other
- Initial project setup (commit [`2031f80`](https://github.com/LibraxisAI/loctree/commit/2031f80)).

---

Release notes are generated from the last 5 commits on the default branch (`main`).
## [0.3.6] - 2025-11-25

### Added
- Python analyzer: TYPE_CHECKING-aware imports (`isTypeChecking`), dynamic import tagging (`importlib.import_module`, `__import__`), `__all__` expansion for star imports, and stdlib vs local disambiguation (`resolutionKind`).
- New flag `--py-root <path>` (repeatable) to add extra Python package roots for resolution.

### Changed
- JSON schema bumped to `1.2.0`; per-import records now include `resolutionKind` and `isTypeChecking`. Fixtures count as dev noise in duplicate scoring.
