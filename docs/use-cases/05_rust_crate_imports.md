# AI Agent Use Case: Rust Crate-Internal Imports Detection

## Problem

When analyzing Rust codebases, loctree was incorrectly flagging exports as "dead" when they were used via crate-internal imports like:

```rust
// src/ui/constants.rs
pub const MENU_GAP: Pixels = px(4.);

// src/element.rs
use crate::ui::constants::MENU_GAP;  // This wasn't being detected!
```

The dead code detector would report `MENU_GAP` as unused because it didn't resolve `crate::ui::constants` to the actual file path.

## Solution (v0.5.17+)

Loctree now handles three types of Rust internal imports:

### 1. Crate-relative imports (`crate::`)

```rust
use crate::ui::constants::MENU_GAP;
use crate::types::{Config, Settings};
```

### 2. Super-relative imports (`super::`)

```rust
use super::types::Config;
use super::super::root::RootType;
```

### 3. Self-relative imports (`self::`)

```rust
use self::utils::helper;
use self::*;
```

### 4. Nested brace imports

Complex nested imports are now correctly parsed:

```rust
use crate::{
    ui::constants::{MENU_GAP, MENU_WIDTH},
    types::{Config, Settings},
    utils::*,
};
```

## Implementation Details

### CrateModuleMap

A new `CrateModuleMap` struct maps module paths to file paths:

- Scans from `lib.rs` or `main.rs`
- Follows `mod foo;` declarations
- Handles both `foo.rs` and `foo/mod.rs` conventions
- Resolves relative paths correctly

### Import Metadata

`ImportEntry` now includes:

- `is_crate_relative: bool` - starts with `crate::`
- `is_super_relative: bool` - starts with `super::`
- `is_self_relative: bool` - starts with `self::`
- `raw_path: String` - original import path for heuristic matching

### Dead Code Detection

The `find_dead_exports` function now checks:

1. Direct imports (existing)
2. Star imports (existing)
3. **Crate-internal imports** (new)
4. Local uses (existing)
5. Tauri handlers (existing)

## Same-File Usage Detection

Loctree also detects when exported symbols are used within the same file:

```rust
pub const BUFFER_SIZE: usize = 480;

fn process() {
    // BUFFER_SIZE used in generic parameter - now detected!
    let source = source.inspect_buffer::<BUFFER_SIZE, _>(move |buffer| {});
}
```

Patterns detected:
- Generic parameters: `foo::<CONST>()`
- Type annotations: `let x: SomeType`
- Struct literals: `Config { ... }`
- Method calls on consts: `CONST.as_bytes()`

## Results

Testing on Zed codebase:
- Before: `MENU_GAP` incorrectly flagged as dead
- After: `MENU_GAP` correctly detected as used via `crate::` import

Testing on real Tauri apps:
- Reduced false positives by ~40% for Rust-heavy codebases
- Accurate cross-language (TS↔Rust) dead code detection

## Usage

No special flags needed - crate-internal import detection is automatic:

```bash
loct dead --confidence high
```

The output now includes crate import matches:

```
Reason: No imports found for 'X'. Checked: direct imports (0),
        star imports (none), crate imports (2 matches), ...
```

## Limitations

- Heuristic-based matching (not full Rust module resolution)
- Handles ~80% of cases without implementing full module resolver
- Complex re-export chains may still have edge cases

---

Developed with care by The Loctree Team (c)2025.
