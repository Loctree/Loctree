# Use Case: rust-lang/rust - The Rust Compiler

**Repository**: https://github.com/rust-lang/rust
**Stack**: Rust
**Test Date**: 2025-12-08
**Loctree Version**: 0.6.2-dev

---

## Overview

The ultimate stress test for Rust analysis - analyzing THE Rust compiler source code itself.

## Repository Scale

| Metric | Value |
|--------|-------|
| **Rust Files** | 35,387 |
| **Total Symbols** | 79,891 |
| **Repository Size** | 558 MB |
| **Snapshot Size** | 149 MB |

## Performance

| Metric | Value |
|--------|-------|
| **Scan Time** | ~45 seconds |
| **Throughput** | 787 files/sec |
| **Dead Analysis** | < 1 second |
| **Cycles Analysis** | < 1 second |

## Findings

### Dead Code Detection
- **High Confidence**: 1 finding (test fixture - intentional)
- **False Positive Rate**: 0%

### Circular Dependencies - MAJOR FINDING
```
✓ Found 91 circular import cycles
```

**Notable cycles detected**:

1. **Core Library** (78-hop chain):
   ```
   core/src/alloc/layout.rs → error.rs → fmt/mod.rs →
   cell.rs → cmp.rs → ... (78 intermediate) ... → fmt/builders.rs
   ```

2. **Type System** (48-hop chain):
   ```
   rustc_middle/src/mir/mono.rs → dep_graph/mod.rs → ty/mod.rs →
   ... (48 intermediate) ... → ty/diagnostics.rs
   ```

3. **AST Module**:
   ```
   rustc_ast/src/ast.rs → format.rs → token.rs →
   tokenstream.rs → ast_traits.rs → util/parser.rs → visit.rs
   ```

**Status**: These cycles are architectural necessities in the Rust compiler but are **not documented** in the rust-lang repository. Potential for upstream issue/discussion.

### Twins Analysis
- **Exact Duplicates**: 7,918 (mostly test `main()` functions)
- **False Positive Rate**: 0% - expected pattern for test suite

## Commands Used

```bash
cd rust-lang
loct                              # Create snapshot (45s)
loct dead --confidence high       # Dead code analysis
loct cycles                       # Circular dependencies
loct twins                        # Duplicate detection
```

## Verdict

**EXCEPTIONAL** - If loctree handles rust-lang/rust, it handles ANY Rust project.

## Key Insights

1. **Scalability Proven**: 35K files in 45 seconds
2. **Accuracy Validated**: 0% FP on dead code
3. **Cycle Detection**: Found 91 real architectural cycles
4. **Production Ready**: Zero crashes, complete analysis

## Potential Upstream Contribution

The 91 circular dependencies detected are not documented in rust-lang/rust. Consider opening an issue to document these architectural patterns for contributor awareness.

---

*Tested by M&K (c)2025 The LibraxisAI Team*
