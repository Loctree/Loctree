# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [0.2.9] - 2025-11-22

### Added
- Graph toolbar upgrades: fit, reset, graph-only fullscreen, dark mode toggle, and tooltips with full path + LOC; node size now scales with LOC and uses stable preset layout computed in Rust.
- Graph safety/perf guards: caps at 8k nodes / 12k edges, skips overflow with warnings, and prevents rendering when filters empty; legend/hints updated.

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
