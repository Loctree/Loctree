# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Released]

## [0.5.18] - 2025-12-06

### 🎯 Major: Twins Analysis (Semantic Duplicate Detection)

New `loct twins` command for semantic duplicate analysis — a comprehensive tool for detecting code organization issues inspired by Monty Python's Dead Parrot sketch.

### Added

**Dead Parrots Detection**
- Finds exports with 0 imports across the entire codebase
- Filters out Tauri registered handlers (not false positives)
- Filters out locally used symbols within the same file
- Smart detection reduces false positives by ~75% compared to naive analysis

**Exact Twins Detection**
- Identifies same symbol name exported from multiple files
- Highlights potential naming conflicts and duplicate implementations
- Groups twins by symbol name for easy review

**Barrel Chaos Analysis**
- **Missing barrels**: Directories with multiple files imported externally but no `index.ts`
- **Deep re-export chains**: Detects `index.ts → sub/index.ts → sub/sub/index.ts` (depth > 2)
- **Inconsistent import paths**: Same symbol imported via different paths

### Usage

```bash
loct twins              # Full analysis: dead parrots + exact twins + barrel chaos
loct twins --dead-only  # Only exports with 0 imports
loct twins --path src/  # Analyze specific path
```

### Technical Details
- 638 tests passing (added 12 new tests for twins detection)
- ~800 lines of new analyzer code in `twins.rs` and `barrels.rs`
- Zero breaking changes to public API

---

## [0.5.17] - 2025-12-06

### 🎯 Major: False Positive Massacre

This release dramatically reduces false positives in dead export detection across all major frameworks. Based on smoke tests against 11 real-world repositories (loctree-dev, SvelteKit, FastAPI, Vue Core, GitButler, etc.), we identified and fixed 6 critical FP sources.

### Added

**Rust Same-File Usage Detection** (Agent 1)
- Detects types used in struct/enum field definitions within the same file
- Handles generic parameters: `Vec<T>`, `Option<T>`, `HashMap<K,V>`, `Result<T,E>`
- Parses tuple structs, enum variants with data, and associated types
- **NEW**: Detects const usage in generics: `fn foo::<BUFFER_SIZE>()`, `create_buffer::<SIZE, _>()`
- Fixes 100% FP rate in Rust projects where types were only used as field types

**Rust Crate-Internal Import Resolution** (Agent 7, 8, 9)
- Resolves `use crate::foo::Bar` imports to actual file paths
- Handles `use super::Bar` and `use self::foo::Bar` relative imports
- Supports nested brace imports: `use crate::{foo::{A, B}, bar::C}`
- Fuzzy symbol matching for complex multi-line imports
- `CrateModuleMap` for module path → file path resolution
- Fixes MENU_GAP-style false positives in Zed and similar large Rust codebases

**SvelteKit Virtual Module Resolution** (Agent 2)
- Recognizes SvelteKit virtual modules: `$app/*`, `$lib/*`, `$env/*`, `$service-worker`
- Parses `vite.config.js/ts` for custom path aliases
- Resolves tsconfig `paths` with wildcard patterns
- Virtual modules now resolve to `__virtual__/$app/forms` style paths
- Fixes 83% FP rate in SvelteKit projects

**Python FastAPI Decorator Tracking** (Agent 3)
- Extracts type references from decorator parameters:
  - `response_model=User` → marks `User` as used
  - `Depends(get_db)` → marks `get_db` as used
  - `List[Schema]`, `Optional[Model]` → extracts inner types
- Recognizes FastAPI/Pydantic factories: `Query`, `Body`, `Path`, `Header`, `Cookie`, `Form`, `File`
- Fixes 100% FP rate in FastAPI projects

**Svelte Template Function Call Detection** (Agent 4)
- Parses Svelte template expressions: `{formatDate(value)}`
- Detects event handlers: `on:click={handleClick}`, `on:submit|preventDefault={submit}`
- Recognizes bind directives: `bind:value={store}`, `bind:this={element}`
- Handles transitions and actions: `transition:fade`, `use:tooltip`
- Extracts component usage: `<MyComponent />`, `<svelte:component this={comp}/>`
- Fixes 40-50% FP rate in Svelte projects (GitButler-level)

