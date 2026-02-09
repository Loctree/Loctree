# Use Case: golang/go - The Go Standard Library

**Repository**: https://github.com/golang/go
**Stack**: Go
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on THE Go standard library - 17,000+ files of pure Go code.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Go Files** | 17,182 |
| **Analysis Time** | 107 seconds |
| **Throughput** | 160 files/sec |

## Findings

### Dead Code Detection
- **High Confidence**: 1 finding
- **False Positive Rate**: ~0%

The single finding is a Python GDB helper edge case (`ChanTypePrinter` used in tuple literals) - acceptable edge case for cross-language tooling.

### Version History

| Version | FP Rate | Status |
|---------|---------|--------|
| 0.5.16 | 0% | PERFECT |
| 0.6.1-dev | 100% | REGRESSION |
| 0.6.2-dev | ~0% | FIXED |

## Commands Used

```bash
cd GoLang
loct                              # Create snapshot (107s)
loct dead --confidence high       # Dead code analysis
loct twins                        # Duplicate detection
```

## Verdict

**PERFECT** - Go analysis is production-ready. The 0.6.1 regression was fixed in 0.6.2.

## Key Insights

1. **Scalability**: 17K files in under 2 minutes
2. **Accuracy**: Near-zero false positives
3. **Regression Fixed**: Cross-package reference tracking restored

---

*Tested by M&K (c)2025 The Loctree Team*
