# loctree – fast reference for agents (CLI & outputs)

This is a concise, up-to-date guide for how to drive loctree as of 0.3.3, focused on analyzer mode and machine-readable outputs.

## Modes
- **Tree/LOC (default)**: `loctree <roots> [options]` – prints an ASCII tree with LOC highlights.
- **Import/Export analyzer**: add `-A` – detects imports, exports, re-exports, duplicate exports, dynamic imports, Tauri command calls/handlers, and can emit HTML/JSON/JSONL.

## Inputs & filters (analyzer)
- `--ext <list>`: restrict files by extension. Default analyzer set: `ts,tsx,js,jsx,mjs,cjs,rs,css,py`.
- `-g, --gitignore`: honor .gitignore.
- `-I, --ignore <path>`: repeatable ignore paths.
- `--ignore-symbols <list>` / `--ignore-symbols-preset <name>`: drop noisy symbols (preset `common` → `main,run,setup,test_*`).
- `--focus <glob[,..]>`: show duplicate groups only if any file matches.
- `--exclude-report <glob[,..]>`: hide duplicate groups whose files match globs (analysis still runs).
- `--limit <N>`: cap top lists (dups, dynamic imports); default 8.

## Outputs
- `--json` prints JSON to stdout; `--json-out <file>` writes JSON (parents auto-created; warns on overwrite). `--jsonl` (analyzer) emits one JSON object per root per line.
- `--html-report <file>` writes HTML (parents auto-created). `--graph` embeds an interactive Cytoscape graph (self-hosted asset). `--serve` requires `--html-report` and starts a tiny local server so links can open in editor/OS; `--editor-cmd` customizes the opener.
- `--verbose`: extra debug logs (paths of written reports, graph warnings).

Graph safety: defaults `MAX_GRAPH_NODES=8000`, `MAX_GRAPH_EDGES=12000`; overridable via `--max-nodes/--max-edges`. When limits hit, graph is skipped and a warning is logged (visible with `--verbose` in CLI; also shown in HTML if present).

## JSON shape (analyzer)
Schema is declared in the report:
- Top-level: `schema`, `schemaVersion` (currently `1.1.0`), `generatedAt` (UTC RFC3339), `rootDir`, `root`, `languages`, `filesAnalyzed`, `duplicateExports*`, `reexportCascades`, `dynamicImports`, `commands`, `commands2`, `symbols`, `clusters`, `aiViews`, `files`.
- Files: stable `id`, `path` (relative to root), `loc`, `language`, `kind` (`code|test|story|config|generated`), `isTest`, `isGenerated`, imports (`sourceRaw`, `resolvedPath`, `isBareModule`, symbols with `name/alias`, kind), exports (`name`, `kind`, `exportType`, `line`), reexports (`star/named` + resolved), command calls/handlers with lines.
- Derived sections:
  - `commands2`: canonical handler per command + call sites + status (`ok|missing_handler|unused_handler`).
  - `symbols`/`clusters`: occurrences with canonical pick, severity, duplicateScore, reasons.
  - `aiViews`: default export chains, suspicious barrels, dead symbols, `ciSummary` (duplicate cluster counts, top clusters).
Legacy sections (`commands`, `duplicateExports*`, etc.) remain for compatibility.

## Graph drawer (HTML)
- Graph and controls live in a bottom drawer with a toggle. Toolbar includes text filter, min-degree, labels on/off, fit/reset/fullscreen/dark mode, PNG/JSON export. Component panel lists disconnected components (id, size, sample, isolates, edges, LOC) with highlight/dim/copy/export controls.

## Typical commands
- FE/BE combined (with graph + HTML + JSON):
  ```bash
  mkdir -p logs/loctree/reports
  loctree -A src src-tauri/src --ext ts,tsx,rs,css --gitignore --graph \
    --exclude-report "**/__tests__/**" \
    --json-out logs/loctree/reports/report.json \
    --html-report logs/loctree/reports/report.html \
    --verbose
  ```
- Quick JSON only:
  ```bash
  loctree -A src --ext ts,tsx --gitignore --json > /tmp/loctree.json
  ```

## Notes / gaps
- No `--fail-on-*` guards yet; CI gating must be scripted externally.
- `--serve` requires `--html-report`; graph still respects size limits—use `--max-nodes/--max-edges` if needed.
