# Use Case: AI Agent Fixes Circular Imports in AnythingLLM

> Real session transcript of Claude Code agent using loctree to diagnose and fix 6 circular import cycles in [AnythingLLM](https://github.com/Mintplex-Labs/anything-llm) repository.

## The Task

```
Your task: Fix 6 circular import cycles and prepare a PR to upstream.

STEP 1: Learn the tool - Run `loctree --help-full`
STEP 2: Diagnose - Run `loctree . -A --circular`
STEP 3: Fix each cycle
STEP 4: Validate with `yarn lint` and `yarn test`
STEP 5: Prepare PR
```

## Agent Session

### Step 1: Learning the Tool

```bash
$ loctree --help-full

loctree (Rust) - AI-oriented Project Analyzer - Full reference

Usage: loctree [root ...] [options]

Modes:
  init (default)            Scan and save snapshot to .loctree/snapshot.json
  slice <file>              Holographic slice: extract context for AI agents
  --analyze-imports, -A     Import/export analyzer mode
  --tree                    Directory tree view with LOC counts

Analyzer mode options (-A):
  --circular                Find circular imports (SCC analysis)
  --dead                    List potentially unused exports
  --sarif                   SARIF 2.1.0 output for CI integration
  ...
```

### Step 2: Initial Diagnosis

```bash
$ loctree . -A --circular

Circular imports detected (6 cycles):
  Cycle 1: embed/src/components/ChatWindow/ChatContainer -> .../ChatHistory
  Cycle 2: .../SlashPresets -> .../AddPresetModal -> .../EditPresetModal
  Cycle 3: .../LLMSelector/action -> .../LLMSelector
  Cycle 4: .../PromptInput -> .../SpeechToText
  Cycle 5: .../Admin/Users -> .../UserRow -> .../EditUserModal -> .../NewUserModal
  Cycle 6: .../CommunityHub/ImportItem/Steps -> ... (10-element cycle!)
```

### Step 3: Fixing Cycle by Cycle

#### Cycle 1: ChatContainer ↔ ChatHistory

**Problem:** `SEND_TEXT_EVENT` exported from parent, imported by child.

```javascript
// ChatContainer/index.jsx exports:
export const SEND_TEXT_EVENT = "anythingllm-embed-send-prompt";

// ChatHistory/index.jsx imports:
import { SEND_TEXT_EVENT } from "..";
```

**Fix:** Create `constants.js`:

```javascript
// ChatContainer/constants.js
export const SEND_TEXT_EVENT = "anythingllm-embed-send-prompt";
```

**Verify:**
```bash
$ loctree . -A --circular
Circular imports detected (5 cycles):  # ✅ Down from 6!
```

#### Cycle 2: SlashPresets ↔ Modals

**Problem:** `CMD_REGEX` exported from parent, imported by child modals.

**Fix:** Create `constants.js`, update imports.

```bash
$ loctree . -A --circular
Circular imports detected (4 cycles):  # ✅ Down from 5!
```

#### Cycles 3-6: Same Pattern

Each cycle followed the same pattern:
1. Identify shared constant/component causing the cycle
2. Extract to dedicated file (`constants.js`, `shared.js`, or `events.js`)
3. Update all imports
4. Verify with `loctree . -A --circular`

### Final Result

```bash
$ loctree . -A --circular
No circular imports detected.  # 🎉
```

## Key Insights

1. **Pattern Recognition**: Most circular imports are caused by child components importing constants/utilities from parent
2. **Simple Fix**: Extract shared code to dedicated files that don't import from the component tree
3. **Incremental Verification**: Running loctree after each fix confirms progress and catches regressions
4. **AI-Friendly Workflow**: Clear diagnostic output enables AI agents to work autonomously

## The PR

```markdown
## Summary
Resolves 6 circular import cycles detected by static analysis.

## Changes
- Extract shared constants to dedicated files
- Restructure component/config relationships
- No functional changes

## Testing
- [x] `yarn lint` passes
- [x] `yarn test` passes
- [x] `loctree . -A --circular` reports no cycles
```

---

*This use case demonstrates loctree's value for AI-assisted code maintenance. The clear, actionable output enables agents to work autonomously on codebase health tasks.*
