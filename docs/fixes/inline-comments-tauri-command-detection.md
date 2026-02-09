# Fix: Comments Breaking Tauri Command Detection

**Issue**: False "missing handler" reports for valid `#[tauri::command]` functions
**Fixed in**: `v0.8.9`
**Affected versions**: `<= v0.8.8`

## Problem Description

Loctree was failing to detect Tauri command handlers when comments appeared after attributes.

### Example of Affected Code

```rust
#[tauri::command]
#[allow(non_snake_case)] // camelCase param matches frontend convention
pub async fn process_data(
    app_handle: tauri::AppHandle,
    inputData: Vec<u8>,
) -> Result<Vec<u8>, String> {
    // ...
}
```

This pattern is common when developers document *why* an attribute is needed (e.g., explaining `#[allow(...)]` suppressions).

### Symptoms

- `loct commands` reported the handler as `[MISSING]`
- `handlers.json` showed `backend_handler: null` for the command
- The snapshot showed `command_handlers: []` for the file despite having valid `#[tauri::command]` functions

## Root Cause

The regex pattern for detecting `#[tauri::command]` functions only allowed whitespace (`\s*`) between:
1. The `#[tauri::command]` attribute
2. Additional attributes (like `#[allow(...)]`)
3. The `pub async fn` definition

When a comment appeared after an attribute, the regex failed to match.

### Original Regex (simplified)

```regex
#[tauri::command...]
(?:\s*#\s*\[[^\]]*\])*   <- additional attributes with whitespace
\s*                       <- ONLY whitespace allowed here
(?:pub...)?fn...
```

### The Breaking Patterns

```rust
#[allow(non_snake_case)] // comment here
                         ^ NOT whitespace - regex fails
pub async fn handler()
```

```rust
#[allow(non_snake_case)] /* block comment */
                         ^ also fails
pub fn handler()
```

## Solution

Updated the regex with two improvements:

### 1. Comment Support

Accept whitespace, line comments (`// ...`), AND block comments (`/* ... */`):

```regex
^\s*#[tauri::command...]
(?:\s*(?://[^\n]*|/\*[\s\S]*?\*/))?           <- optional comment after main attr
(?:(?:\s|//[^\n]*|/\*[\s\S]*?\*/)*#\[...])*   <- attrs with whitespace OR comments
(?:\s|//[^\n]*|/\*[\s\S]*?\*/)*               <- whitespace OR comments before pub
(?:pub...)?fn...
```

The key change: `\s*` -> `(?:\s|//[^\n]*|/\*[\s\S]*?\*/)*`

This allows:
- Pure whitespace (as before)
- Line comments (`// ...`) after attributes
- Block comments (`/* ... */`) after attributes
- Multiline block comments between attributes
- Standalone comment lines between attributes

### 2. Line Anchoring

Added `^\s*` at the start to anchor to line beginning. This prevents false positives from matching `#[tauri::command]` inside comments or strings:

```rust
// This won't match anymore:
let example = "#[tauri::command]\npub fn fake() {}";

// Neither will this:
// #[tauri::command]
// pub fn commented_out() {}
```

## Files Changed

```
loctree_rs/src/analyzer/regexes.rs
+-- regex_tauri_command_fn()                         - main fix
+-- regex_custom_command_fn()                        - same fix for custom macros
|
+-- Positive tests (should match):
|   +-- test_tauri_command_with_inline_comment()     - // after attr
|   +-- test_tauri_command_comment_between_attrs()   - // between attrs
|   +-- test_tauri_command_with_block_comment()      - /* */ after attr
|   +-- test_tauri_command_with_multiline_block_comment()
|
+-- Negative tests (should NOT match):
    +-- test_tauri_command_in_line_comment_no_match()
    +-- test_tauri_command_in_string_no_match()
```

## Testing

### Before Fix

```bash
$ loct commands | grep my_handler
  [MISSING] my_handler
    Frontend calls (1): src/services/api.ts:42
    [!] Why: Frontend calls invoke('my_handler') but no #[tauri::command] found
```

### After Fix

```bash
$ loct commands | grep my_handler
  [OK] my_handler
    Frontend calls (1): src/services/api.ts:42
    Backend: src-tauri/src/commands/handlers.rs:15
```

### Regression Tests

```rust
#[test]
fn test_tauri_command_with_inline_comment() {
    let re = regex_tauri_command_fn();
    let with_comment = r#"#[tauri::command]
#[allow(non_snake_case)] // parameter name matches frontend camelCase
pub async fn my_handler(...) -> Result<String, String> {"#;

    assert!(re.captures(with_comment).is_some());
}

#[test]
fn test_tauri_command_with_block_comment() {
    let re = regex_tauri_command_fn();
    let with_block = r#"#[tauri::command]
#[allow(non_snake_case)] /* camelCase for frontend */
pub fn my_handler() {}"#;

    assert!(re.captures(with_block).is_some());
}

#[test]
fn test_tauri_command_in_line_comment_no_match() {
    let re = regex_tauri_command_fn();
    let commented_out = r#"// #[tauri::command]
// pub fn disabled_handler() {}"#;

    assert!(re.captures(commented_out).is_none());
}
```

## Impact

Commands that were previously reported as "missing" due to comments after attributes are now correctly detected. The line anchoring also prevents false positives from commented-out or string-embedded code.

## Related

- Tauri command bridge detection: `loct commands`
- Rust analyzer: `loctree_rs/src/analyzer/rust/mod.rs`
- Custom command macros: Also fixed in `regex_custom_command_fn()`
