# Use Case: Agent Recon Loop — Map Before You Code

> How AI agents use loctree to build a mental model of the codebase before touching a single line.

**Context:** Real agent workflow observed in production (CodeScribe, macOS Rust+AppKit)
**Date:** 2026-02

## The Problem

An AI agent receives a task: "refactor module X, remove tab Y, redirect routing to overlay Z." The agent has two choices:

1. **Grep and pray** — search for strings, open 40 files, start editing, discover breakage mid-flight.
2. **Recon first** — build a structural map, understand dependencies, then operate with precision.

Without structural awareness, agents waste tokens on exploration, create duplicate code, miss consumers, and break downstream modules they didn't know existed.

## The Recon Loop (3 phases)

### Phase 1: Map the terrain

```bash
loct auto                           # Build/refresh snapshot
loct --for-ai                       # AI-optimized overview (health, hubs, quick wins)
```

After this, the agent knows: project structure, entry points, health score, hot spots. No guessing.

### Phase 2: Zoom into the task

```bash
loct find Transcription             # Where is the symbol defined, exported, imported?
loct find show_transcription_tab    # All call-sites before removing anything
loct slice app/ui/voice_chat --consumers --json  # Full dependency context
```

The agent now has:
- Every export and re-export of the target symbol
- Every file that consumes the module being changed
- The exact blast radius of the refactor

### Phase 3: Validate after changes

```bash
loct twins                          # Catch dead code left behind
loct health                         # Quick check: cycles, dead, twins
cargo clippy -- -D warnings         # Compiler-level validation
cargo test                          # Runtime validation
```

## Why This Beats grep

| Step | grep approach | loctree approach |
|------|--------------|-----------------|
| Find symbol | `rg "Transcription"` → 200 string matches | `loct find Transcription` → exports, imports, call-sites with roles |
| Understand scope | Open files one by one, trace manually | `loct slice <file> --consumers` → full dependency graph |
| Check for dead code | Hope someone notices | `loct twins` → immediate detection |
| Verify completeness | "I think I got everything" | Snapshot diff → structural proof |

## Real Example: Agent Plans a Refactor

From a production session, the agent's **first move** before any code change:

```
0) First: "dependency map" so we don't chase the compiler blindly
   - loct --for-ai app/ui/voice_chat
   - loct find Transcription / loct find show_transcription_tab
   - Goal: list all exports/re-exports and all call-sites BEFORE removing anything.
```

The agent chose this approach **organically** — nobody prompted it to use loctree. The tool was useful enough that the agent reached for it as step zero.

## The Pattern

```
RECON    →  loct auto + loct --for-ai + loct find <symbol>
CONTEXT  →  loct slice <file> --consumers --json
CHANGE   →  implement with full structural awareness
VALIDATE →  loct twins + loct health + clippy + tests
```

This loop works for any agent (Claude Code, Codex, Cursor, custom) on any codebase loctree supports.

## Key Insight

The recon phase typically takes 5-10 seconds. The alternative — an agent exploring blindly, opening wrong files, creating duplicates, breaking consumers — takes minutes to hours and often requires human intervention to fix.

**5 seconds of recon prevents 5 hours of cleanup.**

---

*Extracted from production agent sessions. VibeCrafted with AI Agents (c)2026 VetCoders*
