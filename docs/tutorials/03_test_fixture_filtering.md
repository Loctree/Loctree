# Test Fixture Filtering Implementation for Loctree

## Problem Summary

Loctree reports currently include test fixtures as false positives in:
1. **Commands analysis** - Test handlers appear as "unused"
2. **Twins/Dead parrots analysis** - Test fixtures appear as dead code
3. **Crowds analysis** - Test files create noise crowds

Example false positives:
```
[UNUSED] greet - tests/fixtures/tauri_app/src-tauri/src/main.rs:4
[UNUSED] save_data - tests/fixtures/tauri_app/src-tauri/src/main.rs:9
```

## Solution Overview

Add test filtering using the existing `is_test_file()` function in `/Users/maciejgad/hosted/loctree/loctree_rs/src/cli/dispatch.rs` (lines 895-930).

### Key Function (Already Exists)
```rust
/// Check if a file path looks like a test file
fn is_test_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Directory patterns: tests/, __tests__/, test/, spec/
    if path_lower.contains("/tests/")
        || path_lower.contains("/__tests__/")
        || path_lower.contains("/test/")
        || path_lower.contains("/spec/")
        || path_lower.contains("/fixtures/")
        || path_lower.contains("/mocks/")
    {
        return true;
    }

    // File patterns: *_test.*, *.test.*, *_spec.*, *.spec.*, test_*, tests.*
    if filename.contains("_test.")
        || filename.contains(".test.")
        || filename.contains("_spec.")
        || filename.contains(".spec.")
        || filename.contains("_tests.")  // Rust: module_tests.rs
        || filename.starts_with("test_")
        || filename.starts_with("spec_")
        || filename.starts_with("tests.")  // tests.rs
        || filename == "conftest.py"
    {
        return true;
    }

    false
}
```

## Required Changes

### 1. Add `include_tests` field to `TwinsOptions`

**File:** `loctree_rs/src/cli/command.rs`
**Location:** Lines 515-521

```rust
pub struct TwinsOptions {
    /// Root directory to analyze (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Show only dead parrots (symbols with 0 imports)
    pub dead_only: bool,

    /// Include test files in analysis (default: false)
    pub include_tests: bool,
}
```

**Note:** `CrowdOptions` (line 514) and `DeadOptions` (line 318-322) already have this field.

### 2. Update `handle_twins_command` to filter test files

**File:** `loctree_rs/src/cli/dispatch.rs`
**Location:** Lines 1057-1139

Add filtering after loading snapshot (after line 1093):

```rust
let output_mode = if global.json {
    crate::types::OutputMode::Json
} else {
    crate::types::OutputMode::Human
};

// Filter out test files unless --include-tests is specified
let files: Vec<_> = if opts.include_tests {
    snapshot.files.clone()
} else {
    snapshot
        .files
        .iter()
        .filter(|f| !is_test_file(&f.path))
        .cloned()
        .collect()
};

// Run dead parrot analysis
let result = find_dead_parrots(&files, opts.dead_only);  // Changed from &snapshot.files

// ... later in the function (line 1110) ...
// Run exact twins detection (unless dead_only)
if !opts.dead_only {
    let twins = detect_exact_twins(&files);  // Changed from &snapshot.files
    // ... rest of function ...
}
```

### 3. Update `handle_commands_command` to filter test fixtures

**File:** `loctree_rs/src/cli/dispatch.rs`
**Location:** Lines 1283-1339

Replace the existing bridge filtering section (after line 1324):

