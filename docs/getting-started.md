# Getting Started with loctree

5-minute quickstart to analyzing your codebase with loctree.

## Installation

```bash
# From crates.io (recommended)
cargo install loctree

# Or from source
git clone https://github.com/Loctree/Loctree
cd loctree/loctree_rs
cargo install --path .

# Verify installation
loct --version
```

## First Scan

Run loctree in any project directory:

```bash
cd your-project
loct
```

```
Analyzing: your-project
Stack detected: TypeScript (tsconfig.json)
Scanning 247 files...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 100% 247/247 files

Artifacts written to .loctree/
  snapshot.json   - Full dependency graph (127 KB)
  findings.json   - Issues detected (dead code, cycles, etc.)
  agent.json      - AI-optimized context bundle
  manifest.json   - Index for tooling

Health score: 82/100
  ✓ No circular imports
  ⚠ 3 unused exports (--confidence high)
  ⚠ 1 dead parrot (0 imports)
```

## Key Artifacts

After scanning, loctree creates `.loctree/` with these files:

### snapshot.json
Complete import/export graph. Query it with jq-style syntax:

```bash
loct '.metadata'                    # Project info
loct '.files | length'              # Count files
loct '.edges[] | select(.from | contains("api"))'  # Filter edges
```

### findings.json
All detected issues:

```bash
cat .loctree/findings.json | jq '.dead_exports[] | select(.confidence == "high")'
```

### agent.json
AI-optimized bundle with health score and quick wins:

```bash
cat .loctree/agent.json | jq '.summary'
```

### manifest.json
Index for IDE integrations and tooling:

```bash
cat .loctree/manifest.json
```

## Essential Commands

### Scan and analyze
```bash
loct                    # Full scan with auto-detection
loct --fresh            # Force rescan (ignore cache)
loct --watch            # Continuous scan on file changes
```

### Get context for a file
```bash
loct slice src/components/ChatPanel.tsx
```

Output shows 3 layers:
- **Core**: The file itself
- **Deps**: Direct and transitive dependencies
- **Consumers**: Files that import this file (use `--consumers`)

```
Slice for: src/components/ChatPanel.tsx

Core (1 files, 180 LOC):
  src/components/ChatPanel.tsx (180 LOC, tsx)

Deps (4 files, 320 LOC):
  [d1] src/hooks/useChat.ts (90 LOC)
    [d2] src/contexts/ChatContext.tsx (150 LOC)
    [d2] src/utils/api.ts (80 LOC)

Consumers (3 files, 240 LOC):
  src/App.tsx (120 LOC)
  src/routes/chat.tsx (80 LOC)
  src/layouts/MainLayout.tsx (40 LOC)

Total: 8 files, 740 LOC
```

Add `--json` for AI consumption:
```bash
loct slice src/main.rs --consumers --json | claude
```

### Search for symbols
```bash
loct find useAuth                # Find symbol definitions/usage
loct find ChatPanel              # Find similar components
loct f useAuth                   # Short alias
```

### Find dead exports
```bash
loct dead                        # All unused exports
loct dead --confidence high      # High confidence only
```

Detects:
- Unused exports across all languages
- Re-export chains (barrel files)
- Registry patterns (WeakMap/WeakSet)
- Python `__all__` declarations

### Detect circular imports
```bash
loct cycles
```

```
Circular import detected:
  src/components/UserProfile.tsx
  → src/hooks/useUser.ts
  → src/contexts/UserContext.tsx
  → src/components/UserProfile.tsx

Cycle length: 3 files
Impact: 12 files in component
```

### Quick health check
```bash
loct health
```

```
Health Score: 82/100

Issues:
  ✓ Circular imports: 0
  ⚠ Dead exports: 3 (high confidence)
  ⚠ Dead parrots: 1 (0 imports)
  ⚠ Twins: 0

Recommendations:
  1. Review unused export in src/utils/helpers.ts:45
  2. Check dead parrot: calculateDistance (src/geo/distance.ts)
```

### Full codebase audit
```bash
loct audit
```

Runs comprehensive checks:
- Circular imports
- Dead exports
- Twins (duplicate exports)
- Zombie code (orphan files + shadows)
- Functional crowds (clustering)

## Query Mode (jq-style)

Query snapshot data directly:

```bash
# Extract metadata
loct '.metadata'

# Count files
loct '.files | length'

# Find all dead exports
loct '.dead_parrots[]'

# Find cycles
loct '.cycles[]'

# Filter by path
loct '.files[] | select(.path | contains("src/api"))'
```

Options:
- `-r, --raw` - Raw output (no JSON quotes)
- `-c, --compact` - One line per result
- `-e, --exit-status` - Exit 1 if result is false/null

## Next Steps

### IDE Integration

loctree provides language server (LSP) integration for real-time analysis:

```bash
# Coming soon - see docs/ide/ for setup
loct lsp --install
```

See [ide/](ide/) for editor-specific setup guides.

### MCP Server

Use loctree as a Model Context Protocol server for AI agents:

```bash
# Coming soon - see docs/integrations/ for setup
loct mcp --start
```

See [integrations/](integrations/) for AI tool integration guides.

### CI Integration

Add loctree to your CI pipeline:

```bash
# GitHub Actions example
loct lint --fail --sarif > loctree.sarif
loct health --json | jq '.summary.health_score'
```

Fail on issues:
- `--fail` - Exit non-zero if findings detected
- `--sarif` - SARIF 2.1.0 output for GitHub/GitLab

### Tauri Projects

For Tauri applications, loctree provides specialized commands:

```bash
loct commands              # Show FE↔BE handler bridges
loct trace <handler>       # Trace handler end-to-end
loct events                # Event flow analysis
loct coverage --handlers   # Handler test coverage
```

## Getting Help

```bash
loct --help              # Main help
loct --help-full         # All 27 commands
loct <command> --help    # Per-command help
loct --help-legacy       # Legacy flag migration
```

## Common Workflows

### Before creating a new component
```bash
loct find ChatSurface
# Found: ChatPanel (distance: 2), ChatWindow (distance: 3)
# → Consider reusing ChatPanel instead
```

### Before refactoring
```bash
loct impact src/utils/api.ts
# Shows all files that depend on api.ts
```

### Continuous development
```bash
loct --watch
# Auto-rescans on file changes
# Press Ctrl+C to stop
```

### AI-assisted development
```bash
# Get AI bundle with health + quick wins
loct --for-ai > context.json

# Get context for specific file
loct slice src/main.rs --json | your-ai-tool
```

---

**Created by M&K (c)2025 The LibraxisAI Team**
Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)
