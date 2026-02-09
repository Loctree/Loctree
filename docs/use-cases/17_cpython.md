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
- **High Confidence**: 278 candidates (v0.6.1)
- **False Positive Rate**: 100% without library mode
- **With `--library-mode`**: Auto-detects stdlib, excludes `__all__` exports

### Why Library Mode Matters (v0.6.x)

Without library mode, all flagged exports are **public API** for external use:
- `calendar.APRIL` - Public API constant
- `csv.DictWriter` - Core utility class
- `ftplib.all_errors` - Used by urllib, socket, asyncio
- `typing.override` - Used across stdlib

**v0.6.x improvements**:
- Auto-detects `Lib/` directory as stdlib
- Respects `__all__` declarations in modules
- Use `--library-mode` for proper stdlib analysis

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

**LIBRARY MODE REQUIRED** - Use `--library-mode` for proper stdlib analysis (v0.6.x).

## Key Insights

1. **Performance**: Fast and stable (33 files/sec)
2. **Accuracy**: Requires library mode for public API analysis
3. **v0.6.x**: Auto-detection of stdlib, `__all__` tracking implemented

## Recommendations

For Python stdlib/library analysis:
- Use `--library-mode` flag for automatic `__all__` exclusion
- Auto-detects `Lib/` directory as Python stdlib
- `pyproject.toml` parsing for public API hints
- Focus on internal modules (`_internal/`, `_impl/`) for application code

---

*Tested by M&K (c)2025 The Loctree Team*
