# loctree

LOC-aware tree with Git/gitignore awareness, pruning by extension, JSON output, and per-root summaries. Available in Rust (recommended), Node.js, and Python.

## Why use it
- Filters by extension (`--ext rs,ts,tsx,py,...`) and prunes non-matching branches.
- Respects gitignore (`-g`), custom ignores (`-I`), and max depth (`-L`).
- Human or JSON output; per-root summary of totals and “large” files (>=1000 LOC).
- Multi-root: pass several paths in one command.

## Install
Recommended (Rust binary):
```bash
curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install.sh | sh
# INSTALL_DIR=$HOME/bin CARGO_HOME=$HOME/.cargo curl -fsSL ... | sh
```

Alternatives:
- Node wrapper: `curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_node.sh | sh`
- Python wrapper: `curl -fsSL https://raw.githubusercontent.com/LibraxisAI/loctree/main/tools/install_py.sh | sh`

*(Each installer drops a wrapper in `$HOME/.local/bin` by default and hints if PATH needs updating.)*

## Quick start
```bash
loctree src --ext rs,tsx --summary
loctree src src-tauri/src -I node_modules -L 2
loctree . --json > tree.json
```
JSON shape: single root -> object; multi-root -> array. Large files (>=1000 LOC) are listed separately and colored when `--color` (or `-c`) is on.

## CLI flags (all runtimes)
- `--ext <list>`         Comma-separated extensions; prunes others.
- `-I, --ignore <path>`  Ignore path (repeatable; abs or relative).
- `-g, --gitignore`      Respect gitignore via `git check-ignore`.
- `-L, --max-depth <n>`  Limit recursion depth.
- `--color[=mode]`       auto|always|never (default auto); `-c` = always.
- `--json`               Machine-readable output.
- `--summary[=N]`        Totals + top-N large files (default 5).

## Dev & tests
- Node: `node tools/tests/loctree-node.test.mjs`
- Python: `node tools/tests/loctree-py.test.mjs`
- Rust: `node tools/tests/loctree-rs.test.mjs` and `cargo check`

CI runs the three test scripts on PRs/pushes (see `.github/workflows/ci.yml`).

## Patch helper
`tools/apply_patch.py` applies Codex-style `*** Begin Patch` blocks (stdin, arg string, or file). Handy for scripted edits/CI where Codex isn’t available.

## License
MIT – reuse it, fork it, wire it into your own scripts.
