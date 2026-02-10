# LSP Protocol Reference

Technical reference for the `loctree-lsp` Language Server Protocol implementation.

## Server Capabilities

```json
{
  "textDocumentSync": {
    "openClose": true,
    "save": { "includeText": false }
  },
  "hoverProvider": true,
  "definitionProvider": true,
  "referencesProvider": true,
  "codeActionProvider": {
    "codeActionKinds": ["quickfix", "refactor"]
  },
  "diagnosticProvider": {
    "interFileDependencies": true,
    "workspaceDiagnostics": true
  }
}
```

## Supported Methods

### Lifecycle

| Method | Support | Notes |
|--------|---------|-------|
| `initialize` | ✅ | Returns capabilities |
| `initialized` | ✅ | Loads snapshot, starts file watcher |
| `shutdown` | ✅ | Cleanup |
| `exit` | ✅ | Process termination |

### Text Document

| Method | Support | Notes |
|--------|---------|-------|
| `textDocument/didOpen` | ✅ | Triggers diagnostics |
| `textDocument/didSave` | ✅ | Refreshes diagnostics |
| `textDocument/didClose` | ✅ | Clears diagnostics |
| `textDocument/hover` | ✅ | Import stats, consumers |
| `textDocument/definition` | ✅ | Export location |
| `textDocument/references` | ✅ | All import locations |
| `textDocument/codeAction` | ✅ | Quick fixes |
| `textDocument/publishDiagnostics` | ✅ | Dead/cycle/twin warnings |

### Workspace

| Method | Support | Notes |
|--------|---------|-------|
| `workspace/didChangeConfiguration` | ✅ | Settings updates |
| `workspace/didChangeWatchedFiles` | ✅ | Snapshot refresh |

## Diagnostic Types

### Dead Export (`loctree:dead-export`)

```json
{
  "range": { "start": {"line": 10, "character": 7}, "end": {"line": 10, "character": 20} },
  "severity": 2,
  "code": "dead-export",
  "source": "loctree",
  "message": "Export 'unusedFunction' has 0 imports",
  "data": {
    "symbol": "unusedFunction",
    "confidence": "high"
  }
}
```

**Severity levels:**
- `Warning (2)`: High confidence dead export
- `Information (3)`: Low confidence (might be API)
- `Hint (4)`: Test-only export

### Circular Import (`loctree:cycle`)

```json
{
  "range": { "start": {"line": 1, "character": 0}, "end": {"line": 1, "character": 30} },
  "severity": 2,
  "code": "cycle",
  "source": "loctree",
  "message": "Circular import: a.ts → b.ts → c.ts → a.ts",
  "relatedInformation": [
    {
      "location": { "uri": "file:///project/b.ts", "range": {...} },
      "message": "imports c.ts"
    }
  ]
}
```

### Twin Symbol (`loctree:twin`)

```json
{
  "range": { "start": {"line": 5, "character": 7}, "end": {"line": 5, "character": 13} },
  "severity": 3,
  "code": "twin",
  "source": "loctree",
  "message": "Symbol 'Config' also exported from 3 other files",
  "relatedInformation": [
    {
      "location": { "uri": "file:///project/other/config.ts", "range": {...} },
      "message": "Also exported here"
    }
  ]
}
```

## Code Actions

### Quick Fixes (`quickfix`)

| Action | Trigger | Effect |
|--------|---------|--------|
| Remove unused export | `dead-export` | Deletes `export` keyword |
| Add to .loctignore | `dead-export` | Appends pattern |
| Show in report | `dead-export` | Opens HTML report |
| Go to next in cycle | `cycle` | Navigates to next file |

### Refactors (`refactor`)

| Action | Description |
|--------|-------------|
| Extract to barrel | Move export to index.ts |
| Inline barrel export | Replace re-export with direct |
| Consolidate twins | Show picker for all locations |

## Custom Commands

Executable via `workspace/executeCommand`:

| Command | Parameters | Description |
|---------|------------|-------------|
| `loctree.refresh` | none | Re-run analysis |
| `loctree.openReport` | none | Open HTML report |
| `loctree.navigateToFile` | `path: string` | Open file in editor |

## Initialization Options

```json
{
  "initializationOptions": {
    "workspaceRoot": "/path/to/project",
    "autoRefresh": false
  }
}
```

## Error Codes

| Code | Message | Resolution |
|------|---------|------------|
| -32001 | Snapshot not found | Run `loct` first |
| -32002 | Snapshot stale | Run `loct` to refresh |
| -32003 | Parse error | Check file syntax |

---

*Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team*
