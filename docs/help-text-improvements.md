# Help Text Improvements - December 2025

## Summary

Updated inline help text for all loctree subcommands to prevent user confusion between different command purposes.

## Problem

User feedback indicated confusion between `loct find` (semantic search) and impact analysis. The original help text wasn't clear enough about what each command does vs. what it doesn't do.

## Changes Made

### 1. `loct find` - Semantic Search

**Key Clarifications:**
- Explicitly states it's "semantic search for symbols by name pattern"
- Adds clear "NOT" statements:
  - "NOT impact analysis - for dependency impact, see your editor's LSP"
  - "NOT dead code detection - for that, use 'loct dead' or 'loct twins'"
- Added SEARCH MODES section explaining the three modes
- Added RELATED COMMANDS section pointing to correct tools

**Before:**
```
loct find - Search symbols/files with regex filters
Powerful search across the codebase with multiple filter modes.
```

**After:**
```
loct find - Semantic search for symbols by name pattern
NOT impact analysis - for dependency impact, see your editor's LSP
NOT dead code detection - for that, use 'loct dead' or 'loct twins'
```

### 2. `loct slice` - Extract Dependencies

**Key Clarifications:**
- Changed title from "holographic context" to "Extract file + dependencies for AI context"
- Clarified it shows what the file USES, not what USES it
- Added "NOT impact analysis" statement
- Better OUTPUT FORMAT section explaining the 3 layers
- Added examples with JSON output for AI tools

### 3. `loct twins` - Dead Parrots & Duplicates

**Key Clarifications:**
- Changed title to emphasize "dead parrots (0 imports)" first
- Expanded explanation of what each category means
- Added warning: "This is a code smell detector - findings are hints, not verdicts"
- Better categorization of the three types of issues

### 4. `loct cycles` - Circular Imports

**Key Clarifications:**
- Changed title from "Detect circular imports" to "Detect circular import chains"
- Added concrete example: "A → B → C → A"
- Expanded consequences section with real-world issues
- Added OUTPUT FORMAT section
- Added FIXING CYCLES section with actionable steps

### 5. `loct commands` - Tauri Handler Coverage

**Key Clarifications:**
- Changed title to "Tauri FE↔BE handler coverage analysis"
- Better explains the three types of issues
- Added DETECTION section explaining what it scans
- OUTPUT section shows the three report categories
- Better examples with CI automation

### 6. `loct query` - Graph Queries

**Key Clarifications:**
- Changed title from "Query snapshot data" to "Graph queries"
- Better explains each query kind with descriptions
- Added OUTPUT section explaining what you get
- Clear ARGUMENTS section

### 7. `loct diff` - Snapshot Comparison

**Key Clarifications:**
- Changed title to "Compare snapshots between branches/commits"
- Added comparison to git diff: "Unlike git diff (line changes), this shows structural changes"
- Better explains what snapshots can be (git refs, tags, SHAs)
- Added OUTPUT section showing categories
- More examples with different use cases

## Format Improvements

All help texts now follow a consistent structure:

1. **TITLE** - One-line summary of what the command does
2. **USAGE** - Syntax
3. **DESCRIPTION** - What it does, what it's NOT (if confusing)
4. **SECTIONS** - Modes, Options, Arguments
5. **EXAMPLES** - Real-world usage
6. **OUTPUT** (if relevant) - What the output looks like
7. **RELATED COMMANDS** - Where to go for related functionality

## Testing

Run these to verify the improvements:

```bash
# Test each command's help
loct find --help
loct slice --help
loct twins --help
loct cycles --help
loct commands --help
loct query --help
loct diff --help

# Verify no compilation issues
cargo clippy
```

## User Impact

Users should now:
- Immediately understand what each command does
- Know what each command does NOT do (preventing confusion)
- Find related commands easily via RELATED COMMANDS section
- Have better examples showing real-world usage
- Understand output format before running commands

## Files Modified

- `/Users/maciejgad/hosted/loctree/loctree_rs/src/cli/parser.rs`
  - Updated help text for: find, slice, twins, cycles, commands, query, diff
  - No code logic changes, only documentation strings

## Next Steps

Consider:
1. Adding `--help-full` flag for extended help with more examples
2. Creating interactive `loct tutorial` command
3. Adding shell completion with inline hints
4. Creating docs/tutorials/ with step-by-step guides
