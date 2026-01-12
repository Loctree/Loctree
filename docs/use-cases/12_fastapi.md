# Use Case: tiangolo/fastapi - The FastAPI Framework

**Repository**: https://github.com/tiangolo/fastapi
**Stack**: Python
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on FastAPI - the popular Python web framework with heavy decorator usage.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Python Files** | 1,184 |
| **Analysis Time** | 20.69 seconds |
| **Throughput** | 57 files/sec |

## Findings

### Dead Code Detection
- **High Confidence**: 0
- **False Positive Rate**: 0%

### Route Detection
```bash
loct routes    # 451 FastAPI routes detected!
```

Loctree correctly identifies all FastAPI route decorators:
- `@app.get()`, `@app.post()`, etc.
- `@router.get()`, `@router.post()`, etc.
- Nested routers and includes

### Version History

| Version | FP Rate | Status |
|---------|---------|--------|
| 0.5.16 | 100% | BROKEN (decorators) |
| 0.6.1-dev | 0% | FIXED |
| 0.6.2-dev (pre-fix) | CRASH | UTF-8 bug |
| 0.6.2-dev (post-fix) | 0% | PERFECT |

## Commands Used

```bash
cd FastAPI
loct                              # Create snapshot
loct dead --confidence high       # Dead code analysis
loct routes                       # Route detection (451 found!)
loct twins                        # Duplicate detection
```

## Verdict

**PERFECT** - FastAPI analysis is production-ready after UTF-8 fix.

## Key Insights

1. **Decorator Tracking**: `response_model=X` correctly parsed
2. **Route Detection**: 451 endpoints found automatically
3. **UTF-8 Fixed**: Emoji in strings no longer crash

## Special Feature: `loct routes`

Perfect for FastAPI projects:
```bash
loct routes              # List all endpoints
loct routes --json       # Export for documentation
loct routes --unused     # Find orphaned handlers
```

---

*Tested by M&K (c)2025 The LibraxisAI Team*
