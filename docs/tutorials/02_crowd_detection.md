# Crowd Detection — Finding Functional Duplicates

`loct crowd` detects **functional crowds** — groups of files clustering around the same functionality. This helps identify:

- **Dead parrots**: Files that look alive but nobody uses
- **Accidental duplicates**: Multiple implementations of the same thing
- **Refactoring targets**: High-similarity files that could be consolidated

## Quick Start

```bash
# Scan first (creates snapshot)
loct scan

# Auto-detect all crowds in codebase
loct crowd

# Find files clustering around specific pattern
loct crowd message
loct crowd patient
loct crowd SOAP

# JSON output for AI agents
loct crowd assistant --json
```

## How It Works

Crowd detection uses three signals:

### 1. Name Clustering
Files matching a pattern by name (case-insensitive substring match):
- `loct crowd auth` finds: `AuthContext.tsx`, `useAuth.ts`, `auth.rs`, `authService.ts`

### 2. Import Similarity (Jaccard)
Measures overlap between files' import sets:
```
similarity = |imports_A ∩ imports_B| / |imports_A ∪ imports_B|
```
- Score 1.0 = identical imports (likely duplicates!)
- Score 0.5+ = high overlap (potential consolidation)
- Score 0.0 = no common imports

### 3. Usage Asymmetry
Compares importer counts across crowd members:
- If `AuthContext.tsx` has 50 importers but `useAuthLegacy.ts` has 2 → asymmetry detected
- Underused files are candidates for removal

## Output Format

### Human-Readable (default)

```
╭─ CROWD: "assistant" ─────────────────────────────────────╮
│ Crowd Score: 10.0/10 (HIGH - needs attention!)
│
│ 📁 FILES IN CROWD (38 files)
│   AssistantAnchorContext.tsx     █████ 50 importers
│   useAssistantActivity.ts        ██    12 importers
│   AssistantHostManager.tsx       █      5 importers
│   useAssistantLegacy.ts                 0 importers  ← dead parrot?
│
│ 🔍 ISSUES DETECTED
│   • Usage asymmetry: AssistantAnchorContext.tsx is primary, 3 underused
│   • Export overlap: AssistantAnchorContext ↔ useAssistantActivity (1.0)
│   • Name collision: 38 files with similar names
╰──────────────────────────────────────────────────────────╯
```

### JSON (for AI agents)

```bash
loct crowd assistant --json
```

```json
[
  {
    "pattern": "assistant",
    "members": [
      {
        "file": "src/contexts/AssistantAnchorContext.tsx",
        "match_reason": { "type": "name_match", "matched": "assistant" },
        "importer_count": 50,
        "similarity_scores": [
          ["src/hooks/useAssistantActivity.ts", 1.0],
          ["src/contexts/AssistantPresenceContext.tsx", 0.5]
        ]
      }
    ],
    "score": 10.0,
    "issues": [
      { "type": "usage_asymmetry", "primary": "...", "underused": ["..."] },
      { "type": "export_overlap", "files": ["..."], "overlap": ["..."] }
    ]
  }
]
```

## CLI Options

```bash
loct crowd [PATTERN] [OPTIONS]

ARGUMENTS:
    [PATTERN]    Pattern to detect crowd around (e.g., "message", "patient")
                 If not specified, auto-detects all crowds

OPTIONS:
    --auto, -a       Detect all crowds automatically (default if no pattern)
    --min-size <N>   Minimum crowd size to report (default: 2)
    --limit <N>      Maximum crowds to show (default: 10)
    --json           Output as JSON for AI agents
    --help, -h       Show help
```

## Interpreting Crowd Score

| Score | Meaning | Action |
|-------|---------|--------|
| 0-3   | Low     | Healthy separation, no action needed |
| 4-6   | Medium  | Review for potential consolidation |
| 7-10  | High    | Needs attention — likely dead code or duplicates |

Score factors:
- More members → higher score
- More issues → higher score
- Usage asymmetry → adds 0.5 per underused file

## Issue Types

### Usage Asymmetry
```
• Usage asymmetry: AuthContext.tsx is primary, 3 underused
```
One file dominates usage while others are rarely imported. The underused files are likely:
- Legacy code that should be removed
- Duplicates that should be consolidated
- Specialized variants that should be documented

### Export Overlap
```
• Export overlap: AuthContext ↔ useAuth (similarity: 0.85)
```
Two files have very similar import patterns, suggesting:
- They do the same thing (consolidate them)
- One wraps the other (check if wrapper is needed)
- Copy-paste duplication (refactor)

### Name Collision
```
• Name collision: 5 files with similar names
```
Multiple files matching the same pattern can confuse:
- Developers (which `auth.ts` do I import?)
- AI agents (wrong context in slice)
- Build tools (aliasing issues)

## Real-World Examples

### Example 1: Dead Parrot Detection

```bash
$ loct crowd hook

╭─ CROWD: "hook" ─────────────────────────────────────╮
│ 📁 FILES IN CROWD (12 files)
│   useAuth.ts                     █████ 45 importers
│   usePatient.ts                  ████  32 importers
│   useAuthLegacy.ts                      0 importers  ← DEAD
│   usePatientOld.ts                      0 importers  ← DEAD
```

**Action**: Remove `useAuthLegacy.ts` and `usePatientOld.ts`

### Example 2: Consolidation Target

```bash
$ loct crowd message --json | jq '.[] | .members[] | select(.similarity_scores | any(.[1] > 0.8))'
```

Finds files with >80% import similarity — strong consolidation candidates.

### Example 3: AI Agent Workflow

```bash
# 1. Find crowds
loct crowd --json > crowds.json

# 2. Extract high-score crowds for review
jq '[.[] | select(.score > 7)]' crowds.json

# 3. Get slice for the most problematic file
loct slice src/hooks/useMessageHandler.ts --consumers --json | claude
```

## Integration with Other Commands

```bash
# Find crowd, then check if members are dead exports
loct crowd message
loct dead --path "message"

# Find crowd, then trace impact of removing underused file
loct crowd auth
loct find --impact src/hooks/useAuthLegacy.ts

# Combine with diff to see if crowd grew
loct diff --since main --problems-only
```

## Best Practices

1. **Run after major features**: New feature added 5 files? Check for crowds.
2. **Review before refactoring**: `loct crowd <area>` before touching code.
3. **CI integration**: Alert on crowd score > 8 in changed directories.
4. **Document intentional crowds**: Some duplication is OK (tests, mocks).

## Troubleshooting

### "No files found matching pattern"
- Pattern is case-insensitive substring match
- Try broader pattern: `auth` instead of `useAuth`
- Ensure snapshot exists: `loct scan` first

### High score but files are intentionally separate
- Tests and mocks naturally cluster
- Use `--min-size 3` to filter small crowds
- Consider `.loctreeignore` for intentional duplicates

### Similarity scores all 0.0
- Files have no common imports
- This is actually good — they're truly independent
- Score comes from name collision only

---

Developed with 💀 by The Loctree Team (c)2025.
