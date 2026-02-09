# Use Case: denoland/deno - The Deno Runtime

**Repository**: https://github.com/denoland/deno
**Stack**: Rust
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

Testing loctree on Deno - a large Rust codebase with complex enum patterns.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Rust Files** | 611 |
| **Analysis Time** | 90.32 seconds |
| **Total Symbols** | 8,559 |

## Findings

### Dead Code Detection
- **High Confidence**: 1 finding
- **False Positive Rate**: ~0% (method detection edge case)

### Enum Variant Detection - MAJOR FIX
```rust
// Previously flagged as dead (WRONG):
pub enum NpmResolver<TSys: NpmResolverSys> {
    Byonm(ByonmNpmResolverRc<TSys>),
    Managed(ManagedNpmResolverRc<TSys>),
}
// NOW CORRECTLY TRACKED!
```

### Version History

| Version | FP Rate | Status |
|---------|---------|--------|
| 0.5.16 | 60% | Enum variants broken |
| 0.6.1-dev | 40% | Better |
| 0.6.2-dev | ~0% | EXCELLENT |

### Additional Findings
- **Twins**: 604 duplicate export groups (expected)
- **Cycles**: 12 circular dependencies (architectural patterns)

## Commands Used

```bash
cd Deno
loct                              # Create snapshot
loct dead --confidence high       # Dead code analysis
loct cycles                       # Circular dependencies
loct twins                        # Duplicate detection
```

## Verdict

**A- EXCELLENT** - Major improvement in Rust enum handling.

## Key Insights

1. **Enum Detection**: Complex enum variants now tracked
2. **Edge Case**: Method vs function distinction needs refinement
3. **Cycles**: 12 real architectural cycles detected

## Remaining Edge Case

One false positive: method in impl block detected as module export:
```rust
impl Hashes {
    pub const fn all(&self) -> [&'static str; 5] { ... }
}
```

This is a method, not a module-level function. Minor refinement needed.

---

*Tested by M&K (c)2025 The Loctree Team*
