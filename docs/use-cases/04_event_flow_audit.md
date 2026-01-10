# Use Case: AI Agent Audits Event Flow and Finds Orphan Bridges

> Real session transcript of Claude agent using `loct events` to audit audio/transcription event flow — discovering orphan emitters and missing listeners in 2 seconds.

**Project:** Vista (Tauri: Rust + React/TypeScript)
**Loct version:** 0.6.x
**Date:** 2025-12-13
**Task:** Deep audit of audio recording and transcription flow for race conditions and missing event handlers

## The Challenge

The agent was tasked with auditing the entire audio/transcription pipeline for:
- Race conditions between concurrent operations
- Missing event handlers (orphan emitters/listeners)
- Memory leaks and cleanup issues
- PII exposure risks

Without loctree, auditing event flow would require:
- Manual `rg` searches for `emit(` and `listen` patterns
- Cross-referencing Rust emitters with TypeScript listeners
- Building mental model of which events are actually connected
- No visibility into orphan events (emitted but never listened to)

## Session Transcript

### Step 1: Quick Event Bridge Audit

```bash
$ loct events
✓ Found 52 event bridge(s)
Event Bridges Analysis:

Found 52 event bridge(s):

  Event: transcription_completed
    Emit locations (3):
      src-tauri/src/engines/recordings/services/visit.rs:379 [emit_literal]
      src-tauri/src/engines/recordings/services/visit.rs:392 [emit_literal]
      src-tauri/src/engines/recordings/services/visit.rs:512 [emit_literal]
    Listen locations (4):
      src/features/ai-suite/hooks/useAiSuggestions.ts:23
      src/hooks/visits/useTranscriptionStatus.ts:52
      src/hooks/visits/useVisitsList.ts:401
      ... and 1 more

  Event: audio_recording_transcription
    Emit locations (1):
      src-tauri/src/engines/recordings/services/visit.rs:510 [emit_literal]
    ⚠️  No listeners found (orphan emitter?)
```

**Time:** 2 seconds

**Insight:** Immediately visible:
- `transcription_completed` is well-connected (3 emitters → 4 listeners)
- `audio_recording_transcription` is **orphan** — emitted but never listened to!

### Step 2: Find All Orphan Emitters

```bash
$ loct events | grep -B2 "No listeners found"

  Event: ready
    Emit locations (2):
      src/hooks/audio/pipeline/AudioCaptureCoordinator.ts:55
      src/hooks/audio/pipeline/AudioCaptureCoordinator.ts:93
    ⚠️  No listeners found (orphan emitter?)

  Event: tray-quick-patient
    Emit locations (1):
      src-tauri/src/app/tray.rs:127
    ⚠️  No listeners found (orphan emitter?)

  Event: audio_recording_transcription
    Emit locations (1):
      src-tauri/src/engines/recordings/services/visit.rs:510
    ⚠️  No listeners found (orphan emitter?)

  ... (24 total orphan emitters found)
```

**Time:** 3 seconds

**Insight:** The codebase has **24 orphan emitters** — events being emitted but never listened to. This is significant tech debt and potential dead code.

### Step 3: Analyze Specific Orphan Event

Looking at `audio_recording_transcription` in context:

```bash
$ loct slice src-tauri/src/engines/recordings/services/visit.rs --json | jq '.eventBridges'
[
  "audio_recording_transcription",
  "transcription_completed",
  "transcription_error"
]
```

Cross-referencing the Rust code (line 510-512):
```rust
let _ = app_handle.emit("audio_recording_transcription", payload);
if let Some(visit_id) = visit_id {
    let _ = app_handle.emit("transcription_completed", visit_id);
}
```

**Insight:** `audio_recording_transcription` carries richer payload:
```json
{
  "recordingId": "...",
  "visitId": "...",
  "status": "...",
  "transcriptPreview": "first 280 chars..."
}
```

While `transcription_completed` only sends `visit_id`. The orphan event has valuable data that's being thrown away!

## Resolution

Based on loctree's findings, the agent:

### 1. Added Missing Listener

```typescript
// useTranscriptionStatus.ts - NEW listener
const unlistenRecordingTranscription = await listenToTauriEvent<{
  recordingId: string;
  visitId: string | null;
  status: string;
  transcriptPreview: string;
}>('audio_recording_transcription', (event) => {
  if (event.payload.visitId === visitId) {
    setStatus((prev) => ({
      ...prev,
      transcriptPreview: event.payload.transcriptPreview,
      recordingId: event.payload.recordingId,
    }));
  }
});
```

