# Loctree for VSCode

Dead code detection, circular import analysis, and codebase navigation powered by [loctree](https://github.com/LibraxisAI/loctree-suite).

## Features

- **Dead Export Detection**: Highlights unused exports with configurable severity
- **Circular Import Warnings**: Shows cycles in your dependency graph
- **Duplicate Export Detection**: Finds symbols exported from multiple files
- **Rich Hover Information**: See import counts and top consumers
- **Go to Definition**: Navigate to export locations across files
- **Find All References**: See all files importing a symbol
- **Quick Fixes**: Remove unused exports or add to .loctignore

## Requirements

1. Install the loctree CLI:
   ```bash
   cargo install loctree
   ```

2. Scan your project:
   ```bash
   cd your-project
   loct
   ```

This creates a `.loctree/` folder with the analysis snapshot.

## Usage

The extension activates automatically when a `.loctree/` folder is detected.

### Commands

- **Loctree: Refresh Analysis** - Re-run `loct` to update the snapshot
- **Loctree: Open HTML Report** - View the interactive dependency graph
- **Loctree: Show Health Summary** - Quick health check in terminal
- **Loctree: Analyze Change Impact** - See what breaks if you modify a file

### Status Bar

The status bar shows current health:
- ✓ **Loctree: healthy** - No issues detected
- ⚠ **Loctree: 5 dead, 2 cycles** - Issues found (click for details)

### Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `loctree.serverPath` | Path to loctree-lsp binary | (auto-detect) |
| `loctree.autoRefresh` | Refresh on file save | `false` |
| `loctree.showStatusBar` | Show status in status bar | `true` |
| `loctree.diagnosticSeverity` | Severity for dead exports | `warning` |

## Supported Languages

- TypeScript / JavaScript
- Rust
- Python (partial)
- Go (partial)

## Development

```bash
cd editors/vscode
npm install
npm run compile
```

Press F5 to launch the extension development host.

## License

MIT - Created by M&K (c)2025 The LibraxisAI Team
