# Use Case: Auditable "Done" — Snapshot as Proof

> When an agent says "done," how do you know it's actually done? Loctree turns claims into evidence.

**Context:** The trust gap between AI agent output and human verification
**Date:** 2026-02

## The Problem

AI agents complete tasks and report "done." But:

- Did the agent actually remove all dead code, or just the obvious ones?
- Did the refactor introduce new circular imports?
- Are there orphaned re-exports the agent didn't notice?
- Did the agent create duplicate symbols that already existed elsewhere?

Without structural verification, "done" is an opinion. With a snapshot, "done" is an artifact.

## The Workflow: Before/After Snapshots

### Before the task

```bash
loct                    # Build snapshot
cp -r .loctree/latest .loctree/before-refactor   # Save state
```

The snapshot captures: every file, every import edge, every export, every dead symbol, every cycle. This is the **baseline**.

### Agent performs the task

The agent works: removes components, redirects routing, cleans up dead code. Reports "done."

### After the task

```bash
loct                    # Rebuild snapshot
loct health             # Quick structural summary
```

Now you can compare:

```bash
# What changed structurally?
loct diff --since before-refactor

# New dead code introduced?
loct '.dead_parrots | length'

# New cycles?
loct '.cycles | length'

# Orphaned files?
loct '.orphans | length'
```

## What "Auditable Done" Looks Like

### Agent report (claim):
> "Removed Transcription tab, redirected non-assistive to overlay, cleaned up dead exports."

### Loctree verification (evidence):

```
Health Check Summary (after)

Cycles:      0 total (was 0)          -- no regression
Dead:        2 high confidence (was 8) -- 6 removed, 2 are test-only
Twins:       0 duplicate groups (was 1) -- resolved
Orphans:     0 new files              -- clean

Files changed: 14
Edges removed: 6 (transcription imports)
Edges added:   2 (overlay routing)
Exports removed: 4 (voice_chat transcription API)
```

This is verifiable without reading a single line of code.

## The Trust Equation

| Verification method | Time | Confidence | Scales? |
|---------------------|------|------------|---------|
| Read every diff line | 30-60 min | High | No |
| Run tests only | 2-5 min | Medium (misses structural debt) | Yes |
| Snapshot diff | 10 sec | High (structural + semantic) | Yes |
| Tests + snapshot diff | 2-5 min | Very high | Yes |

The snapshot catches what tests miss: dead code accumulation, new cycles, orphaned modules, duplicate symbols. Tests verify behavior; snapshots verify structure.

## CI Integration

Make "auditable done" automatic:

```yaml
# .github/workflows/structural-check.yml
- name: Structural baseline
  run: loct

- name: Check no regressions
  run: |
    dead=$(loct '.dead_parrots | length')
    cycles=$(loct '.cycles | length')
    if [ "$dead" -gt "$ALLOWED_DEAD" ] || [ "$cycles" -gt 0 ]; then
      echo "Structural regression detected"
      loct health
      exit 1
    fi
```

## SARIF for PR Reviews

```bash
loct lint --sarif > report.sarif
```

Upload to GitHub — reviewers see structural issues inline, not buried in agent logs.

## Key Insight

The real luxury in the age of AI agents isn't speed — it's **auditability**.

An agent that codes fast but leaves unverifiable state is a liability. An agent that produces a snapshot diff alongside its code diff is a partner.

```
Without snapshot: "I removed the dead code."  → trust me
With snapshot:    dead_parrots: 8 → 2         → verify me
```

---

*Extracted from production agent sessions. VibeCrafted with AI Agents (c)2026 VetCoders*
