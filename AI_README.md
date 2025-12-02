# loct — AI Agent Quick Reference (v0.5.9)

Static analysis for AI agents: scan once, slice many. Default `loct` writes `.loctree/snapshot.json`; run `loct -A` or `loct lint --sarif` if you need full artifacts (`analysis.json`, `report.html`, `report.sarif`, etc.).

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

# 4) Quick queries (new!)
loct query who-imports src/utils.ts    # what files import this
loct query where-symbol useAuth        # find symbol definitions
loct query component-of src/api.ts     # graph component

# 5) Tauri FE↔BE coverage
loct commands --missing   # FE calls without BE handlers
loct commands --unused    # Handlers without FE calls
loct events --json        # Emit/listen, ghost/orphan/races

# 6) Hygiene
loct dead --confidence high  # unused exports (alias-aware)
loct cycles                  # circular imports

# 7) Delta / diff (new!)
loct diff --since main       # compare against another snapshot

# 8) CI / policy
loct lint --fail --sarif > results.sarif
```

## Install

```bash
cargo install loctree
loct --version   # expect 0.5.9+
```

## Auto-Detect Stack

| Marker          | Stack      | Auto-Ignores                |
|-----------------|------------|-----------------------------|
| `Cargo.toml`    | Rust       | `target/`                   |
| `tsconfig.json` | TypeScript | `node_modules/`, `dist/`    |
| `pyproject.toml`| Python     | `.venv/`, `__pycache__/`    |
| `src-tauri/`    | Tauri      | All above                   |
| `vite.config.*` | Vite       | `dist/`, `build/`           |

## What to Run (by goal)

- **Context for AI**: `loct slice <file> --consumers --json` (deps always included)
- **Find duplicates/usage**: `loct find <pattern>` (fuzzy + defs/uses)
- **Quick queries**: `loct query who-imports <file>`, `loct query where-symbol <sym>`
- **Impact**: `loct impact <file>`
- **Dead code**: `loct dead --confidence high`
- **Circular imports**: `loct cycles`
- **Tauri FE↔BE**: `loct commands --missing`, `loct commands --unused`, `loct events --json`
- **Delta between scans**: `loct diff --since <snapshot_id>`
- **CI guardrails**: `loct lint --fail --sarif > results.sarif`

## Tips

- `loct` caches analyses; use `--full-scan` to force a rescan.
- Artifacts live in `.loctree/` after each scan: `snapshot.json`, `analysis.json`, `report.sarif`, `report.html`.
- SARIF file integrates with GitHub/GitLab code scanning and IDEs.
- SARIF includes `loctree://open?f=<file>&l=<line>` URLs for IDE integration.
- Respect `.gitignore` by default; add `--scan-all` to include node_modules/target/.venv.

## Philosophy

Know *why* it works (or doesn’t):
- Import graphs over assumptions
- Dead-code and barrel/alias awareness to cut false positives
- Holographic slices so AI writes with real context, not guesses

Developed with 💀 by The Loctree Team (c)2025.
