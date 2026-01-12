# Use Case: AI Agent Implements Feature with Graph-Aware Navigation

> Real session transcript of Claude agent using loctree to implement 6 Tauri command handlers and plan integration — demonstrating 60x speedup over grep-based workflows.

**Project:** Vista (Tauri: Rust + React/TypeScript)
**Loct version:** 0.5.9
**Date:** 2025-12-01
**Task:** Implement recording pipeline handlers and assess bootstrap integration

## The Challenge

The agent received a task briefing to implement 6 Tauri command handlers for a recording engine. The requirements included:

1. Implement handlers wrapping `RecordingEngine` core API
2. Register them in `lib.rs`
3. Assess feasibility of adding pipeline health check to `post_auth_bootstrap`

Without loctree, this would require:
- Manually searching for existing patterns to follow
- Grepping through 1500+ files to understand the architecture
- Building mental model of module dependencies

## Session Transcript

### Step 1: Verify FE↔BE Contract Status

```bash
$ loct commands --missing

Tauri command coverage:
  Missing handlers (frontend calls without backend):
    save_libraxis_api_key, load_llm_endpoint, load_stt_endpoint,
    load_libraxis_api_key, load_tts_endpoint
  Unused handlers (HIGH confidence):
    validate_veterinary_medication, transcription_publish_partial,
    cancel_invitation, get_pending_invitations, voice_get_metrics...
```

**Insight:** The new `recording_*` commands weren't in the missing list yet because frontend wasn't calling them. This confirmed we're adding new functionality, not fixing broken contracts.

**Time:** 2 seconds

### Step 2: Find Symbol Definition Location

```bash
$ loct query where-symbol post_auth_bootstrap
where-symbol 'post_auth_bootstrap':
  (no results)
```

Snapshot was stale (different branch). Quick rescan:

```bash
$ loct --full-scan
Scanned 1353 files in 10.08s
Commands: 289 handlers, 38 unused
```

**Time:** 10 seconds (full rescan)

### Step 3: Get Full Context with Slice

```bash
$ loct slice src-tauri/src/bootstrap/post_auth.rs --consumers --json
```

```json
{
  "commandBridges": ["post_auth_bootstrap"],
  "consumers": [
    {"path": "src-tauri/src/lib.rs", "loc": 571, "depth": 1}
  ],
  "deps": [
    {"path": "src-tauri/src/session.rs", "loc": 297, "depth": 1},
    {"path": "src-tauri/src/utils/secure_logger.rs", "loc": 171, "depth": 2}
  ],
  "eventBridges": [
    "post-auth-bootstrap:summary",
    "post-auth-bootstrap:progress"
  ],
  "stats": {
    "total_files": 4,
    "total_loc": 1616
  }
}
```

**Insight:** In 3 seconds, the agent received:
- Complete dependency graph (who this file imports)
- Consumer graph (who imports this file)
- Event bridge information (what Tauri events it emits)
- Exact LOC counts for context sizing

**Time:** 3 seconds

### Step 4: Find Existing Patterns

```bash
$ loct find RecordingEngine

=== Semantic Matches (20) ===
  RecordingEngine (score: 1.00)
    in export in src-tauri/src/engines/recordings/engine_core.rs
  RecordingEngine (score: 1.00)
    in export in src-tauri/src/engines/recordings/mod.rs
  RecordingEvent (score: 0.73)
    in export in src/services/audio/engineBridge.ts
  RecordingEngineConfig (score: 0.71)
    in export in src-tauri/src/engines/recordings/engine_core.rs
  ...

=== Dead Code Status ===
  WARNING: Symbol appears to be dead code in:
    - src-tauri/src/engines/recordings/engine_core.rs
```

**Insight:** The fuzzy search found related symbols (RecordingEvent, RecordingEngineConfig) AND flagged that `RecordingEngine` was dead code — because no handlers were using it yet. This confirmed the implementation task was needed.

**Time:** 2 seconds

## Implementation (Guided by Loctree Context)

With the slice output, the agent knew exactly:
- Where handlers should be added (following `transcription_*` pattern in `engine.rs`)
- Where to register them (`lib.rs` line ~370)
- What the bootstrap step pattern looks like (from `post_auth.rs` slice)

### Handlers Implemented

