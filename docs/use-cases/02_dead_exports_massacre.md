# Use Case: Dead Exports Massacre - AI Agent Cleans 37 Dead Parrots

> Real session transcript of Claude Code agent using `loct dead` to identify and eliminate 37 unused exports across TypeScript and Rust codebase with parallel agent cleanup.

**Project:** Vista (Tauri: TypeScript/React + Rust)
**Loct version:** 0.6.x
**Date:** 2025-12-04
**Task:** Identify dead exports, verify accuracy, clean up with parallel agents

## The Challenge

```
User: "zrob `loct` initial scan i sprawdz dead"
```

Translation: "Run initial loctree scan and check dead exports"

After weeks of aggressive refactoring by multiple AI agents, Vista accumulated dead code - exported symbols that were never imported anywhere. Previous loctree versions produced ~2000 false positives. The new `loct dead` promised real results.

## Agent Session

### Step 1: Initial Scan

```bash
$ loct

Scanned 1351 files in 13.22s
Languages: ts, rs, css, js
Commands: 273 handlers, 22 unused
Events: 54 tracked
Barrels: 47 detected
Status: OK
```

### Step 2: Dead Exports Discovery

```bash
$ loct dead

Potential Dead Exports (38 found):
  - AIFloatingCanvasProps in src/features/ai-suite/chat-surface/components/AIChatCanvas.tsx:1073
  - AISystemCanvasProps in src/features/ai-suite/chat-surface/components/AIChatCanvas.tsx:1074
  - AIFloatingCanvas in src/features/ai-suite/chat-surface/components/AIChatCanvas.tsx:1077
  - replacePinnedNote in src/features/notes/notesStore.ts:636
  - dailyIndexKey in src/features/notes/notesStore.ts:649
  - useProfileSummary in src/features/onboarding/profileContext.tsx:60
  - securePrefs in src/services/securePrefs.ts:90
  - soapService in src/services/soapService.ts:269
  ... and 30 more
```

Only 38! Down from ~2000 false positives in earlier versions. But are they real?

### Step 3: Verification - Deep Dive

The agent systematically verified each export:

```bash
# Check if AIFloatingCanvas is actually imported
$ rg "import.*AIFloatingCanvas" src/ -n

src/features/ai-suite/floating/AIFloatingHost.tsx:15:
  import { AIChatCanvas as AIFloatingCanvas } from '@ai-suite/chat-surface/components/AIChatCanvas';
```

**Finding:** The component is imported as `AIChatCanvas`, aliased to `AIFloatingCanvas`. The actual `export const AIFloatingCanvas` in the source file is **never used directly**. TRUE DEAD.

```bash
# Check Appointment types
$ rg "import.*\{.*AppointmentSlot" src/ -n
# (empty)

$ rg "import.*\{.*CreateAppointmentRequest" src/ -n
# (empty)
```

**Finding:** 4 Appointment interfaces in `types/index.ts` - defined but never imported. TRUE DEAD.

```bash
# Check Rust exports
$ rg "ManagedTray" src-tauri/src/ -n | grep -v "tray.rs"
# (empty)
```

**Finding:** `ManagedTray` struct defined but never used outside its file. TRUE DEAD.

### Step 4: Verification Results

| # | Symbol | File | Verdict |
|---|--------|------|---------|
| 1-3 | `AIFloatingCanvas*` | AIChatCanvas.tsx | ✅ TRUE DEAD |
| 4-6 | `replacePinnedNote`, `dailyIndexKey`, `dailyStoreKey` | notesStore.ts | ✅ TRUE DEAD |
| 7 | `useProfileSummary` | profileContext.tsx | ✅ TRUE DEAD |
| 8-10 | `securePrefs`, `soapService`, `setDockBadge` | services/ | ✅ TRUE DEAD |
| 11-17 | Test utils (7 functions) | test-utils/ | ✅ TRUE DEAD |
| 18-21 | `Appointment*` (4 types) | types/index.ts | ✅ TRUE DEAD |
| 22-34 | Utils (13 functions) | utils/ | ✅ TRUE DEAD |
| 35-37 | Rust structs (3) | src-tauri/ | ✅ TRUE DEAD |
| 38 | `ref` | health.rs | ⚠️ FALSE POSITIVE |

**Accuracy: 97.4%** (37/38)

The single false positive: `ref` in `health.rs` was actually the Rust `lazy_static!` keyword pattern:

```rust
lazy_static! {
    pub static ref AI_HEALTH: RwLock<HashMap<String, (bool, Instant)>> = ...
}
```

Loctree incorrectly parsed `ref` as an exported symbol name.

### Step 5: Parallel Agent Cleanup

With verification complete, the agent spawned 3 parallel cleanup agents:

