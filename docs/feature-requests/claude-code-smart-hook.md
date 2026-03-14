# Feature Request: Claude Code Smart Grep→loct Suggestion Hook

## Summary

Bundle an optional Claude Code hook that intelligently suggests `loct` commands when grep patterns would benefit from code-aware analysis. This would be installed alongside `mcp-server-loctree` configuration.

## Motivation

AI agents (Claude Code, Cursor, etc.) frequently use grep/ripgrep for code search. However, many search patterns would be served better by loct's semantic analysis:

| grep pattern | What user wants | loct advantage |
|--------------|-----------------|----------------|
| `useAuthHandler` | Find hook definition + usages | `loct f` knows import chain |
| `ChatPanel` | Find component | `loct f` shows [DEF] + consumers |
| `run_agent` | Find Rust symbol | `loct f` works across TS+Rust |
| `dead` / `unused` | Find dead code | `loct '.dead_parrots'` is instant |
| `circular` | Find cycles | `loct '.cycles'` pre-indexed |

## Proposed Solution

### 1. New installer option

```bash
loct install-claude-hook    # Interactive setup
loct install-claude-hook -y # Auto-yes, non-interactive
```

### 2. What it installs

**Hook script** → `~/.claude/hooks/loct-smart-suggest.sh`
- Non-blocking (always exit 0)
- Pattern detection for 12+ use cases
- Max 3 suggestions per session (anti-spam)
- Suggests jq-style queries where applicable

**Settings update** → `~/.claude/settings.json`
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Grep",
        "hooks": [{
          "type": "command", 
          "command": "~/.claude/hooks/loct-smart-suggest.sh"
        }]
      }
    ]
  }
}
```

### 3. Combined with MCP server setup

The existing `mcp-server-loctree` installation could offer this as optional step:

```
$ loct mcp install

Installing MCP server for Claude Code...
✓ Added mcp-server-loctree to ~/.claude/settings.json

Would you like to install the smart grep→loct suggestion hook? [Y/n]
This shows helpful loct commands when grep patterns would benefit from code-aware search.

✓ Installed hook to ~/.claude/hooks/loct-smart-suggest.sh
✓ Added PreToolUse hook to settings.json

Done! Restart Claude Code to activate.
```

## Pattern Detection Cases

The hook detects when loct would genuinely help:

1. **PascalCase** (`UserProfile`) → `loct f` for React components/types
2. **useXxx hooks** (`useAuth`) → `loct f` with import chain
3. **handleXxx/onXxx** → `loct f` for event handlers
4. **snake_case** (`run_agent`) → `loct f` cross-language
5. **invoke/emit** → `loct trace` for Tauri bridge
6. **import/export** → `loct query who-imports`
7. **dead/unused/orphan** → `loct '.dead_parrots'`
8. **circular/cycle** → `loct '.cycles'`
9. **duplicate/twin** → `loct '.twins'`
10. **count/total** → `loct '.files | length'`

## Example Output

When Claude Code uses Grep with pattern `useAgentSlashHandler`:

```
┌─────────────────────────────────────────────────────────────
│ 🌳 Hook search? loct shows definition + import chain
│ → loct f useAgentSlashHandler
└─────────────────────────────────────────────────────────────
```

## Implementation Notes

- Hook reads tool input from stdin (JSON format)
- Uses jq for parsing, falls back to grep
- Session-based counter prevents suggestion spam
- Zero blocking - grep always runs, just with helpful hint

## Reference Implementation

Working prototype at: `~/.claude/hooks/loct-smart-suggest.sh`
(Created during Vista development session by M&K)

---

VibeCrafted with AI Agents (c)2026 Loctree Team
