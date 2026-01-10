#!/bin/bash
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
# ============================================================================
# loct-smart-suggest.sh - Context-aware loct suggestions for Claude Code
# ============================================================================
#
# ALTERNATIVE APPROACH: PreToolUse hook that SUGGESTS loct commands
# instead of automatically augmenting. Use this if you prefer manual control.
#
# Non-blocking! Just adds helpful hints to stderr when loct would be better.
#
# INSTALLATION:
#   1. Copy to ~/.claude/hooks/loct-smart-suggest.sh
#   2. chmod +x ~/.claude/hooks/loct-smart-suggest.sh
#   3. Add to ~/.claude/settings.json under PreToolUse (not PostToolUse)
#
# ============================================================================

INPUT=$(cat)

# Extract pattern from JSON
PATTERN=$(echo "$INPUT" | jq -r '.pattern // .tool_input.pattern // empty' 2>/dev/null)
if [[ -z "$PATTERN" ]]; then
    PATTERN=$(echo "$INPUT" | grep -oP '"pattern"\s*:\s*"\K[^"]+' 2>/dev/null || echo "")
fi

[[ -z "$PATTERN" ]] && exit 0

# Track suggestions to avoid spam (max 3 per session)
SUGGEST_COUNT_FILE="/tmp/.loct-suggest-count-$(date +%Y%m%d)"
SUGGEST_COUNT=$(cat "$SUGGEST_COUNT_FILE" 2>/dev/null || echo "0")
[[ "$SUGGEST_COUNT" -ge 3 ]] && exit 0

suggest() {
    local hint="$1"
    local cmd="$2"
    echo "" >&2
    echo "┌─────────────────────────────────────────────────────────────" >&2
    echo "│ 🌳 $hint" >&2
    echo "│ → $cmd" >&2
    echo "└─────────────────────────────────────────────────────────────" >&2
    echo "" >&2
    echo $((SUGGEST_COUNT + 1)) > "$SUGGEST_COUNT_FILE"
}

# Case 1: React Component or Type (PascalCase)
if [[ "$PATTERN" =~ ^[A-Z][a-zA-Z0-9]{2,}$ ]]; then
    suggest "Symbol search? loct finds definition + all usages" \
            "loct f $PATTERN"
    exit 0
fi

# Case 2: React Hook (useXxx)
if [[ "$PATTERN" =~ ^use[A-Z][a-zA-Z0-9]+$ ]]; then
    suggest "Hook search? loct shows definition + import chain" \
            "loct f $PATTERN"
    exit 0
fi

# Case 3: Event Handler (handleXxx, onXxx)
if [[ "$PATTERN" =~ ^(handle|on)[A-Z][a-zA-Z0-9]+$ ]]; then
    suggest "Handler search? loct finds definition + prop passing" \
            "loct f $PATTERN"
    exit 0
fi

# Case 4: Tauri command patterns
if [[ "$PATTERN" =~ invoke|safeInvoke|emit\( ]]; then
    suggest "Tauri bridge? loct trace shows FE↔BE coverage" \
            "loct trace <handler_name>"
    exit 0
fi

# Case 5: Import/export analysis
if [[ "$PATTERN" =~ ^import|^export|from.+import ]]; then
    suggest "Import analysis? loct has full dependency graph" \
            "loct q who-imports <file>"
    exit 0
fi

# Case 6: Snake_case symbol (Rust/Python)
if [[ "$PATTERN" =~ ^[a-z][a-z0-9]*_[a-z_0-9]+$ ]]; then
    suggest "Symbol search? loct finds across TS+Rust with context" \
            "loct f $PATTERN"
    exit 0
fi

# Case 7: Checking if something exists/is used
if [[ "$PATTERN" =~ ^(is|has|can|should)[A-Z] ]]; then
    suggest "Checking usage? loct can tell if it's dead code" \
            "loct f $PATTERN"
    exit 0
fi

# Case 8: Dead/unused patterns
if [[ "$PATTERN" =~ dead|unused|orphan|stale ]]; then
    suggest "Dead code hunt? loct has pre-indexed findings" \
            "loct health"
    exit 0
fi

# Case 9: Circular/cycle patterns
if [[ "$PATTERN" =~ circular|cycle|loop|recursive ]]; then
    suggest "Cycle detection? loct has SCC analysis ready" \
            "loct health"
    exit 0
fi

# Case 10: Duplicate/twin patterns
if [[ "$PATTERN" =~ duplicate|twin|copy|similar ]]; then
    suggest "Finding duplicates? loct detected exact twins" \
            "loct health"
    exit 0
fi

# No match - grep is fine
exit 0
