# Use Case: facebook/react - The React Library

**Repository**: https://github.com/facebook/react
**Stack**: TypeScript/JavaScript (JSX/TSX)
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on THE React library source - the definitive test for JSX/TSX analysis.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Files Scanned** | 3,951 |
| **Analysis Time** | 49 seconds |
| **Throughput** | 81 files/sec |
| **Artifacts Generated** | 33 MB |

## Performance

| Metric | Value |
|--------|-------|
| **Scan Time** | 49.02 seconds |
| **Dead Exports Found** | 676 |
| **Dead Parrots** | 1,684 |
| **Circular Dependencies** | 8 cycles |

## Findings

### Dead Code Detection
- **High Confidence**: 676 candidates
- **False Positive Rate**: ~20%
- **True Positives**: Compiler internals, experimental features, legacy dev tools

### False Positive Categories (v0.6.x Improvements)
1. **Import aliasing** (`import {X as Y}`) - ~10-15% (improved tracking)
2. **Type-only exports** in .js files - ~5% (better Flow type detection)
3. **Global registries** (WeakMap/Set patterns) - ~5% (registry pattern detection added)

### Circular Dependencies
```
✓ Found 8 circular import cycles
```

Notable findings:
1. **Massive compiler cycle**: 115 files in babel-plugin-react-compiler
2. **DevTools store** ↔ cache co-dependency
3. **DOM event system** ↔ priority scheduler
4. **Reconciler lanes** ↔ DevTools hooks

### True Dead Code Found
- babel-plugin-react-compiler internal utilities (~200 exports)
- Experimental ReactServer features (Activity, ViewTransition)
- Dev-only tools (console patching, debug prints)
- Unused type helpers in Flow integration

## Commands Used

```bash
cd react
loct                              # Create snapshot
loct dead --confidence high       # Dead code analysis
loct cycles                       # Circular dependencies
loct twins                        # Duplicate detection
```

## Verdict

**A- PASSED** - Production-ready with significantly improved dead code accuracy (v0.6.x).

## Grade Card

| Category | Grade | v0.6.x Improvements |
|----------|-------|---------------------|
| Performance | A+ | No change |
| JSX/TSX Support | A | WeakMap/WeakSet patterns |
| Dead Code Accuracy | B+ | 40% → 20% FP rate |
| Cycle Detection | A | No change |
| Scalability | A+ | No change |

## Recommended Workflow

```bash
loct dead --confidence high > candidates.txt
# Verify top 50 with ripgrep
# Delete exports with 0 matches (safe)
# Review exports with only type matches
```

---

*Tested by M&K (c)2025 The LibraxisAI Team*
