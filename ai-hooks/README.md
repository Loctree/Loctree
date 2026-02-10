# AI Hooks

Integration hooks for AI coding assistants (Claude Code, Codex CLI, Gemini CLI).

## Hook Packages

### 🌳 Loctree - Structural Analysis (v10)
**TRUE AUGMENTATION** - automatically adds semantic context to every grep/rg.

| Pattern Type | Example | Augmentation | Time |
|--------------|---------|--------------|------|
| **Any symbol** | `passthrough` | `loct find` - semantic + params + dead status | ~75ms |
| **PascalCase** | `McpToolsState` | `loct find` - definition + similar symbols | ~75ms |
| **snake_case** | `detect_language` | `loct find` - all matches + params | ~75ms |
| **Directory** | `src/providers/` | `focus` - all files + deps | ~56ms |
| **File** | `toolRegistry.ts` | `slice` - deps + consumers tree | ~81ms |
| **Tauri command** | `mcp_list_integrations` | Backend handler + Frontend calls | ~47ms |

**Key change in v10:** Uses `loct find` (~75ms) instead of `query where-symbol` for ALL symbol lookups. This provides:
- `symbol_matches`: Exact definitions with file + line
- `param_matches`: Function parameters matching the pattern (NEW in 0.8.4)
- `semantic_matches`: Similar symbols with similarity scores
- `dead_status`: Whether the symbol is exported and/or dead

**Works with:**
- `Grep` tool → PostToolUse:Grep

**Dual Output:**
- `stderr` → User sees in terminal
- `stdout` (JSON) → AI sees via `hookSpecificOutput.additionalContext`

| Hook | Trigger | Function |
|------|---------|----------|
| `loct-grep-augment.sh` | PostToolUse:Grep | Auto-adds semantic context via `loct find` |
| `loct-smart-suggest.sh` | PostToolUse:* | Proactive refactoring suggestions |

### 🧠 Memex - Memory/RAG Augmentation

**INSTITUTIONAL KNOWLEDGE** - automatically loads relevant memories from your vector DB.

| Hook | Trigger | Function | Time |
|------|---------|----------|------|
| `memex-startup.sh` | SessionStart | Load project-specific memories | ~50ms |
| `memex-context.sh` | PostToolUse:Grep | Augment grep with relevant memories | ~50ms |
| `memory-on-compact.sh` | PreCompact | Save session context before compaction | ~50ms |

**Requires:**
- `rmcp-memex` server running: `rmcp-memex serve --http-port 8987`
- Indexed memories (use `rmcp-memex index <file>` or MCP tools)

**Environment Variables:**
- `MEMEX_URL` - Server URL (default: `http://localhost:8987`)
- `MEMEX_LIMIT` - Max results per query (default: `3`)
- `MEMEX_TIMEOUT` - Request timeout in seconds (default: `3`)

## Installation

### Interactive (Recommended)
```bash
cd loctree-suite
make ai-hooks
```

You'll be prompted to choose which CLIs to configure (Claude Code, Codex CLI, etc.)

### Non-Interactive
```bash
# Install hooks for Claude Code
make ai-hooks CLI=claude

# Install hooks for all supported CLIs
make ai-hooks CLI=all
```

## Requirements

### Loctree hooks
- `loct` CLI installed (`make install` in loctree-suite)
- `jq` for JSON parsing

### Memex hooks (optional)
- `rmcp-memex` CLI installed (`cargo install rmcp-memex`)
- Memex server running (`rmcp-memex serve --http-port 8987`)
- `curl` and `jq` for HTTP API calls

The installer will offer to install missing dependencies.

## Manual Configuration

If automatic settings.json update fails, add hooks manually:

### Claude Code (~/.claude/settings.json)
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Grep",
        "hooks": [
          {"type": "command", "command": "~/.claude/hooks/loct-grep-augment.sh"}
        ]
      }
    ]
  }
}
```

## Environment Variables

### Loctree
- `LOCT_AUGMENT=0` - Disable loctree augmentation
- `LOCT_MAX_LINES=40` - Max output lines
- `LOCT_TIMEOUT=15` - Command timeout (seconds)

## Loctree Command Reference

### ⚡ FAST Commands (<100ms)

| Command | Description | Time |
|---------|-------------|------|
| `loct find <pattern>` | Semantic + params + dead status | ~75ms |
| `loct slice <file>` | File + deps + consumers | ~81ms |
| `loct impact <file>` | What breaks if file changes | ~49ms |
| `loct query who-imports <f>` | Files importing target | ~53ms |
| `loct query where-symbol <s>` | Where symbol is defined (exact) | ~75ms |
| `loct focus <dir>` | All files in directory + deps | ~56ms |
| `loct commands` | Tauri command bridges | ~47ms |
| `loct health` | Quick health summary | ~372ms |

> **All commands support `--json` output!**

### 🔍 Analysis Commands

| Command | Description |
|---------|-------------|
| `loct dead` | Unused exports (dead code) |
| `loct cycles` | Circular imports |
| `loct twins` | Duplicate symbol names |
| `loct zombie` | Dead + orphan + shadows |
| `loct routes` | FastAPI/Flask routes |

### 🔤 Single-Letter Aliases

| Alias | Command |
|-------|---------|
| `loct s` | slice |
| `loct f` | find |
| `loct i` | impact |
| `loct h` | health |
| `loct q` | query |

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│  Claude: Grep "detect_language" (ripgrep ~10ms)             │
│                           │                                 │
│                           ▼                                 │
│              PostToolUse:Grep hook fires                    │
│                           │                                 │
│                           ▼                                 │
│              ┌────────────────┐                             │
│              │ loct-grep      │                             │
│              │ augment.sh v10 │                             │
│              │ (~75ms)        │                             │
│              └────────────────┘                             │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────┐        │
│  │ 🌳 LOCTREE CONTEXT                              │        │
│  │ Symbol: classify.rs:27, twins.rs:500            │        │
│  │ Semantic: detect_stack (0.53), detect_crowd...  │        │
│  │ Dead: exported=true, dead=false                 │        │
│  └─────────────────────────────────────────────────┘        │
│                                                             │
│  USER sees: stderr output in terminal                       │
│  AI sees: system-reminder with full JSON context            │
└─────────────────────────────────────────────────────────────┘
```

## What's New in v10

1. **`loct find` is now FAST** (~75ms vs ~14500ms in older versions)
2. **Catch-all pattern matching** - any alphanumeric pattern ≥3 chars triggers augmentation
3. **Parameter matching** - finds function parameters, not just exports
4. **Semantic matching** - typo recovery, similar symbol suggestions
5. **Dead code status** - instant feedback on whether symbol is used

## Troubleshooting

### Hooks not running
1. Check settings.json syntax: `jq . ~/.claude/settings.json`
2. Verify hook is executable: `ls -la ~/.claude/hooks/`
3. Restart AI CLI

### Loctree not finding anything
1. Run initial scan: `loct scan` in project root
2. Check snapshot: `loct '.metadata'` (artifacts are cached by default; set `LOCT_CACHE_DIR=.loctree` for repo-local artifacts)

### No augmentation for a pattern
- Pattern must be ≥3 chars and alphanumeric
- `loct find` must return matches (check with `loct find <pattern> --json`)

---
Vibecrafted with AI Agents by VetCoders (c)2026 The Loctree Team
