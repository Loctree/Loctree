# Use Case: Dogfooding Loctree - AI Agent Fixes Its Own False Positive

> Real session transcript of Claude Code agent running loctree on its own codebase, discovering a false positive in cycle detection, and implementing the fix.

**Project:** loctree (Rust CLI)
**Loct version:** 0.5.9-dev
**Date:** 2025-12-01
**Task:** Dogfood loctree, review findings, fix discovered issues

## The Challenge

```
User: "lekarzu lecz się sam - wykonaj `loct` na naszym repo z root'a i przejrzyj findings"
```

Translation: "Physician, heal thyself - run `loct` on our repo from root and review findings"

## Agent Session

### Step 1: Running Loctree on Itself

```bash
$ ./loctree_rs/target/release/loctree loctree_rs/src -A --ext rs --circular

Circular imports detected (2 cycles):
  Cycle 1: tests/fixtures/js-cyclic-deep/a.js -> b.js -> c.js -> a.js
  Cycle 2: analyzer/report.rs -> snapshot.rs -> analyzer/root_scan.rs ->
           analyzer/runner.rs -> analyzer/output.rs -> analyzer/sarif.rs
```

### Step 2: Investigating the Cycles

**Cycle 1**: Test fixture - intentional, used to test cycle detection itself. ✅

**Cycle 2**: Suspicious. Let me trace the imports in `report.rs`:

```bash
$ grep -n "use crate::" loctree_rs/src/analyzer/report.rs

# No crate imports in production code!
# But wait...

$ grep -n "snapshot" loctree_rs/src/analyzer/report.rs
153:    use crate::snapshot::CommandBridge;
```

Line 153 is inside `#[cfg(test)] mod tests { ... }` - a **test-only import**!

### Step 3: Root Cause Analysis

```rust
// loctree_rs/src/analyzer/report.rs

use serde::Serialize;  // Only external import

pub struct CommandGap { ... }
pub struct AiInsight { ... }
// ... pure type definitions ...

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::CommandBridge;  // ← FALSE POSITIVE SOURCE!

    #[test]
    fn my_test() { ... }
}
```

The Rust analyzer was including imports from `#[cfg(test)]` modules in cycle detection, creating false positives for test-only dependencies.

### Step 4: Implementing the Fix

Added `strip_cfg_test_modules()` function to filter out test modules before extracting imports:

```rust
/// Strip `#[cfg(test)]` annotated modules from content to avoid false positive cycles.
fn strip_cfg_test_modules(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut in_cfg_test_attr = false;

    while let Some(ch) = chars.next() {
        // Look for #[cfg(test)]
        if ch == '#' && chars.peek() == Some(&'[') {
            // ... collect attribute ...

            if attr_inner.starts_with("#[cfg(test)")
                || attr_inner.starts_with("#[cfg(all(") && attr_inner.contains("test")
            {
                in_cfg_test_attr = true;
                result.truncate(pos);  // Remove the attribute
            }
            continue;
        }

        // If after #[cfg(test)], look for `mod` and skip the entire block
        if in_cfg_test_attr && ch == 'm' {
            // ... detect "mod" keyword ...
            // ... skip balanced braces ...
        }

        result.push(ch);
    }
    result
}
```

Modified `analyze_rust_file()` to use filtered content:

```rust
pub(crate) fn analyze_rust_file(content: &str, relative: String, ...) -> FileAnalysis {
    // Strip #[cfg(test)] modules to avoid false positive cycles
    let production_content = strip_cfg_test_modules(content);

    for caps in regex_rust_use().captures_iter(&production_content) {
        // ... extract imports from production code only ...
    }
}
```

### Step 5: Adding Tests

```rust
#[test]
fn strip_cfg_test_excludes_test_imports() {
    let content = r#"
use serde::Serialize;

pub struct MyType { pub name: String }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::CommandBridge;  // Should be excluded!
}
"#;
    let analysis = analyze_rust_file(content, "report.rs".to_string(), &[]);

    assert_eq!(analysis.imports.len(), 1);  // Only serde
    assert!(!analysis.imports.iter().any(|i| i.source.contains("snapshot")));
}

#[test]
fn strip_cfg_test_preserves_non_test_cfg() {
    // #[cfg(target_os = "macos")] modules should still be included
}

#[test]
fn strip_cfg_test_handles_nested_blocks() {
    // Properly handles nested {} braces inside test modules
}
```

### Step 6: Validation

```bash
$ cargo clippy -- -D warnings
    Finished `dev` profile in 1.33s

$ cargo test "analyzer::rust"
running 10 tests
test analyzer::rust::tests::strip_cfg_test_excludes_test_imports ... ok
test analyzer::rust::tests::strip_cfg_test_handles_nested_blocks ... ok
test analyzer::rust::tests::strip_cfg_test_preserves_non_test_cfg ... ok
# ... 7 more tests ...
test result: ok. 10 passed; 0 failed
```

### Step 7: Verifying the Fix

```bash
$ cargo build --release
$ ./loctree_rs/target/release/loctree loctree_rs/src -A --ext rs --circular

Circular imports detected (1 cycles):
  Cycle 1: analyzer/output.rs -> snapshot.rs -> analyzer/root_scan.rs -> analyzer/runner.rs
```

The false positive from `report.rs` is **gone**!

## Real Finding: Architectural Coupling

The remaining cycle is a **genuine finding**:

```
output.rs → snapshot::GitContext
snapshot.rs → analyzer::output::RootArtifacts, process_root_context, write_report
```

These are function-local imports in production code (`run_init()` and `write_auto_artifacts()`), not test-only. This works in Rust due to lazy evaluation but represents tight coupling.

**Verdict:** Real architectural issue, could be refactored by moving `GitContext` to shared types module.

## Summary

| Metric | Value |
|--------|-------|
| False positives eliminated | 1 |
| Real issues discovered | 1 (architectural coupling) |
| Lines of code added | ~100 |
| Tests added | 3 |
| Time to implement | ~15 minutes |

## Key Insight

Loctree's Rust analyzer was treating `#[cfg(test)]` imports the same as production imports. This is technically correct (they ARE imports), but misleading for cycle detection since test dependencies don't affect production runtime.

## Meta Note

This session demonstrates the power of **dogfooding**: running your own tool on your own codebase reveals edge cases that external testing might miss.

The AI agent was able to:
1. **Diagnose** the issue using loctree's own cycle detection
2. **Trace** the false positive to its source
3. **Implement** a targeted fix
4. **Test** the fix with comprehensive unit tests
5. **Validate** the improvement by re-running the tool

This is the ultimate test: a tool that can find and fix its own bugs.

---

*Created by M&K (c)2025 The LibraxisAI Team*
*Co-Authored-By: [Maciej](void@div0.space) & [Klaudiusz](the1st@whoai.am)*
