# Use Case: sveltejs/svelte - The Svelte Compiler

**Repository**: https://github.com/sveltejs/svelte
**Stack**: TypeScript/JavaScript
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on THE Svelte compiler source - the core that powers the Svelte framework.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Files Scanned** | 405 |
| **Analysis Time** | ~30 seconds |
| **Throughput** | 13.5 files/sec |

## Findings

### Dead Code Detection
- **High Confidence**: 47 candidates
- **False Positive Rate**: 70%
- **True Positives**: 3-4 genuinely dead internal functions

### Confirmed Dead Code
- `ELEMENTS_WITHOUT_TEXT`
- `log_reactions`
- `not_equal`
- `bind_props`

### False Positive Categories
1. **TypeScript .d.ts re-exports** (60% of FPs) - easing functions, SSR functions
2. **Compiler-generated usage** (30% of FPs) - functions used by compiled `.svelte` output
3. **Dynamic references** (10% of FPs) - runtime lookups, reflection patterns

### Additional Findings
- **Dead Parrots**: 3,942 (TypeScript definition duplicates)
- **Circular Dependencies**: 15 (6 architectural, 9 test fixtures)

## Commands Used

```bash
cd svelte-core
loct                              # Create snapshot
loct dead --confidence high       # Dead code analysis
loct cycles                       # Circular dependencies
loct twins                        # Duplicate detection
```

## Verdict

**PASSED with caveats** - Excellent for internal dead code, needs `--library-mode` for public API.

## Key Insights

1. **Internal dead code**: 100% accurate (3/3 true positives)
2. **Public API exports**: 70% FP (expected for library codebases)
3. **Value delivered**: Identified 3-4 genuinely dead functions for cleanup

## Recommendations

- Use `--library-mode` flag when analyzing framework/library source
- Focus on internal modules, not public API barrel exports
- Cross-reference with `package.json` exports field

---

*Tested by M&K (c)2025 The Loctree Team*