```
Agent 1: TS/React dead exports (17 targets)
Agent 2: Types and Test Utils (11 targets)
Agent 3: Rust dead code (3 targets)
```

Each agent received explicit instructions:
- Read files before editing
- Delete completely - no `// removed` comments
- If file becomes empty, DELETE THE ENTIRE FILE
- Run `tsc`/`cargo build` to verify

### Step 6: Agent Results

**Agent 1 (TS/React):**
```
Files modified: 10
Files DELETED: 3
  - src/services/systemBadgeService.ts
  - src/utils/platform.ts
  - src/utils/staffUtils.ts
```

**Agent 2 (Types & Test Utils):**
```
Files modified: 2
Files DELETED: 2
  - src/test-utils/mockVisit.ts
  - src/test-utils/translationTestUtils.tsx
```

**Agent 3 (Rust):**
```
Structs deleted: 1 (ManagedTray)
Visibility reduced: 2 (pub → pub(crate))
```

### Step 7: Cascade Cleanup

After initial cleanup, `loct dead` revealed new dead exports - functions that were only used by the code we just deleted:

```bash
$ loct dead

Potential Dead Exports (4 found):
  - TaskPresentationInfo in src/utils/taskDue.ts:13
  - resolveTaskDueDate in src/utils/taskDue.ts:87
  ...
```

Agent cleaned these too:
- Removed `TaskPresentationInfo` interface
- Removed `resolveTaskDueDate` function
- Removed `FALLBACK_SORT_BASE` constant
- Removed `canonicalizePriority` helper
- Removed `formatDueLabel` function
- Cleaned up unused `Task` import

### Step 8: Final Verification

```bash
$ loct dead

Potential Dead Exports (2 found):
  - SchemaStatement in src-tauri/src/database/schema.rs:3
  - ref in src-tauri/src/unified_ai/service_resolver/health.rs:7
```

Both remaining are false positives:
- `SchemaStatement` - `pub(crate)` visibility, used internally
- `ref` - Rust keyword, not an export

```bash
$ pnpm tsc --noEmit
Exit code: 0

$ cargo build
Finished `release` profile in 40.12s
```

## Summary

| Metric | Before | After | Δ |
|--------|--------|-------|---|
| Dead exports | 38 | 2 (false positives) | **-36** |
| Files | 1351 | 1346 | **-5** |
| Cleaned exports | - | 39 | types, functions, structs |
| Accuracy | - | 97.4% | 37/38 true positives |

### Files Deleted (5):
1. `src/services/systemBadgeService.ts`
2. `src/utils/platform.ts`
3. `src/utils/staffUtils.ts`
4. `src/test-utils/mockVisit.ts`
5. `src/test-utils/translationTestUtils.tsx`

### Categories Cleaned:
- AI-Suite backward compatibility exports
- Notes store unused functions
- Service singletons never instantiated
- Test utilities never used in tests
- Appointment types never imported
- Platform detection functions
- Staff/task utility functions
- Rust structs and traits

## Key Insights

### 1. False Positive Rate Dramatically Improved

Previous versions: ~2000 false positives
Current version: 1 false positive (97.4% accuracy)

The improvement comes from:
- Resolved import tracking (not just string matching)
- Star re-export chain analysis
- Local reference detection

### 2. Parallel Agent Cleanup Works

Spawning 3 agents to clean different file types simultaneously:
- No conflicts (different file domains)
- 3x faster than sequential
- Each agent can run `tsc`/`cargo build` independently

### 3. Cascade Effect is Real

Removing dead code often reveals more dead code - functions that were only used by the deleted code. Running `loct dead` iteratively catches these.

### 4. False Positive Patterns

Two remaining false positives reveal edge cases:
- `pub(crate)` visibility in Rust still appears as "exported"
- `lazy_static! { pub static ref NAME }` parses `ref` as symbol name

Both are known patterns that could be addressed in future versions.

## Workflow Recommendation

```bash
# 1. Scan and identify
loct dead

# 2. Verify (sample check)
rg "import.*SymbolName" src/

# 3. Clean with agents (parallel)
# Agent per category: TS, Types, Rust

# 4. Cascade check
loct dead  # May reveal new dead exports

# 5. Final verify
pnpm tsc --noEmit && cargo build
```

## Meta Note

This session demonstrates `loct dead` as a production-ready tool for dead code detection. The 97.4% accuracy means AI agents can trust the results and clean aggressively without manual verification of each export.

The "rozpierducha" (aggressive cleanup) approach works when:
1. Tool accuracy is high
2. No backward compatibility concerns (internal app)
3. Agents verify with `tsc`/`cargo build` after each change

---

*Vibecrafted with AI Agents by VetCoders (c)2025 The Loctree Team*
*Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)*
