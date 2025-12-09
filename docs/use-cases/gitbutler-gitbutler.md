# Use Case: gitbutlerapp/gitbutler - Git Client

**Repository**: https://github.com/gitbutlerapp/gitbutler
**Stack**: Rust + Svelte
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on GitButler - a modern Git client built with Tauri (Rust + Svelte).

## Repository Scale

| Metric | Value |
|--------|-------|
| **Files Scanned** | 1,726 |
| **Analysis Time** | 21.27 seconds |
| **Throughput** | 81 files/sec |

## Findings

### Dead Code Detection
- **High Confidence**: 111 candidates
- **False Positive Rate**: 25-33%

### Known Regression

Svelte component method references not detected:
```svelte
<!-- ChatInput.svelte -->
<script>
export function focusInput() {
    richText.richTextEditor?.focus();
}
</script>

<!-- Usage in parent - NOT DETECTED -->
<ChatInput bind:this={chatInput} />
{chatInput?.focusInput()}
```

### Version History

| Version | FP Rate | Status |
|---------|---------|--------|
| 0.5.16 | 40-50% | Default imports broken |
| 0.5.17 | 8.4% | Major improvement |
| 0.6.1-dev | 0% | PERFECT |
| 0.6.2-dev | 25-33% | REGRESSION |

## Commands Used

```bash
cd GitButler
loct                              # Create snapshot
loct dead --confidence high       # Dead code analysis
loct twins                        # Duplicate detection
```

## Verdict

**REGRESSION** - Svelte component method refs need fixing.

## Key Insights

1. **Svelte Components**: Method refs via `bind:this` not tracked
2. **Rust Analysis**: Works well
3. **Tauri Integration**: Command bridges functional

## Workaround

Until fixed, manually verify Svelte component exports:
```bash
loct dead --confidence high | grep ".svelte" | while read line; do
    symbol=$(echo "$line" | awk -F: '{print $NF}')
    rg "\\.$symbol\\(" . --type svelte
done
```

---

*Tested by M&K (c)2025 The LibraxisAI Team*
