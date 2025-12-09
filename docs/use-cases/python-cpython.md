# Use Case: python/cpython - The Python Interpreter

**Repository**: https://github.com/python/cpython
**Stack**: Python (+ C)
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on THE Python interpreter source - the reference implementation of Python.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Python Files** | 842 (stdlib) |
| **Analysis Time** | 25.09 seconds |
| **Throughput** | 33 files/sec |

## Findings

### Dead Code Detection
- **High Confidence**: 278 candidates
- **False Positive Rate**: 100% (expected for stdlib)

### Why 100% FP is Expected

All flagged exports are **public API** for external use:
- `calendar.APRIL` - Public API constant
- `csv.DictWriter` - Core utility class
- `ftplib.all_errors` - Used by urllib, socket, asyncio
- `typing.override` - Used across stdlib

**These are NOT dead** - they're exported for millions of Python programs!

### Additional Findings
- **Dead Parrots**: 172 classes/functions
- **Circular Dependencies**: 2 lazy cycles (both safe)

## Known Issue

**Unicode Bug Found**: Devanagari numerals (like `६`) in test files caused panic at `py.rs:224:32`. Test directory excluded to complete scan.

**Status**: Fixed in subsequent patch using `bytes_match_keyword`.

## Commands Used

```bash
cd cpython
loct                              # Create snapshot
loct dead --confidence high       # Dead code analysis
loct cycles                       # Circular dependencies
```

## Verdict

**NOT SUITABLE for stdlib analysis** - 100% FP expected for public library APIs.

## Key Insights

1. **Performance**: Fast and stable (33 files/sec)
2. **Accuracy**: Not designed for public library APIs
3. **Need**: `--library-mode` flag essential for stdlib/framework code

## Recommendations

For Python stdlib/library analysis:
- Implement `--library-mode` to exclude `__all__` exports
- Parse `setup.py`/`pyproject.toml` for public API hints
- Focus on internal modules (`_internal/`, `_impl/`)

---

*Tested by M&K (c)2025 The LibraxisAI Team*
