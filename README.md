# loctree — LOC-aware tree for codebases

[![CI](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml/badge.svg)](https://github.com/LibraxisAI/loctree/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

loctree prints a directory tree that includes per-file line counts (LOC), totals, and highlights “large” files. It’s
designed to be fast, scriptable, and usable from multiple runtimes: Rust (native), Node.js, and Python.

## Overview

- Filters by extension (`--ext rs,ts,tsx,py,...`) and prunes non-matching branches.
- Respects `.gitignore` (`-g`), custom ignores (`-I`), and max depth (`-L`).
- Human or JSON output; per-root summary with totals and large files (>= 1000 LOC).
- Multi-root: pass several paths in one command.
- Import/export analyzer mode (`-A/--analyze-imports`) surfaces duplicate exports, re-export chains, dynamic imports,
  CSS `@import`, Rust `use/pub use`/public items, Python `import`/`from` + `__all__` + `importlib`/`__import__`
  dynamic loads, oraz mapuje komendy Tauri: wywołania FE (`safeInvoke`/`invokeSnake`) vs. backend `#[tauri::command]`
  (brakujące/nieużywane handlery w CLI/JSON/HTML). Z `--serve` linki w HTML otwierają pliki w edytorze/OS (domyślnie
  `code -g` lub `open`/`xdg-open`).

Common use cases:

- Pre-review hygiene: quickly spot oversized files or heavy folders (`--summary`, color highlights for >= 1000 LOC).
- Language-focused sweeps: `--ext ts,tsx` for FE, `--ext rs` for Rust, `--ext py` for Python modules, `--ext css` for
  styling audits.
- CI scripting: `--json` feeds automated checks (e.g., fail if any file > N LOC or if new files appear outside an
  allowlist).
- General repo hygiene: combine `--gitignore` and repeated `-I` to skip generated/build/output trees.

## Stack and entry points

- Rust (primary native CLI)
    - Package: `loc_tree_rs` (Cargo binary `loctree` at `loc_tree_rs/src/main.rs`)
- Node.js (ESM script)
    - Entry: `loctree.mjs`
- Python (single-file script)
    - Entry: `loctree.py`
- Shell helper
    - `loctree.sh` is an example script aimed at a specific path (`src-tauri/src`); not part of the cross-runtime CLI.

## Requirements

- Rust toolchain: cargo + rustc (edition 2021)
    - Tested in CI on “stable” via `dtolnay/rust-toolchain@stable`
- Node.js: v20.x (per CI) or newer
    - Note: the code uses ESM (`import`), so Node must support ESM
- Python: 3.11 (per CI) or newer

Optional tools used by installers:

- curl or wget (to fetch scripts)

TODOs

- Specify minimal Rust version (MSRV). Currently not pinned. TODO: decide and document.
- Windows support notes. TODO: verify behavior of `--gitignore` on Windows shells.
- Prebuilt release artifacts/Homebrew tap automation. A helper exists, but publishing is manual for now.

## Installation

Recommended (Rust native binary):

```bash
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh
```

# Env overrides:

# INSTALL_DIR=$HOME/.local/bin  CARGO_HOME=$HOME/.cargo  curl -fsSL ... | sh

Alternatives (wrappers around repo scripts):

- Node wrapper:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_node.sh | sh
  ```
- Python wrapper:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_py.sh | sh
  ```

Each installer drops a small wrapper into `$HOME/.local/bin` by default and will hint if your PATH needs updating.

Environment variables recognized by installers:

- `INSTALL_DIR` — where to place the runnable wrapper (default: `$HOME/.local/bin`)
- `CARGO_HOME` — cargo home for Rust install (default: `~/.cargo`) [Rust installer]
- `LOCTREE_HOME` — where to store downloaded script file [Node: `$HOME/.local/lib/loctree-node` | Python:
  `$HOME/.local/lib/loctree-py`]

## Usage

Quick start:

```bash
loctree src --ext rs,tsx --summary
loctree src src-tauri/src -I node_modules -L 2
loctree . --json > tree.json
loctree packages/app -A --ext ts,tsx --json   # import/export analyzer
```

JSON shape: single root -> object; multi-root -> array. Large files (>= 1000 LOC) are listed separately and colored when
`--color` (or `-c`) is on.

CLI flags (all runtimes):

- `--ext <list>`         Comma-separated extensions; prunes others (analyzer defaults to ts,tsx,js,jsx,mjs,cjs,rs,css,py).
- `--ignore-symbols <l>` Analyzer mode: comma-separated symbol names to skip in duplicate-export detection (case-insensitive).
- `-I, --ignore <path>`  Ignore path (repeatable; abs or relative).
- `-g, --gitignore`      Respect gitignore via `git check-ignore`.
- `-L, --max-depth <n>`  Limit recursion depth.
- `-H, --show-hidden`    Show dotfiles and `.DS_Store`.
- `--color[=mode]`       `auto|always|never` (default `auto`); `-c` = always.
- `--loc <n>`            Large-file threshold for highlighting (tree mode). Default 1000.
- `--json`               Machine-readable output.
- `--jsonl`              Analyzer: one JSON object per line (per root).
- `--html-report <file>` Write analyzer results to an HTML report file.
- `--graph`              Embed an interactive import graph in the HTML report (Cytoscape.js from CDN).
- `--serve`              Start a tiny local server so HTML links can open files in your editor/OS.
- `--editor-cmd <tpl>`   Command template for opening files (`{file}`, `{line}`), default tries `code -g`.
- `--ignore-symbols <l>` Analyzer mode: comma-separated symbol names to skip in duplicate-export detection (case-insensitive).
- `--ignore-symbols-preset <name>` Analyzer mode: predefined ignore set (currently `common` → `main,run,setup,test_*`).
- `--summary[=N]`        Totals + top-N large files (default 5).
- `-A, --analyze-imports` Import/export analyzer mode (duplicate exports, re-export cascades, dynamic imports).
- `--limit <N>`          Analyzer: cap top lists for duplicates/dynamic imports (default 8).
- `--fail-on-duplicates <N>` Analyzer: exit 2 if duplicate-export groups exceed N (for CI).
- `--fail-on-dynamic <N>`   Analyzer: exit 2 if files with dynamic imports exceed N (for CI).

Runtime-specific entry points:

- Rust: `loc_tree_rs/` via cargo
    - Run: `cargo run --quiet --manifest-path loc_tree_rs/Cargo.toml -- . --summary`
    - Build binary locally: `cargo build --release --manifest-path loc_tree_rs/Cargo.toml`
- Node: `node loctree.mjs . --summary`
- Python: `python3 loctree.py . --summary`

## Scripts and automation

- Installers:
    - `tools/install.sh` (Rust via `cargo install --git`)
    - `tools/install_node.sh` (downloads `loctree.mjs` and writes `loctree-node` wrapper)
    - `tools/install_py.sh` (downloads `loctree.py` and writes `loctree-py` wrapper)
- CI: `.github/workflows/ci.yml` runs 3 jobs on pushes/PRs to `main`:
    - Node tests: `node tools/tests/loctree-node.test.mjs` (Node 20)
    - Python tests: `node tools/tests/loctree-py.test.mjs` (Python 3.11)
    - Rust tests: `node tools/tests/loctree-rs.test.mjs` (Rust stable)
- Release helper:
    - `tools/release/update-tap.py` — prints a Homebrew formula body for release artifacts (manual step). TODO: wire a
      publishing workflow.

## Environment variables

The CLI itself does not require environment variables for normal operation.

Install-time variables (see Installation above): `INSTALL_DIR`, `CARGO_HOME`, `LOCTREE_HOME`.

## Development and tests

- Node: `node tools/tests/loctree-node.test.mjs`
- Python: `node tools/tests/loctree-py.test.mjs`
- Rust: `node tools/tests/loctree-rs.test.mjs` and `cargo check`

CI runs all three test scripts on PRs/pushes (see `.github/workflows/ci.yml`).

## Project structure

Selected files and directories:

```
.
├─ loctree.mjs            # Node.js ESM CLI
├─ loctree.py             # Python CLI
├─ loctree.sh             # Example shell helper (not part of main CLI)
├─ loc_tree_rs/           # Rust crate (binary `loctree`)
│  ├─ Cargo.toml
│  └─ src/main.rs
├─ tools/
│  ├─ install.sh          # Rust installer (cargo install --git)
│  ├─ install_node.sh     # Node wrapper installer
│  ├─ install_py.sh       # Python wrapper installer
│  ├─ fixtures/basic-tree # Test fixtures
│  └─ tests/              # Test runners (Node)
└─ .github/workflows/ci.yml
```

## Patch helper

`tools/apply_patch.py` applies Codex-style `*** Begin Patch` blocks (stdin, arg string, or file). Useful for scripted
edits/CI.

## License

MIT — reuse it, fork it, wire it into your own scripts. See [LICENSE](LICENSE).
