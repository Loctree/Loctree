# VSCode Extension

The Loctree VSCode extension provides real-time dead code detection, circular import warnings, and code navigation powered by the `loctree-lsp` language server.

## Installation

### From Source (Current)

```bash
cd editors/vscode
npm install
npm run compile
```

Then in VSCode: `F1` → "Developer: Install Extension from Location" → select `editors/vscode`

### From Marketplace (Coming Soon)

```
ext install libraxis.loctree
```

## Features

### Diagnostics

The extension shows warnings directly in your editor:

| Diagnostic | Severity | Description |
|------------|----------|-------------|
| Dead Export | Warning | Export has 0 imports across codebase |
| Circular Import | Warning | File is part of an import cycle |
| Twin Symbol | Information | Symbol exported from multiple files |

### Hover Information

Hover over any export to see:
- Import count across the codebase
- Top consumer files
- Export location details

### Go to Definition

`F12` or `Ctrl+Click` on imports to jump to:
- Original export location
- Re-export chain resolution
- Cross-language definitions (TS → Rust for Tauri)

### Code Actions

`Ctrl+.` on diagnostics to access quick fixes:
- **Remove unused export** - Delete the export keyword
- **Add to .loctignore** - Suppress this warning
- **Show in HTML report** - Open detailed analysis

## Configuration

In VSCode settings (`Ctrl+,`):

```json
{
  "loctree.serverPath": "/custom/path/to/loctree-lsp",
  "loctree.autoRefresh": false,
  "loctree.trace.server": "verbose"
}
```

| Setting | Default | Description |
|---------|---------|-------------|
| `serverPath` | auto-detect | Path to loctree-lsp binary |
| `autoRefresh` | `false` | Re-scan on file save |
| `trace.server` | `off` | LSP message logging |

## Status Bar

The status bar shows loctree status:

- 🌳 **Loctree: healthy** - No issues detected
- 🌳 **Loctree: 5 dead** - Number of dead exports
- 🌳 **Loctree: loading** - Scanning in progress

Click to open the Output panel for details.

## Commands

Open command palette (`F1`) and search for "Loctree":

| Command | Description |
|---------|-------------|
| `Loctree: Refresh` | Re-run `loct` and update diagnostics |
| `Loctree: Open Report` | Open HTML report in browser |
| `Loctree: Show Health` | Display health score summary |

## Requirements

- Loctree CLI installed (`cargo install loctree`)
- Project must have `.loctree/` folder (run `loct` first)

## Troubleshooting

### No diagnostics appearing

1. Check Output panel → "Loctree" for errors
2. Ensure `.loctree/snapshot.json` exists
3. Run `loct` in project root

### Server not starting

```bash
# Check if loctree-lsp is in PATH
which loctree-lsp

# Or set custom path in settings
"loctree.serverPath": "/path/to/loctree-lsp"
```

### Stale diagnostics

Click status bar → "Loctree: Refresh" or run `loct` in terminal.

---

*Created by M&K (c)2025 The LibraxisAI Team*
