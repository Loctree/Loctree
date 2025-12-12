# loct — AI Agent Quick Reference (v0.6.15)

Static analysis for AI agents: scan once, slice many. Default `loct` writes `.loctree/<branch@sha>/snapshot.json`; use `loct report --serve` (or `loct lint --sarif`) when you need full artifacts (`analysis.json`, `report.html`, `report.sarif`, etc.).

**v0.6.15 highlights**: jq-style query mode (`loct '.metadata'`), bundle distribution analysis (`loct dist`), Dart/Flutter support.

> **Full documentation:** [AI Agent's Manual](docs/tutorials/ai-agents-manual.md)

## Core Flow

```bash
# 1) Scan from repo root (auto-detects stack, writes snapshot + reports)
loct

# 2) Extract context for a task (3 layers: core, deps, consumers)
loct slice src/components/ChatPanel.tsx --consumers --json | claude

# 3) Find before you create (fuzzy + defs/uses in one)
loct find ChatPanel                  # avoid duplicates
loct find useAuth                    # definitions & uses
loct find src/utils/api.ts           # quick impact-ish lookup

# 4) Quick queries
loct query who-imports src/utils.ts    # what files import this
loct query where-symbol useAuth        # find symbol definitions
loct query component-of src/api.ts     # graph component

# 5) jq-style queries (NEW! v0.6.15)
loct '.metadata'                       # extract snapshot metadata
loct '.files | length'                 # count files
loct '.edges[] | select(.from | contains("api"))'  # filter edges
loct '.command_bridges | map(.name)'   # list all command names
loct '.files[] | select(.loc > 500)' -c  # large files, compact

# 6) Tauri FE↔BE coverage
loct commands --missing   # FE calls without BE handlers
loct commands --unused    # Handlers without FE calls
loct events --json        # Emit/listen, ghost/orphan/races

# 7) Hygiene
loct dead --confidence high  # unused exports (alias-aware)
loct cycles                  # circular imports

# 8) Twins analysis (semantic duplicates)
loct twins                   # dead parrots + exact twins + barrel chaos
loct twins --dead-only       # only exports with 0 imports

# 9) Crowd detection (functional duplicates)
loct crowd                   # auto-detect all crowds
loct crowd message           # files clustering around "message"
loct crowd --json            # JSON for AI agents

# 10) Delta / diff
loct diff --since main       # compare against another snapshot

# 11) Bundle analysis (verify tree-shaking)
loct dist dist/bundle.js.map src/  # compare source vs bundle

# 12) CI / policy
loct lint --fail --sarif > results.sarif
```

## Install

```bash
cargo install loctree
loct --version   # expect 0.6.15+
```

## Auto-Detect Stack

| Marker          | Stack         | Auto-Ignores                | Languages |
|-----------------|---------------|-----------------------------| --------- |
| `Cargo.toml`    | Rust          | `target/`                   | Rust |
| `tsconfig.json` | TypeScript    | `node_modules/`, `dist/`    | TS, JS, JSX, TSX |
| `pyproject.toml`| Python        | `.venv/`, `__pycache__/`    | Python |
| `src-tauri/`    | Tauri         | All above                   | TS, Rust |
| `vite.config.*` | Vite          | `dist/`, `build/`           | Auto |
| `pubspec.yaml`  | Dart/Flutter  | `.dart_tool/`, `build/`     | Dart |
| `go.mod`        | Go            | `vendor/`                   | Go |

## What to Run (by goal)

- **Context for AI**: `loct slice <file> --consumers --json` (deps always included)
- **Find duplicates/usage**: `loct find <pattern>` (fuzzy + defs/uses)
- **Quick queries**: `loct query who-imports <file>`, `loct query where-symbol <sym>`
- **jq-style queries**: `loct '.files | length'`, `loct '.edges[] | select(...)'`
- **Impact**: `loct impact <file>`
- **Dead code**: `loct dead --confidence high`
- **Circular imports**: `loct cycles`
- **Twins analysis**: `loct twins` (dead parrots, exact twins, barrel chaos)
- **Functional crowds**: `loct crowd` (find similar files clustering around same functionality)
- **Bundle analysis**: `loct dist <sourcemap> <srcdir>` (verify tree-shaking, symbol-level)
- **Tauri FE↔BE**: `loct commands --missing`, `loct commands --unused`, `loct events --json`
- **Delta between scans**: `loct diff --since <snapshot_id>`
- **CI guardrails**: `loct lint --fail --sarif > results.sarif`

## CLI cheat sheet (`loct --help`)

- Scan & cache: `loct` (writes `.loctree/<branch@sha>/snapshot.json`)
- Slice for AI: `loct slice <file> [--consumers --json]`
- Quick queries: `loct query who-imports <file>`, `loct query where-symbol <sym>`, `loct query component-of <file>`
- **jq queries**: `loct '.metadata'`, `loct '.files | length'`, `loct '.edges[]' -c`
- Twins analysis: `loct twins` (dead parrots + exact twins + barrel chaos)
- Bundle dist: `loct dist dist/bundle.js.map src/` (symbol-level dead export detection)
- Analysis shortcuts: `loct -A --dead`, `loct -A --circular`, `loct -A --report report.html`
- Diff snapshots: `loct diff --since <main|HEAD~N|snapshot_id>`
- Serve report: `loct -A --report report.html --serve`
- Full options: `loct --help-full` (agent-friendly)

## Tips

- `loct` caches analyses; use `--full-scan` to force a rescan.
- Artifacts live in `.loctree/<branch@sha>/`: `snapshot.json` is always written; `analysis.json`/`report.html`/`report.sarif` are produced via `loct report` or `loct lint --sarif`.
- SARIF file integrates with GitHub/GitLab code scanning and IDEs.
- SARIF includes `loctree://open?f=<file>&l=<line>` URLs for IDE integration.
- Respect `.gitignore` by default; add `--scan-all` to include node_modules/target/.venv.
- Events: set `LOCT_EVENT_ALIASES="rust://foo=tauri://foo,legacy_evt=new_evt"` to bridge cross-language names; self-emits with matching literals in the same file are treated as resolved to reduce orphan noise.

## Rust-Specific Features (v0.5.17+)

- **Crate-internal imports** — Resolves `use crate::foo::Bar`, `use super::Bar`, `use self::foo::Bar` for accurate dead code detection
- **Same-file usage** — Detects when exported symbols like `BUFFER_SIZE` are used locally (in generics, type annotations, etc.)
- **Nested brace imports** — Handles complex imports like `use crate::{foo::{A, B}, bar::C}`

## SvelteKit-Specific Features (v0.5.17+)

- **Virtual modules** — Resolves `$app/navigation`, `$app/stores`, `$lib/*` paths
- **Runtime modules** — Maps SvelteKit runtime internals correctly

## Dead Code Detection Improvements (v0.6.x)

- **WeakMap/WeakSet Patterns** — Detects registry patterns (React DevTools, observability tools)
- **Flow Type Annotations** — Understands Flow syntax alongside TypeScript
- **Re-export Chains** — Tracks .d.ts files and barrel exports (Svelte, library types)
- **Python `__all__`** — Respects public API declarations in Python modules
- **Library Mode** — Auto-detects npm packages (package.json "exports") and Python stdlib (Lib/ directory)

## Dart/Flutter Support (v0.6.x)

- **Full language support** — Imports, exports, dead code detection
- **Auto-detection** — Recognizes `pubspec.yaml`, ignores `.dart_tool/`, `build/`
- **Package imports** — Resolves `package:` imports correctly

## Philosophy

Know *why* it works (or doesn't):
- Import graphs over assumptions
- Dead-code and barrel/alias awareness to cut false positives
- Holographic slices so AI writes with real context, not guesses

Developed with 💀 by The Loctree Team (c)2025.