```rust
// 6 new handlers following existing transcription_* pattern
// mapping between Tauri commands and RecordingEngine methods:
// recording_start_session    -> RecordingEngine::start_session()
// recording_push_chunk       -> RecordingEngine::push_chunk()
// recording_finalize_session -> RecordingEngine::finalize_session()
// recording_abort_session    -> RecordingEngine::abort_session()
// recording_session_status   -> RecordingEngine::session_status()
// recording_active_sessions  -> RecordingEngine::active_sessions()
```

### Verification

```bash
$ cargo check
warning: field `metadata` is never read  # Expected - from engine_core.rs
    Finished `dev` profile in 13.41s

$ cargo clippy
    Finished `dev` profile in 14.90s  # No new warnings
```

## Time Comparison: Loctree vs Grep

| Task | Loctree | Grep/Ripgrep |
|------|---------|--------------|
| FE↔BE coverage check | **2s** | ~15-20 min (manual diff of `rg "tauri::command"` vs `rg "invoke\("`) |
| Find symbol + dead code status | **2s** | ~30s (exact match only, no dead code info) |
| Full context slice with deps/consumers | **3s** | ~10-15 min (recursive `rg "use.*module"` + mental graph) |
| Understand bootstrap pattern | **3s** (via slice) | ~5-10 min (read file + trace imports) |
| **Total analysis time** | **~15s** | **~30-40 min** |

**Speedup: ~120x for analysis phase**

### Why Grep Falls Short

1. **No graph awareness** — `rg "RecordingEngine"` finds occurrences but doesn't know:
   - Is it dead code?
   - What imports it?
   - What does it import?

2. **No fuzzy matching** — Searching for `RecordingEngine` won't find `RecordingEngineConfig` or `RecordingEvent` without separate queries

3. **No contract verification** — Checking FE↔BE coverage manually requires:
   ```bash
   # Extract BE handlers
   rg '#\[tauri::command' src-tauri/ -A2 | grep 'fn ' > /tmp/handlers.txt

   # Extract FE invokes
   rg "invoke\(['\"]" src/ -o | sort -u > /tmp/invokes.txt

   # Manual diff and parsing...
   ```

   Even then, you don't get confidence levels (is handler truly unused or dynamically invoked?)

4. **No context sizing** — Grep can't tell you "this slice is 1616 LOC across 4 files" to help with context window management

## Assessment: Bootstrap Integration

From the slice output, the agent determined:

**Effort: LOW (~30-45 min)**

The `post_auth.rs` file already has:
- Step infrastructure (`run_step()` function)
- Critical vs soft step classification
- Event emission for progress tracking

Adding `"audio.pipeline_check"` step requires:
1. ~30 LOC function in `health.rs`
2. ~10 LOC step registration in `post_auth.rs`

```rust
// Proposed step (informed by slice context)
// steps.push(
//     run_step(app_handle, "audio.pipeline_check", "Verify audio pipeline", {
//         let handle = app_handle.clone();
//         move || async move {
//             health::test_audio_pipeline(&handle).await?;
//             Ok(StepOutcome::Completed(Some("pipeline OK".into())))
//         }
//     })
//     .await?,
// );
```

## Key Takeaways

### For AI Agents

1. **Start with `loct commands --missing`** to understand FE↔BE contract state
2. **Use `loct slice --consumers --json`** to get full context before implementation
3. **Use `loct find`** to discover related symbols and dead code status
4. **Trust the graph** — loctree's import graph is more reliable than grepping

### For Humans

1. **AI agents work faster with structured context** — JSON slice output is directly consumable
2. **Dead code detection prevents wasted effort** — agent knew `RecordingEngine` was unused before starting
3. **Contract verification catches issues early** — missing handlers visible before runtime errors

## Conclusion

Loctree transformed a 40+ minute grep-based analysis into a 15-second graph query workflow. The agent received:

- Complete module dependency graph
- Dead code status
- FE↔BE contract verification
- Event bridge mapping
- Exact LOC counts for context management

This enabled confident implementation of 6 handlers + accurate assessment of bootstrap integration effort — all verified with `cargo check && cargo clippy` passing on first try.

---

*This use case demonstrates loctree's value for AI-assisted feature implementation. Graph-aware navigation enables agents to understand architecture before writing code, reducing iteration cycles and improving first-attempt success rates.*
