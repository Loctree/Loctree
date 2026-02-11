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

### Circular Dependencies - Architectural Analysis
```
✓ Found 91 circular import cycles (intra-crate)
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

**Manual Verification Result**: These are **NOT bugs** - they are normal Rust architecture:
- All cycles are **intra-crate** (`crate::` imports within same library)
- Rust allows modules within the same crate to mutually import each other
- Compiler resolves via lazy name resolution + type checking post-resolution
- This is fundamentally different from JavaScript/Python circular imports
- **No upstream issue warranted** - this is standard Rust module organization

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
3. **Cycle Detection**: Found 91 real architectural cycles (intra-crate, by design)
4. **Zero Crashes**: Complete analysis on 558 MB codebase

## Note on Intra-Crate Cycles

Unlike JavaScript/Python where circular imports can cause `undefined`/`None` at runtime, Rust's module system handles intra-crate cycles gracefully:

```rust
// This is VALID Rust - fmt imports cell, cell imports fmt
// crate::fmt::mod.rs
use crate::cell::Cell;

// crate::cell.rs
use crate::fmt::{Debug, Display};
```

Loctree correctly detects these architectural patterns but they should be interpreted as "module interdependencies" rather than "problematic cycles."

---

*Tested by M&K (c)2025 The Loctree Team*
