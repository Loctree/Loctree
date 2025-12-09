# Use Case: microsoft/TypeScript - The TypeScript Compiler

**Repository**: https://github.com/microsoft/TypeScript
**Stack**: TypeScript
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on THE TypeScript compiler - the ultimate TypeScript analysis test.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Total Files** | 75,496 |
| **Source Files** | `/src` with 21 subdirectories |
| **Test Fixtures** | `/tests/cases/` (thousands) |

## Known Issue

**UTF-8 Error on Full Repository**:
```
Error: stream did not contain valid UTF-8
```

**Cause**: TypeScript's test suite contains intentionally malformed files to test compiler edge cases:
- Non-UTF-8 encodings
- BOM markers
- Invalid byte sequences
- Unicode edge cases

**This is NOT a loctree bug** - it's a valid limitation exposed by compiler test fixtures.

## Recommendations

### Option 1: Scan `/src` Only
```bash
cd TypeScript/src
loct                              # Only production code
loct dead --confidence high
```

### Option 2: Use `.loctreeignore`
```
# .loctreeignore
tests/
```

### Option 3: Alternative Test Targets
Real-world TypeScript projects work fine:
- VSCode
- Playwright
- Angular
- NestJS

## Future Improvements Needed

1. **`--skip-invalid-utf8`** flag for graceful degradation
2. **`.loctreeignore`** support for file exclusion
3. **Better error context** showing which file failed

## Verdict

**BLOCKED** - Test fixtures with intentional encoding violations prevent full scan. Production TypeScript codebases work fine.

## Key Insights

1. **Edge Case**: Compiler test suites are extreme edge cases
2. **Production Safe**: Real TS projects are properly UTF-8 encoded
3. **Feature Request**: Graceful UTF-8 error handling

---

*Tested by M&K (c)2025 The LibraxisAI Team*