**Generated Code Detection** (Agent 5)
- Path-based detection: `**/generated/**`, `**/*.generated.*`, `**/*.g.dart`
- Content-based markers: `@generated`, `DO NOT EDIT`, `auto-generated`, `THIS FILE IS GENERATED`
- Skips generated files in dead export analysis
- Integrated into `FileAnalysis.is_generated` flag

**Vue SFC Script Parsing** (Agent 6)
- Extracts `<script>` and `<script setup>` blocks from `.vue` files
- Supports both Composition API and Options API
- Routes extracted scripts through standard JS/TS analyzer
- Fixes 86% FP rate in Vue projects

### Changed
- `FileAnalysis` now includes `is_generated` flag from both path and content analysis
- Virtual module resolution integrated into standard import resolution pipeline
- Svelte files now analyzed in two passes: script block + template expressions

### Fixed
- Clippy warnings in `resolvers.rs` (collapsible if/else, regex in loop)

### Technical Details
- 667 tests passing (added 27 new tests for new features)
- ~800 lines of new analyzer code across 6 modules
- Zero breaking changes to public API

---

## [0.5.16] - 2025-12-05

### Added
- Progress UI for long-running symbol searches
- Improved `find_symbol` with regex pattern matching and path filters

### Changed
- Crowd match reasons refactored for clearer similarity explanations
- Report UI polish for crowd detection display

---

## [0.5.15] - 2025-12-04

### Added
- **Crowd Detection**: Identifies functional duplicate files using:
  - Structural similarity (import/export patterns)
  - Content fingerprinting
  - Clustering with configurable thresholds
- Router-based subpages in landing for better navigation
- Dead code analysis integrated into HTML reports

### Changed
- Test file handling improved for Python and TypeScript fixtures

---

## [0.5.14] - 2025-12-03

### Added
- Initial crowd detection module with similarity scoring
- Logo assets updated with thicker stem and adjusted positions

### Changed
- Branding assets refresh

---

## [0.5.13] - 2025-12-09

### Added
- Circular-import quick wins now use real cycle data from `cycles::find_cycles`, surfaced in QuickWin output and tests.
- Atomic snapshot writes (via `write_atomic`) for report/SARIF/analysis/dead/handlers/circular/races artifacts to avoid partial files and corruption.
- Alias-aware dynamic import reachability (`@core/*`, Windows case-insensitive) with new fixtures/tests.

### Changed
- Unified CLI help paths: `--help-full` is handled consistently in both binaries; `search` hints now use `loctree …` wording everywhere.
- Install/docs/CI instructions standardized on `cargo install loctree`; removed `curl | sh` mentions.
- Tooltip layer helper (`.tooltip-floating` z-index 9999) and scrollbar CSS now ship with fallbacks.
- Landing/AI README/changelog bumped to 0.5.12+ alignment for release metadata.

### Fixed
- Removed panics in QuickWin/SARIF/snapshot paths; errors now bubble as `Result` or log warnings instead of crashing.
- Mutex poison recovery in `root_scan` avoids thread panics; dead export matching handles Windows casing correctly.
- Ignored and removed generated `**/.loctree/**/report.html` fixture artifacts from the repo.

## [0.5.12] - 2025-12-08

### Added
- Atomic writes for snapshot artifacts (report/SARIF/JSON) to prevent partial files on crash or interrupt.
- Alias-aware dynamic import reachability (handles `@core/*` prefixes and Windows casing) with new tests.

### Changed
- Unified install docs and prompts to `cargo install loctree`.
- Quick-win JSONL and SARIF generation now return/log errors instead of panicking.
- Tooltip layer helper to avoid z-index clashes in reports.

## [0.5.10] - 2025-12-03

### Added
- Snapshot artifacts now live under `.loctree/<branch@commit>/` and `save()` skips rewrites for the same commit/branch (with a hint when the worktree is dirty).
- Base scans print a concise human summary (files, handlers, languages, elapsed); `--serve` binds 0.0.0.0:5075 with loopback/random fallback and warns about the upcoming `loct report --serve` migration.

### Fixed
- Python analyzer: dead-export FP reduction (imported symbols, mixin inheritance, callbacks), line numbers in dead output, faster scan path; `who-imports` query now reports Python imports correctly.
- Report WASM: removed deprecated-init console warning from `report_wasm.js`.
- HTML/auto-artifacts no longer auto-open during tests/builds; analysis artifacts key off parsed output mode (JSON/SARIF now written reliably).

