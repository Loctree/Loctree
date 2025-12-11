# Use Case: nodejs/node - The Node.js Runtime

**Repository**: https://github.com/nodejs/node
**Stack**: JavaScript + C++
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on THE Node.js runtime - massive hybrid codebase.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Total Files** | ~20,400 JS files |
| **lib/ Only** | 348 files |
| **Analysis Time** | 0.13s (lib/ only) |

## Known Issue

**UTF-8 Error on Full Repository**:
```
Error: stream did not contain valid UTF-8
```

**Cause**: Binary ICU Unicode data files (`icudt77l.dat.bz2`)

**Workaround**: Test `lib/` directory only.

## Findings (lib/ Directory)

### Dead Code Detection
- **High Confidence**: 1 finding
- **False Positive Rate**: 0%

The single finding (`eslint.config_partial.mjs` default export) is a true positive within the isolated scope.

### Architecture
- **Circular Imports**: 0 (perfect architecture!)
- Node.js core lib has excellent module structure

## Commands Used

```bash
cd nodejs/lib
loct                              # Create snapshot (0.13s!)
loct dead --confidence high       # Dead code analysis
loct cycles                       # Circular dependencies
```

## Verdict

**INCONCLUSIVE** - Binary file handling needs improvement.

## Key Insights

1. **Performance**: Lightning fast on pure JS (0.13s for 348 files)
2. **Accuracy**: 0% FP within scope
3. **Blocker**: Binary file detection needed

## Recommendations

For Node.js-style repos:
1. Add `.loctreeignore` for binary directories
2. Implement graceful UTF-8 error recovery
3. Auto-detect and skip binary files

---

*Tested by M&K (c)2025 The LibraxisAI Team*