### 2. Extended Interface

```typescript
export interface TranscriptionStatus {
  isGenerating: boolean;
  hasTranscription: boolean;
  error: string | null;
  // NEW: from audio_recording_transcription event
  transcriptPreview?: string;
  recordingId?: string;
}
```

### Verification

```bash
$ loct events | grep "audio_recording_transcription"

  Event: audio_recording_transcription
    Emit locations (1):
      src-tauri/src/engines/recordings/services/visit.rs:510 [emit_literal]
    Listen locations (1):
      src/hooks/visits/useTranscriptionStatus.ts:88
```

No more orphan warning!

## Time Comparison: Loctree vs Grep

| Task | Loctree | Grep/Ripgrep |
|------|---------|--------------|
| Find all event emitters | **2s** | ~5 min (`rg "emit\(" --type rust` + manual parsing) |
| Find all event listeners | **included** | ~5 min (`rg "listen.*Event" --type ts` + manual parsing) |
| Cross-reference emitters↔listeners | **automatic** | ~20-30 min (manual matching of event names) |
| Identify orphan events | **automatic** | ~30-40 min (diff of emit vs listen lists) |
| Get emit/listen locations | **included** | ~10 min (manual `rg -n` for each event) |
| **Total analysis time** | **~5s** | **~60-80 min** |

**Speedup: ~800x for event flow audit**

### Why Grep Falls Short for Event Analysis

1. **No cross-language correlation** — Grep can't connect Rust `app_handle.emit("foo")` with TypeScript `listenToTauriEvent('foo')`

2. **No orphan detection** — You'd need to:
   ```bash
   # Extract Rust emitters
   rg 'emit\("([^"]+)"' src-tauri/ -o | sort -u > /tmp/emits.txt

   # Extract TS listeners
   rg "listen.*Event.*['\"]([^'\"]+)['\"]" src/ -o | sort -u > /tmp/listens.txt

   # Manual diff
   comm -23 /tmp/emits.txt /tmp/listens.txt
   ```
   Even then, regex patterns miss edge cases (string interpolation, constants, etc.)

3. **No payload visibility** — Grep finds the event name but not what data it carries

4. **No location context** — Finding an orphan event doesn't tell you where to add the listener

## Related Findings from Same Session

Using `loct events` as entry point led to broader discoveries:

| Finding | Source | Impact |
|---------|--------|--------|
| 24 orphan emitters | `loct events` | Tech debt backlog |
| Race condition in transcript store | Code review (informed by event flow) | P0 CRITICAL fix |
| Missing app close handler | Code review | P1 HIGH fix |
| Memory leak in setTimeout | Code review | P1 HIGH fix |

The event audit opened the door to a comprehensive pipeline review that found **1 critical** and **5 high-priority** issues.

## Key Takeaways

### For AI Agents

1. **Start audits with `loct events`** — Instantly reveals orphan bridges and event flow issues
2. **Use `loct slice --json`** to get event bridges for specific files
3. **Cross-reference emitter payload** with listener expectations to find data mismatches
4. **Orphan emitters are candidates for either:**
   - Adding missing listener (if data is valuable)
   - Removing dead emit (if truly unused)

### For Humans

1. **Event flow is invisible to grep** — Loctree makes FE↔BE event correlation trivial
2. **Orphan detection prevents silent failures** — Events emitted to void are often bugs
3. **24 orphans in production codebase** — Common problem that's hard to find manually

## Conclusion

`loct events` transformed a 60-80 minute manual audit into a 5-second graph query. The agent received:

- Complete emit↔listen correlation across Rust/TypeScript boundary
- Automatic orphan detection with confidence warnings
- Exact file:line locations for all bridges
- Foundation for deeper pipeline audit

This enabled the agent to immediately identify `audio_recording_transcription` as an orphan event with valuable data being thrown away — a finding that would have taken 30+ minutes to discover with grep, if found at all.

---

*This use case demonstrates loctree's value for cross-language event flow analysis. Graph-aware event correlation enables agents to audit FE↔BE bridges in seconds, revealing orphan events that represent either bugs or tech debt.*
