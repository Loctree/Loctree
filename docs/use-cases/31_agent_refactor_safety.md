# Use Case: Safe Refactor — Agent Removes a UI Tab Without Breaking Consumers

> How an AI agent used loctree to plan and execute a surgical removal of a UI component from a multi-module Rust+AppKit application.

**Context:** CodeScribe (macOS daemon, Rust + AppKit), removing Transcription tab from voice chat overlay
**Date:** 2026-02

## The Task

Remove the "Transcription" tab from a voice chat overlay (codename: Emil) and redirect non-assistive live preview to a separate ephemeral overlay. The voice chat overlay has multiple tabs (Agent, Settings, Transcription), shared state, and consumers across the app.

Removing a tab sounds simple. In practice:
- Multiple modules import from the voice chat API
- State fields, UI buttons, and handler functions reference the tab
- Re-exports propagate through several layers
- Tab bar indexing depends on order (removing slot 1 shifts everything)

## Step 1: Map Dependencies Before Touching Anything

```bash
loct --for-ai app/ui/voice_chat
loct find Transcription
loct find show_transcription_tab
loct find set_transcription_text
loct find opened_voice_chat_overlay_for_transcription
```

**What the agent learned:**
- `set_transcription_text` is exported from `voice_chat_ui` — needs to be redirected to `transcription_overlay`
- `opened_voice_chat_overlay_for_transcription` flag is used in multiple places (auto-hide logic, focus restoration)
- Tab bar positions are hardcoded: Agent=2.0, Settings=3.0 — removing Transcription=1.0 requires renumbering

## Step 2: Plan the Order of Operations

Based on the dependency map, the agent chose this sequence (not random — **informed by the graph**):

1. **Routing first** — redirect calls to `transcription_overlay` (create the target before removing the source)
2. **API cleanup** — remove transcription functions and re-exports from `voice_chat_ui` (this triggers most compile errors)
3. **State + UI + handlers** — remove tab/button/view and dead state fields
4. **Rename/flag cleanup** — remove or replace `opened_voice_chat_overlay_for_transcription`

Why this order matters: removing the API first would create a cascade of compile errors with no working target. Creating the redirect first means the codebase has a valid path at every step.

## Step 3: Safety Checks (Agent-Identified)

The agent flagged these risks **before coding**:

### Risk 1: Is `crate::set_transcription_text` already wired to the overlay?

```bash
loct find set_transcription_text
```

If the crate root doesn't have a re-export to `transcription_overlay`, the safest call is explicit:
```rust
crate::transcription_overlay::set_transcription_text(...)
```

### Risk 2: Auto-hide logic depends on the removed flag

```bash
loct find opened_voice_chat_overlay_for_transcription
```

The flag was used for both auto-hide scheduling and focus restoration. Simply setting it to `false` isn't enough — the agent identified this as a potential replacement with `transcription_overlay_open`.

### Risk 3: Tab bar math

Tab indices were hardcoded. Removing slot 1.0 without adjusting Agent (2.0→1.0) and Settings (3.0→2.0) would leave a visual gap or broken selection logic.

## Step 4: Post-Refactor Validation

```bash
loct twins                    # Catch dead exports left behind
loct health                   # Verify no new cycles introduced
cargo clippy -- -D warnings   # Zero warnings policy
cargo build --release         # Full build
```

### Manual validation checklist (agent-generated):
1. Non-assistive: Ctrl hold → overlay shows live deltas → formatted text after release → auto-hide
2. Assistive: Ctrl+Shift → Emil → Agent tab streams as before
3. Menu "Show Chat Overlay" → Emil has only Agent/Settings, no Transcription

## Key Insights

1. **Dependency-informed ordering** — The agent didn't just "start deleting." It chose an order that minimizes compile errors at each step, based on the actual dependency graph.

2. **Agent-identified risks** — Without `loct find`, the auto-hide flag dependency would have been discovered only after breaking the build. With it, the agent planned the mitigation upfront.

3. **Zero `#[allow(dead_code)]`** — The agent's explicit policy: don't add suppressions to make it compile. Remove dead code cleanly. `loct twins` enforces this.

4. **Every step is verifiable** — Snapshot before, snapshot after. The human can diff structural state, not just code diffs.

## The Pattern

For any "remove component X" task:

```bash
# 1. Map what X touches
loct find <component>
loct slice <module> --consumers

# 2. Plan removal order (target → API → state → cleanup)

# 3. Execute with intermediate checks
loct health   # after each major step

# 4. Final validation
loct twins    # no dead code left
loct health   # no new issues
```

---

*Extracted from production agent sessions. VibeCrafted with AI Agents (c)2026 VetCoders*