### Changed
- HTML reports are generated only when explicitly requested (`loct report --serve` or `--report`); global `--serve` remains as a backwards-compatible alias with a warning.
- Snapshot writing warns and reuses the existing snapshot when nothing changed for the current commit/branch.

## [0.5.7] - 2025-12-01

### Added
- **One-shot artifact bundle**: Bare `loct`/`loctree` now saves the full analyzer output to `.loctree/` alongside `snapshot.json` — `report.html` (with graph), `analysis.json`, `circular.json`, and `py_races.json`, so you don't need to run extra commands after a scan.

### Changed
- **Rebrand alignment**: Updated repository/org references to `Loctree/Loctree` and refreshed version strings to v0.5.7 across crates and docs.
- **Release hygiene**: Rust formatting/clippy cleanups applied for the 0.5.7 publish pipeline.

## [0.5.6] - 2025-12-01

### Fixed
- **AST Parser JSX Fix**: Disabled JSX parsing for `.ts` files (only enabled for `.tsx`/`.jsx`). Previously, TypeScript generics like `<T>` were incorrectly parsed as JSX tags, causing entire files like `api.ts` to fail parsing.
- **Template Literal Support**: Added detection of Tauri `invoke` calls using backticks (`` `cmd` ``). Commands like `` safeInvoke(`create_user`) `` are now correctly identified.
- **False Positive Reduction**: Added exclusion lists to prevent non-Tauri functions from being detected as commands:
  - `NON_INVOKE_EXCLUSIONS`: ~35 patterns like `useVoiceCommands`, `runGitCommand`, `executeCommand`
  - `INVALID_COMMAND_NAMES`: CLI tools like `node`, `cargo`, `pnpm`, `git`
- **Payload Requirement**: `CommandRef` is now only created when a valid command name payload exists, eliminating false positives where function names were mistaken for commands.

### Added
- **Git Context in Reports**: Added `git_branch` and `git_commit` fields to `ReportSection` for future Scan ID system integration.
- **Parser Debug Logging**: Added error logging when OXC parser encounters issues (visible with `--verbose`).

### Changed
- **Vista Project Results**: Improved detection accuracy:
  - Frontend commands: 170 → 254 (+49%)
  - Missing handlers: 18 → 5 (72% reduction in false positives)
  - Unused handlers: 137 → 57 (58% reduction in false positives)

## [0.5.5] - 2025-11-30

### Fixed
- **AI Context Safety**: Limited verbosity of `slice` and `circular` commands to prevent context flooding in LLMs:
  - `slice`: Truncates Deps/Consumers lists > 25 items (showing "... and N more").
  - `circular`: Compresses dependency cycles longer than 12 steps into `head -> ... (N intermediate) ... -> tail` format.

## [0.5.4] - 2025-11-30

### Added
- **Loctree CI workflow**: Separate GitHub Actions workflow that runs loctree self-analysis on all inner crates (loctree_rs, reports, landing) with HTML report artifacts.
- **Version sync script**: `scripts/sync-version.sh` automatically synchronizes version across all crates and hardcoded strings during releases.
- **`loct` CLI alias**: Short alias for `loctree` command for faster typing.

### Changed
- **Binary structure refactored**: Moved CLI entry points from `src/main.rs` to `src/bin/loctree.rs` and `src/bin/loct.rs` to eliminate "multiple build targets" warning.
- **CI matrix**: Loctree CI now runs on both Ubuntu and macOS.

### Fixed
- **Version sync**: All version references (reports footer, lib.rs doc URL, landing easter eggs) now properly synced to 0.5.4.

## [0.5.3] - 2025-11-29

### Added
- **COSE-Bilkent graph layout**: Added force-directed layout algorithm for better dependency graph visualization in HTML reports.
- **`report-leptos` library crate**: Extracted HTML report generation into a standalone crate (v0.1.1) for reuse and cleaner architecture.

### Changed
- **Report UI redesign**: New dark/light theme with improved visual hierarchy and accessibility.
- **Shared JS assets**: Moved graph visualization libraries (Cytoscape, Dagre, COSE-Bilkent) to the library crate.

### Fixed
- **Nested conditions refactored**: Improved `root_scan` and `detect` modules using Rust 2024 if-let chains.

## [0.5.2] - 2025-11-28

### Changed
- Updated Semgrep policy configuration to the latest defaults.

