# loctree

Tree viewer with line-of-code (LOC) counts, shipped in four flavors (Node.js, Python, Rust, Bash). Point it at any directory and you get an `eza`-style tree plus per-file LOC totals, optional filters, and a quick summary of heavyweight files.

## Highlights
- **Cross-runtime**: run it with Node.js (`loctree.mjs`), Python (`loctree.py`), Rust (`loc_tree_rs`), or the tiny Bash prototype (`loctree.sh`).
- **Extension filters**: focus on specific languages via `--ext rs,ts,tsx,py`.
- **Selective sweeps**: limit recursion depth (`-L 2`), skip folders (`-I node_modules`), or respect `.gitignore` (`--gitignore`).
- **Readable output**: clean tree structure, optional ANSI color for 1000+ LOC items, JSON/ND-friendly output, and an end-of-run summary of the largest files.
- **Relative-friendly**: paths such as `../project` or `./src-tauri` work anywhere—you can even call `loctree.mjs` from another repo.

## Node CLI usage
The Node variant has the richest feature set and serves as the canonical CLI.

```bash
node loctree.mjs [root ...] [options]
```

| Option | Description |
| --- | --- |
| `--ext <list>` | Comma-separated extensions to include (e.g. `--ext rs,tsx`). Dots are optional. Non-matching files/empty dirs are pruned from the tree. |
| `-I`, `--ignore <path>` | Ignore a folder/file (relative to `root` or absolute). Repeatable. |
| `--gitignore`, `-g` | Skip entries ignored by Git (requires `git`). |
| `-L`, `--max-depth <n>` | Cap recursion depth (`0` = direct children, `2` = subfolders of subfolders, etc.). |
| `--color[=mode]`, `-c` | Colorize large files. mode: `auto` (default), `always`, `never`; bare `-c` equals `always`. |
| `--json` | Emit JSON instead of a tree view (includes summary + entries). |
| `--summary[=N]` | Print totals plus top-N large files (default 5). |
| `--help`, `-h` | Show help. |

> **Tip:** If you omit `root`, the tool scans the current working directory. Relative paths are resolved against where you run the command.

### Examples
```bash
# Whole repo, TypeScript + Rust only, highlight big files
node loctree.mjs . --ext rs,ts,tsx --color

# Sweep only visit/audio code, ignore generated output, stop after depth 2
node loctree.mjs packages/app/src -I dist -I coverage -L 2

# Respect gitignore and summarize Python files anywhere
node loctree.mjs ../VistaScribe --gitignore --ext py

# Machine-readable output with summary
node loctree.mjs . --json --summary=3 > tree.json

# Multiple roots in one go
loctree src src-tauri/src --ext rs,tsx,ts,json,py -I src-tauri/src/secure -L 3
```

When a file reaches 1000+ LOC, it is highlighted (when `--color` is set) and also listed under a “Large files” section at the end of the report.

## Other runtimes
Node is the “reference” implementation, but the Python and Rust versions now expose the **same flags and behaviour** (color modes, JSON output, summaries), so you can pick whichever runtime you prefer:

- **Rust (`loc_tree_rs`)** – `cargo run -- <root ...> [options]` for the fastest traversal.
- **Python (`loctree.py`)** – `python loctree.py [root ...] [options]` for a no-build dependency.
- **Bash (`loctree.sh`)** – tiny prototype (defaults to `src-tauri/src`, adjust `root` inside the script) that prints LOC via `wc -l`.

Feel free to add new switches in whichever runtime you like—just mirror them across Node/Python/Rust so the experience stays identical.

## Tools

### `tools/apply_patch.py`
Need to apply a Codex-style patch outside the CLI? Drop your diff into `tools/apply_patch.py`. It accepts three input modes:

- Inline argument (for short diffs) – `python tools/apply_patch.py "*** Begin Patch\\n..."`
- Path to a patch file – `python tools/apply_patch.py my.patch`
- Standard input – `cat diff.patch | python tools/apply_patch.py -`

The script understands `*** Add File`, `*** Update File` (plus optional `*** Move to`), and `*** Delete File` blocks. It applies hunks using their context/line numbers and surfaces clear errors when something cannot be matched.

## License
MIT – reuse it, fork it, or wire it into your own build/scripts.
