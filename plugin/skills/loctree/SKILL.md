---
name: loctree
description: >
  Structural code intelligence for codebase understanding. Use when exploring
  a new codebase, before modifying files, before creating new symbols, before
  deleting or refactoring, or when you need to understand dependency relationships.
  Provides repo-view, slice, find, impact, focus, tree, and follow tools via MCP.
---

# Loctree - Structural Code Intelligence

Use loctree MCP tools for codebase awareness BEFORE reading files manually.

## Baseline Protocol

1. **repo-view** - Start here. Get overview: files, LOC, languages, health, top hubs.
2. **slice(file)** - Before modifying any file. Returns file + dependencies + consumers.
3. **find(name)** - Before creating anything new. Symbol search with regex support.
4. **impact(file)** - Before deleting or major refactor. Shows blast radius.
5. **focus(directory)** - Understand a module. Files, internal edges, external deps.
6. **tree** - Directory structure with LOC counts.
7. **follow(scope)** - Pursue signals from repo-view: dead exports, cycles, twins, hotspots.

## When to Use

- Starting a new session on any codebase
- Before editing a file - use slice() to understand its role
- Before creating a new function/type - use find() to check if it exists
- Before deleting or refactoring - use impact() to check blast radius
- When navigating unfamiliar code - use focus() on a directory
- After repo-view flags issues - use follow() to get field-level detail and fix recommendations

## Key Principle

**Scan once, query everything.** First tool call auto-scans the project.
Subsequent calls use cached snapshot (instant). No config needed.
