# Loctree Python Analyzer: Quick Win Plan (for a new dev)

This repo’s “analyzer” mode scans code and emits import/export graphs plus AI-friendly JSON (`--json` / `--json-summary`). You don’t need deep Loctree history—just the shape:
- Entrypoint: `loctree_rs/src/analyzer/runner.rs` orchestrates per-file analysis and aggregates JSON/HTML.
- Per-language parsers live in `loctree_rs/src/analyzer/*.rs` (Python in `py.rs`; helpers in `resolvers.rs`; types in `types.rs`).
- Coverage and insights get baked into `aiViews` in JSON and the HTML report tables/graph.

Goal of this plan: reduce Python noise (false duplicate/missing), improve resolution, and expose better signals for mixed py/ts/rs stacks. Keep changes scoped and observable via JSON/HTML and a few targeted tests.

## Objectives (high level)
1) **TYPE_CHECKING-aware imports**: Skip imports inside `if typing.TYPE_CHECKING:` from “unused/missing” calculations; mark them as low-signal (metadata) so dup/coverage stays clean.
2) **Dynamic import tagging**: Detect `importlib.import_module(...)`, `__import__(...)`, and f-string/module-format patterns; emit them as `dynamicImports` rather than “missing”, capturing the format string for visibility.
3) **__all__ / star re-export expansion**: When a module does `from .mod import *` and the target defines `__all__`, expand to explicit symbols; otherwise tag the star so the graph shows a re-export edge (not a missing module).
4) **Stdlib vs local disambiguation**: If an import matches both stdlib and a local file, prefer the local file when it exists under provided roots; otherwise tag the edge as stdlib to suppress phantom “missing module” warnings.
5) **Namespace/package roots**: Allow additional Python roots (e.g., `--py-root` or inferred from `pyproject.toml` [tool.setuptools.packages.find]); treat `src-tauri/packages/*` or similar as roots for resolution instead of only the scan root.
6) **Test/fixture down-weighting**: Auto-classify `tests/**`, `**/fixtures/**` as low-priority for duplicate severity so they don’t dominate dup clusters.
7) **Cross-lang stem hint (optional)**: If py/ts/rs files share a stem in sibling dirs, add an AI hint that they may be a binding pair (helps find orphaned bindings).
8) **Graph/edge labeling**: For Python edges, tag type: `static`, `dynamic`, `guessed`, `stdlib`; allow filtering stdlib edges in HTML later (plumb the data now).

## What to change (conceptual, not line-by-line)
- **Parser adjustments (py.rs)**:
  - Track when you’re inside `if typing.TYPE_CHECKING:` and mark imports there as `is_type_checking = true`; store them separately or flag them on ImportEntry.
  - Add regex/parsing for `importlib.import_module(...)`, `__import__(...)`, f-strings like `f\"pkg.{name}\"`; push these into `dynamic_imports` with the raw expression.
  - When seeing `from X import *`, try to resolve `X` and, if its content has `__all__`, expand to named exports; otherwise store a star re-export entry.
  - When resolving imports, consult a stdlib list (tiny set or `rustpython-stdlib-list` equivalent baked in) to tag edges as stdlib when no local file matches.

- **Resolver support (resolvers.rs)**:
  - Add optional extra roots list (fed from CLI) for Python resolution.
  - When resolving, prefer a file under known roots before declaring stdlib; if both, choose local and mark `isStdlib=false`.
  - Expose enough metadata on ImportEntry (e.g., `resolution_kind: local|stdlib|dynamic|unknown`).

- **Options / CLI (args.rs & runner.rs)**:
  - Add optional `--py-root <path>` (repeatable) and pass into analysis options.
  - Ensure default roots still work; when `pyproject.toml` exists with `[tool.setuptools.packages.find]`, seed roots from there.

- **Aggregation (runner.rs)**:
  - When building `aiViews` and `duplicateExports`, down-weight or skip type-checking imports and test/fixture files for severity calculations.
  - Keep `dynamicImports` for Python populated with the new forms.
  - If you add new metadata fields (e.g., `resolutionKind`, `isTypeChecking`), include them in JSON (files[*].imports) and sort deterministically.

- **Tests**:
  - Add unit-level tests in `tools/tests/loctree-py.test.mjs` (pattern) or new Rust unit tests for `analyze_py_file` covering:
    - TYPE_CHECKING block is parsed but excluded from “missing/unused”.
    - `importlib.import_module(f\"pkg.{name}\")` ends up in dynamicImports with raw string.
    - `from .mod import *` + `__all__ = ['Foo']` expands to Foo export.
    - Stdlib vs local: prefer local when file exists; otherwise mark stdlib.
  - A small fixture tree under `fixtures/py/` with minimal files is enough.

## Acceptance criteria
- No regression in `cargo test`, `cargo clippy`, and `cargo check`.
- New JSON fields are present and sorted; `--json-summary` (if wired) includes the new signals.
- Running on a mixed repo with aliases/stdlib overlap yields fewer false “missing”/dup entries for Python, and dynamic imports are visible instead of missing.

## How to run locally
- `cargo fmt && cargo clippy --all-targets --all-features && cargo test`
- Quick spot check: `cargo run -- --ext py --json fixtures/py` (or similar) to inspect JSON for the new flags/fields.

## Hand-off notes
- Keep changes minimal and localized (py.rs, resolvers.rs, args/runner for new flags).
- If you add fields to JSON, bump `SCHEMA_VERSION` and note in CHANGELOG.
- Don’t block on HTML filters; just surface the metadata so UI can consume later.
