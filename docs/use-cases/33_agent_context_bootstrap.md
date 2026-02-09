# Use Case: 60-Second Onboarding — Agent Bootstraps a New Codebase

> How an AI agent goes from "I know nothing about this repo" to "I know what to change and what it'll break" in under a minute.

**Context:** Any codebase, any agent. The universal cold-start problem.
**Date:** 2026-02

## The Problem

Every agent session starts the same way: the agent has a task but doesn't know the codebase. Common failure modes:

1. **The grep spiral** — agent searches for strings, opens 30 files, loses context, starts over
2. **The duplicate creation** — agent builds `fetchUser()` when `getUserData()` already exists
3. **The hub file bomb** — agent edits `types.ts` without knowing 62 other files depend on it
4. **The dead trail** — agent extends a module that's actually unused (dead code building on dead code)

All of these waste tokens, time, and human patience.

## The 60-Second Bootstrap

### Second 0-5: Overview

```bash
loct --for-ai
```

Output (condensed):
```
Project: CodeScribe (Rust + AppKit)
Files: 230, LOC: 110K
Languages: Rust (87%), Swift (8%), Other (5%)

Health: 92/100
  Cycles: 0
  Dead exports: 2 (low confidence)
  Twins: 0

Hub files (change carefully):
  types.rs          62 importers
  snapshot.rs       22 importers
  analyzer/mod.rs   12 importers

Entry points: loct (bin), loctree (bin), loctree-mcp (bin)
```

Agent now knows: size, languages, health, what NOT to touch carelessly.

### Second 5-15: Find the target

```bash
loct find <task-relevant-symbol>
```

Example: task is "add attachment support to compose input"

```bash
loct find clipboard
loct find paste
loct find NSPasteboard
loct find compose
```

Output shows: where symbols are defined, whether they're exported/imported, which modules own them. No grep noise.

### Second 15-30: Get structural context

```bash
loct slice app/ui/compose/mod.rs --consumers --json
```

Agent receives:
- **Core**: the file itself (LOC, language, exports)
- **Dependencies**: what it imports (and their dependencies, transitively)
- **Consumers**: who imports this module (blast radius of changes)

### Second 30-45: Check for existing work

```bash
loct find attachment
loct twins
```

Does `Attachment` type already exist? Are there duplicate implementations? Is there dead code in the area the agent is about to extend?

### Second 45-60: Understand impact

```bash
loct impact app/ui/compose/mod.rs
```

What breaks if the agent modifies this file? Direct consumers, transitive consumers, total blast radius.

## After 60 Seconds, the Agent Knows:

| Question | Answer source |
|----------|--------------|
| What's the project structure? | `loct --for-ai` |
| Where is the code I need to change? | `loct find` |
| What depends on it? | `loct slice --consumers` |
| Does similar code already exist? | `loct find` + `loct twins` |
| What breaks if I change this? | `loct impact` |
| Is the area healthy or debt-ridden? | `loct health` |

Compare this to 15-30 minutes of grep + manual file reading + hoping you found everything.

## MCP Server: Zero-Install Agent Integration

For agents that support MCP (Model Context Protocol), loctree runs as a server:

```json
{
  "mcpServers": {
    "loctree": {
      "command": "loctree-mcp",
      "args": []
    }
  }
}
```

Tools available via MCP: `repo-view`, `slice`, `find`, `impact`, `focus`, `tree`. Same data, no CLI parsing needed. Auto-scans on first call.

## The Pattern for Any New Session

```bash
# 1. Where am I?
loct --for-ai

# 2. Where is the thing I need?
loct find <symbol>

# 3. What's around it?
loct slice <file> --consumers

# 4. Does it already exist?
loct find <new-thing-name>

# 5. What breaks if I change it?
loct impact <file>
```

Five commands, under a minute, full structural awareness.

## Key Insight

The fastest agent isn't the one that types code the quickest. It's the one that **understands the codebase before typing anything**.

Loctree compresses the "understand" phase from minutes/hours to seconds — and the understanding is structural (graph-based), not textual (string-based).

---

*Extracted from production agent sessions. VibeCrafted with AI Agents (c)2026 VetCoders*
