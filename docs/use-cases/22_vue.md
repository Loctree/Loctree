# Use Case: vuejs/core - The Vue.js Framework

**Repository**: https://github.com/vuejs/core
**Stack**: TypeScript + Vue SFC
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on Vue.js core - validating Vue Single File Component (SFC) parsing.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Files Scanned** | ~500 |
| **Vue Files** | 11 |
| **Analysis Time** | ~10 seconds |

## Findings

### Dead Code Detection
- **High Confidence**: 0
- **False Positive Rate**: 0%

### Vue SFC Support
- **11/11 Vue files parsed correctly**
- Import extraction: Perfect
- Component imports: Recognized
- Type imports: Handled

### Version History

| Version | FP Rate | Status |
|---------|---------|--------|
| 0.5.16 | 86% | NO Vue SFC parser |
| 0.5.17 | 100% | WORSE |
| 0.6.1-dev | 0% | HUGE WIN |
| 0.6.2-dev | 0% | MAINTAINED |

## Commands Used

```bash
cd VueCore
loct                              # Create snapshot
loct dead --confidence high       # Dead code analysis
loct twins                        # Duplicate detection
```

## Verdict

**EXCELLENT** - Vue SFC parsing is production-ready.

## Key Insights

1. **SFC Parsing**: `<script>` and `<script setup>` blocks handled
2. **Template Bindings**: `{{ foo }}`, `v-bind`, `@click` tracked
3. **Zero Regression**: 0% FP maintained across versions

## Vue-Specific Features

Loctree correctly handles:
- `<script setup>` syntax
- `defineProps()`, `defineEmits()`
- Component auto-imports
- TypeScript in Vue files

---

*Tested by M&K (c)2025 The Loctree Team*
