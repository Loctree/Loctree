---
name: loctree
description: >
  A holographic map of code for AI agents. Use loctree-mcp as the primary
  mapping layer before any edit, delete, or refactor. Build context with
  repo-view/focus/slice/impact/find/follow, then use grep only for local detail.
---

# Loctree — A holographic map of code for AI agents

Static code analysis built for agentic codebase context management.
Find Dead Parrots, Ministry of Silly Exports, and 14k-file features
before they haunt you.

Use loctree MCP tools as the default discovery layer — **before reading files manually**.

## Hard Rules

1. **No edit-before-map**: do not patch code until `repo-view -> focus -> slice`.
2. **No blind delete**: run `impact(file)` before deleting or collapsing modules.
3. **No duplicate symbols**: run `find(name)` before adding functions/types/components.
4. **Scope changes**: prefer feature-scoped changes over global overrides.
5. **Treat high-impact files as risky**: add verification steps before commit.
6. **Grep is local detail only**: never use grep/rg as primary mapping layer.

## Standard Workflow (Required)

### 1) Map repo
- Run `repo-view(project)` once per task.
- Note: top hubs, dead exports, cycles, twins, health indicators.

### 2) Constrain scope
- Run `focus(project, directory)` for each target module (1-3 dirs max).
- Capture external consumers and dependencies.

### 3) File-level context before edit
- For every file to modify, run `slice(file, consumers=true)`.
- Record: direct dependencies, direct consumers, likely transitive risk.

### 4) Change risk check
- For each candidate high-impact file, run `impact(file)`.
- If risk is high, isolate changes (new variant, scoped class, adapter layer).

### 5) Symbol check before creating
- Run `find(name)` for new types/functions/components/keys.
- Supports regex: `find("SymbolA|SymbolB")`.
- Reuse existing patterns when available.

### 6) Pursue signals
- Run `follow(scope)` to drill into problems flagged by repo-view.
- Scopes: `dead`, `cycles`, `twins`, `hotspots`, or `all`.
- Returns field-level detail with actionable recommendations.

### 7) Line-level read
- Use `grep`, `rg`, `cat` only after steps 1-6.

### 8) Validation
- Re-run targeted tests/lint/typecheck.
- For UI: add visual sanity check across themes/modes.

## Tools Reference

| Tool | When | What you get |
|------|------|-------------|
| `repo-view(project)` | Start of task | Files, LOC, languages, health, top hubs |
| `slice(file)` | Before modifying | File + dependencies + consumers |
| `find(name)` | Before creating | Symbol search with regex, multi-query |
| `impact(file)` | Before deleting | Direct + transitive consumers (blast radius) |
| `focus(directory)` | Module deep-dive | Files, internal edges, external deps |
| `tree(project)` | Layout overview | Directory structure with LOC counts |
| `follow(scope)` | After repo-view | Field-level signals: dead, cycles, twins, hotspots |

All tools accept `project` parameter (default: current dir).
First use auto-scans if no snapshot exists. Subsequent calls use cache (instant).

## Output Contract (for agents)

Before refactor, report:
1. **Repo Summary** (3-5 bullets from `repo-view`)
2. **Scope** (focused dirs and why)
3. **Critical Files** (from `slice`)
4. **Risk Map** (from `impact`)
5. **Plan** (ordered phases + rollback points)

After implementation, report:
1. **Changed Files**
2. **Why**
3. **Validation**
4. **Residual Risk**

## Anti-Patterns (Disallowed)

- Running broad `grep` first and editing based on partial matches.
- Deleting/renaming files without `impact`.
- Adding new symbols without `find`.
- Global overrides to fix local issues.
- Skipping `slice` because "it's a small change".

## Portable by Design

- No shell hooks, custom wrappers, or local plugin scripts required.
- No repo-specific paths assumed beyond current workspace.
- If loctree-mcp unavailable: fallback to `rg --files` + `rg -n` + manual dependency tracing.