```rust
// Filter command bridges based on options
let mut bridges: Vec<_> = snapshot.command_bridges.clone();

// Filter out test fixtures from unused handlers (default behavior)
// Test fixtures often appear as "unused" handlers but are valid test setup code
if !opts.missing_only && opts.name_filter.is_none() {
    bridges.retain(|b| {
        // Keep if has handler and is called (OK status)
        if b.has_handler && b.is_called {
            return true;
        }
        // Keep if missing handler (needs implementation)
        if !b.has_handler && b.is_called {
            return true;
        }
        // For unused handlers, filter out test fixtures
        if b.has_handler && !b.is_called {
            if let Some((ref backend_file, _)) = b.backend_handler {
                return !is_test_file(backend_file);
            }
        }
        true
    });
}

// Apply name filter
if let Some(ref filter) = opts.name_filter {
    bridges.retain(|b| b.name.contains(filter));
}

// Apply missing-only filter
if opts.missing_only {
    bridges.retain(|b| !b.has_handler && b.is_called);
}

// Apply unused-only filter (still filter test fixtures)
if opts.unused_only {
    bridges.retain(|b| {
        if !b.has_handler || b.is_called {
            return false;
        }
        // When explicitly asking for unused, still filter test fixtures
        if let Some((ref backend_file, _)) = b.backend_handler {
            return !is_test_file(backend_file);
        }
        true
    });
}
```

### 4. Update CLI parser to support `--include-tests` for twins

**File:** `loctree_rs/src/cli/parser.rs`
**Location:** Find the twins command parsing section

Add flag parsing similar to how it's done for dead and crowd commands:

```rust
// In the twins command parser section
if arg == "--include-tests" {
    opts.include_tests = true;
    continue;
}
```

## Status of Existing Implementations

### Already Working ✅
- **Dead command** (`handle_dead_command`) - Uses `DeadFilterConfig.include_tests` (lines 1169)
- **Crowd command** (`handle_crowd_command`) - Filters test files (lines 966-975)
- `is_test_file()` utility function - Comprehensive test detection (lines 895-930)

### Needs Implementation ⚠️
- **Twins command** - Add `include_tests` field and filtering logic
- **Commands command** - Filter test fixtures from unused handlers
- **CLI parser** - Add `--include-tests` flag parsing for twins command

## Testing the Implementation

### Test with loctree's own repository

```bash
cd /Users/maciejgad/hosted/loctree

# Build with changes
cargo build --release

# Test twins command (should exclude test fixtures by default)
./target/release/loct twins

# Test with --include-tests flag
./target/release/loct twins --include-tests

# Test commands (should exclude test fixtures)
./target/release/loct commands

# Test with specific filter
./target/release/loct commands --name save_data
```

### Expected Results

**Before filtering:**
```
[UNUSED] greet - tests/fixtures/tauri_app/src-tauri/src/main.rs:4
[UNUSED] save_data - tests/fixtures/tauri_app/src-tauri/src/main.rs:9
Dead parrots: 50+ (including test fixtures)
```

**After filtering:**
```
# No test fixtures in output unless --include-tests specified
Dead parrots: 10-20 (actual dead code)
```

## Implementation Notes

### Background Formatter Issue
There is a VS Code + rust-analyzer running that auto-formats code on save. This causes changes to be reverted immediately. To work around this:

1. Kill the `loctree_server` process if running
2. Make all changes atomically in a script
3. Run `cargo build` immediately after changes
4. Or disable format-on-save in VS Code temporarily

### Design Decisions

1. **Default behavior**: Exclude test files (reduce noise)
2. **Opt-in inclusion**: Use `--include-tests` flag when needed
3. **Consistency**: Match existing patterns in crowd and dead commands
4. **No breaking changes**: Existing behavior improves (fewer false positives)

### Backward Compatibility

These changes are **backward compatible** because:
- Default behavior improves (fewer false positives)
- No existing flags or options are changed
- New `--include-tests` flag is opt-in
- Reports will be more accurate by default

## Summary

This implementation filters test fixtures from loctree analysis reports by default, eliminating false positives like "unused test handlers" and "dead test utilities". Users who want to include tests can use the `--include-tests` flag.

The implementation leverages the existing `is_test_file()` function and follows patterns already established in the crowd and dead commands, ensuring consistency across the codebase.

---
**Implementation Status**: Code changes documented, blocked by aggressive auto-formatter
**Testing Required**: Manual verification with loctree's own repository
**Next Steps**: Disable auto-formatter temporarily or apply changes in a single atomic operation