### Fixed
- Synced version references (landing assets and metadata) with the v0.5.2 release to resolve develop/main divergence.

## [0.5.1] - 2025-11-28

### Added
- **Entry point detection**: Proper regex-based detection for Python and Rust entry points:
  - Python: `__main__.py` files and `if __name__ == "__main__":` blocks
  - Rust: `fn main(` and async runtime attributes (`#[tokio::main]`, `#[async_std::main]`)
  - Uses regex with line-start anchors to avoid false positives in comments/strings
- **Lazy import detection**: React.lazy() patterns now properly tracked as dynamic imports:
  - Detects `import('./Foo').then(m => ({ default: m.Bar }))` syntax
  - Prevents false positives for lazy-loaded components in dead export detection

### Changed
- **Python stack detection**: Extended default ignores for Python projects:
  - Added: `packaging`, `logs`, `.fastembed_cache`, `.cache`, `.uv`
  - Covers common ML/data caches and uv package manager artifacts
- **Git hooks restructured**:
  - `pre-commit`: Fast checks on staged files only (fmt auto-fix, cargo check, unit tests)
  - `pre-push`: Comprehensive validation (clippy -D warnings, full tests, integration tests, dogfooding, semgrep)

### Fixed
- **Slice file matching**: Now prioritizes exact path match over `ends_with` match; warns when multiple files match the same target to avoid selecting wrong file (e.g., `backend.py` picking monorepo's root instead of src).
- **Tauri generate_handler! parsing**: Fixed extraction of function names from module-qualified paths (e.g., `commands::foo::bar` now correctly registers `bar` instead of `commands`). Also handles `#[cfg(...)]` attributes inside the macro block without breaking the parser.

## [0.5.0-rc] - 2025-11-28

### Added
- **Holographic Slice** (`slice` command): Extract 3-layer context for AI agents from any file:
  - **Core**: Target file itself (full content)
  - **Deps**: Files imported by target (BFS traversal up to depth 2)
  - **Consumers**: Files that import target (with `--consumers` flag)
  - JSON output for piping directly to AI: `loctree slice src/App.tsx --json | claude`
- **Auto-detect stack**: Automatically detects project type from:
  - `Cargo.toml` → Rust (adds `target/` to ignores)
  - `tsconfig.json` / `vite.config.*` → TypeScript (adds `node_modules/` to ignores)
  - `pyproject.toml` → Python (adds `.venv/`, `__pycache__/` to ignores)
  - `src-tauri/` → Tauri hybrid (sets `--preset-tauri` automatically)
- **Incremental scanning**: Uses file mtime to skip unchanged files. Typical re-scans now show "32 cached, 1 fresh" instead of re-parsing everything.
- **`--full-scan` flag**: Forces re-analysis of all files, bypassing mtime cache.
- **`--consumers` flag**: Include consumer layer in slice output.
- Wired existing modules to CLI:
  - `--circular`: Find circular imports using SCC algorithm
  - `--entrypoints`: List entry points (main, __main__, index)
  - `--sarif`: SARIF 2.1.0 output for CI integration

### Changed
- Rebranded as "AI-oriented Project Analyzer" to reflect the primary use case.
- Help text completely rewritten with slice examples: `loctree slice src/main.rs --consumers`
- Snapshot now stores file mtime for incremental scanning.
- Snapshot edges are always collected (previously only with `--graph`).

### Fixed
- Slice now correctly matches files when edges store paths without extensions.
- Removed unused `SliceConfig` fields (`target`, `json_output`, `deep`).
- Removed unused `Snapshot::file_mtimes()` method.
- Changed all test `unwrap()` to `expect()` with context for cleaner error messages.

## [0.4.7] - 2025-11-28

### Added
- **Snapshot system** ("scan once, slice many"): Running bare `loctree` (no arguments) now scans the project and saves a complete graph snapshot to `.loctree/snapshot.json`.
- New `init` command/mode: `loctree init [path]` explicitly creates or updates the snapshot.
- Snapshot contains: file analyses (imports, exports, commands, events), graph edges, export index, command bridges (FE↔BE mappings), event bridges (emit↔listen), and barrel file detection.
- Snapshot metadata includes: schema version, generation timestamp, detected languages, file count, total LOC, and scan duration.
- Foundation for upcoming "holographic slice" feature (Vertical Slice 2) – context slicing from snapshot.
- **Janitor: circular imports** – new `--circular` flag walks the import graph and reports strongly connected components (including self-loops) as cycles in CLI/JSON.
- **Janitor: entry points** – new `--entrypoints` flag detects Python and Rust entry points (e.g. `if __name__ == "__main__"`, `fn main`, `#[tokio::main]`) to separate startup scripts from dead code.
- **SARIF output for CI** – new `--sarif` flag emits findings (duplicate exports, missing/unused handlers, dead exports, ghost/orphan events) in SARIF 2.1.0 format for GitHub/GitLab integration.
- **Find build artifacts** – new `--find-artifacts` flag finds common build artifact directories (`node_modules`, `.venv`, `target`, `dist`, `build`, `.cache`, `Pods`, `DerivedData`, etc.) and outputs their absolute paths one per line. Useful for cleaning up disk space or excluding from Spotlight indexing. Does not recurse into found directories (prune behavior).

### Changed
- Default behavior: bare `loctree` without arguments now runs in Init mode (creates snapshot) instead of Tree mode.
- Added `Serialize`/`Deserialize` derives to core analysis types for snapshot persistence.
- Made `root_scan` and `coverage` modules public for snapshot building.
- Snapshot summary now shows actionable next steps (`loctree . -A --json`, `loctree . -A --preset-tauri`) instead of not-yet-implemented slice command.

### Fixed
- `--fail-on-missing-handlers`, `--fail-on-ghost-events`, `--fail-on-races` flags now actually work: they return non-zero exit code when issues are detected (previously flags were parsed but had no effect).
- Python analyzer: fixed resolution of relative imports like `from . import mod` and `from .mod import name` so that star re-exports and `__all__` expansion are reflected correctly in the graph and dead-code analysis.

## [0.4.6] - 2025-11-27
### Added
- **Janitor Mode tools**:
  - `--check <query>`: Finds existing components/symbols similar to the query (Levenshtein distance) to prevent duplication before writing new code.
  - `--dead` (alias `--unused`): Lists potentially unused exports (defined but never imported).
  - `--confidence <level>`: Filters dead exports (use `high` to hide implicit uses like `default` exports or re-exports).
  - `--symbol <name>`: Quickly finds all occurrences of a symbol (definitions and usages) across the project.
  - `--impact <file>`: Analyzes dependency graph to show what would break if the target file changed.
  - `--scan-all`: Option to include `node_modules`, `target`, `.venv` in analysis (normally ignored by default).
- **Pipeline Confidence**: "Ghost events" now include confidence scores and recommendations (`safe_to_remove` vs `verify_dynamic_value`).
- **Graph UX**: Sticky tooltips on nodes (persist on hover/click) for easier reading and copying paths.
### Changed
- Default behavior: `loctree` (no args) now ignores `node_modules`, `target`, `.venv` by default to prevent massive snapshots. Use `--scan-all` to override.
- CLI output: Removed emojis from standard output for cleaner, grep-friendly text.
### Fixed
- Fixed false positives in "dead exports" where re-exports were not counted as usage.
- Fixed double-counting of named re-exports in parser.
- Fixed tooltip flickering in HTML report graph.

## [0.4.4] - 2025-11-27

### Security
- Replaced unmaintained `json5` crate (RUSTSEC-2025-0120) with actively maintained `json-five` for tsconfig.json parsing. No API or behavior changes.

### Fixed
- Added Semgrep suppression comments with safety justifications for `innerHTML` usage in `graph_bootstrap.js` (all user data is escaped via `escapeHtml()`; other values are numbers from analyzer).
- Replaced bare `unwrap()` with `expect()` providing context in snapshot module tests to comply with project linting rules.

## [0.4.3] - 2025-11-26

### Fixed
- HTML report no longer renders duplicate graph toolbars; inline graph panels are hidden so the drawer is the single source of controls (no double scrollbars).

### Changed
- Documentation updated for the streamlined graph UI.

## [0.4.2] - 2025-11-26

### Fixed
- Multi-root analyzer now merges frontend calls and backend handlers across roots, so Tauri coverage/commands summaries stop flagging cross-root missing/unused pairs.
- Duplicate export detection skips re-exports and `default` exports from declaration files, reducing barrel/index.ts false positives.
- Event names declared as constants (TS/JS/Rust, including imported consts) are resolved for emit/listen analysis, cutting ghost/orphan noise.

### Changed
- Analyzer scanning logic was extracted into dedicated modules (`scan.rs`/`root_scan.rs`), shrinking `runner.rs` and preparing the upcoming subcommands without changing CLI behavior.

## [0.4.1] - 2025-11-25

### Fixed
- TS path resolver now walks parent dirs to find `tsconfig.json` (or a base in parent), merges `extends`, parses JSONC/JSON5 (tsconfig with comments/trailing commas), and canonicalizes `baseUrl/paths`, so aliases like `@/*` resolve instead of returning null even when tsconfig lives above the scanned root.

## [0.4.0] - 2025-11-25

### Added
- `--ai` concise output mode that emits a compact JSON summary with top issues instead of full per-file payloads.
- Dead-symbol controls: `--top-dead-symbols` (default 20) to cap lists and `--skip-dead-symbols` to omit them entirely.
- AI/bridge payload now keeps a compact list of FE↔BE Tauri command mappings (`bridges`) so agents can jump to handlers/call-sites.

### Changed
- AI/summary views respect the new limits to reduce noisy output; help/README refreshed to mention the AI flags and limits.
- `--preset-tauri` now auto-ignores common build artifacts (`node_modules`, `dist`, `target`, `build`, `coverage`, `docs/*.json`) to cut report noise without extra flags.

### Fixed
- Resolved clippy warning in the open-server editor launcher (mutable closure), no functional change.

## [0.3.8] - 2025-11-24

### Added
- Report UI reorganized into tabs (Overview / Duplicates / Dynamic imports / Tauri coverage / Graph anchor) with a dedicated bottom drawer for the graph and controls.
- Help text split per mode (Tree / Analyzer / Common) and expanded examples; graph/drawer behavior documented.
- Python analyzer refinements: `--py-root` (repeatable) for extra roots, `resolutionKind` + `isTypeChecking` on imports, dynamic import tagging, `__all__` expansion for star imports.

### Fixed
- Dark-mode toggle in the graph drawer no longer panics when Cytoscape style is not ready.
- Resolved stray brace/formatting issues in CLI help output.

## [0.3.6] - 2025-11-23

### Added
- Python analyzer: TYPE_CHECKING-aware imports (`isTypeChecking`), dynamic import tagging (`importlib.import_module`, `__import__`), `__all__` expansion for star imports, and stdlib vs local disambiguation (`resolutionKind`).
- New flag `--py-root <path>` (repeatable) to add extra Python package roots for resolution.

### Changed
- JSON schema bumped to `1.2.0`; per-import records now include `resolutionKind` and `isTypeChecking`. Fixtures count as dev noise in duplicate scoring.

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
- Unified CLI features and JSON output across all runtimes (Node.js, Python, Rust): extension filters, ignore patterns, gitignore support, max depth, color modes, JSON output, and summary reporting (commit [`8962e39`](https://github.com/Loctree/Loctree/commit/8962e39)).
- Installation scripts for fast setup: `install.sh`, `install_node.sh`, and `install_py.sh` (commit [`b6824f4`](https://github.com/Loctree/Loctree/commit/b6824f4)).
- `--show-hidden` (`-H`) option to include dotfiles and other hidden entries in output in Rust and Python CLIs (commit [`12310b4`](https://github.com/Loctree/Loctree/commit/12310b4)).

### Changed
- Standardized the project name from `loc-tree` to `loctree` across runtimes, binaries, installers, and documentation; improved CLI UX and argument parsing, and enhanced error messages (commit [`e31d3a4`](https://github.com/Loctree/Loctree/commit/e31d3a4)).
- Usage/help output refined and examples clarified across Rust, Node, and Python CLIs (commit [`b6824f4`](https://github.com/Loctree/Loctree/commit/b6824f4) and [`8962e39`](https://github.com/Loctree/Loctree/commit/8962e39)).

### Documentation
- Expanded and clarified README with installation instructions, usage details, examples, and project structure overview (commits [`e31d3a4`](https://github.com/Loctree/Loctree/commit/e31d3a4), [`b6824f4`](https://github.com/Loctree/Loctree/commit/b6824f4), [`8962e39`](https://github.com/Loctree/Loctree/commit/8962e39)).

### Other
- Initial project setup (commit [`2031f80`](https://github.com/Loctree/Loctree/commit/2031f80)).

---

Release notes are generated from the last 5 commits on the default branch (`main`).
