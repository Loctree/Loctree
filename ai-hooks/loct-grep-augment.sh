#!/bin/bash
# ============================================================================
# loct-grep-augment.sh v10 - SEMANTIC AUGMENTATION WITH loct find
# ============================================================================
# Created by M&K (c)2026 The Loctree Team
#
# PHILOSOPHY: Every grep gets loctree context. Always <100ms.
#
# BENCHMARK (loctree 0.8.4 - UPDATED 2026-01):
#   loct find               ~75ms  ✅ (semantic + params + dead status)
#   loct commands           ~47ms  ✅
#   loct impact             ~49ms  ✅
#   loct query who-imports  ~53ms  ✅
#   loct focus              ~56ms  ✅
#   loct slice              ~81ms  ✅
#   loct health            ~372ms  ⚠️ (only for health queries)
#
# KEY CHANGE v10: loct find is now FAST! Use it for all symbol lookups.
# ============================================================================

set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# Quick exit if dependencies unavailable
command -v loct >/dev/null 2>&1 || exit 0
command -v jq   >/dev/null 2>&1 || exit 0

# ============================================================================
# OPTIONAL LOGGING (default OFF - set LOCT_HOOK_LOG_FILE to enable)
# ============================================================================
# Only logs metadata (pattern truncated, timing, action) - never tool_response!
# Enable: export LOCT_HOOK_LOG_FILE="$HOME/.claude/logs/loct-grep.log"

LOG_FILE="${LOCT_HOOK_LOG_FILE:-}"
LOG_START_MS=

log_meta() {
    [[ -z "$LOG_FILE" ]] && return 0
    mkdir -p "$(dirname "$LOG_FILE")" 2>/dev/null || true
    printf '%s\n' "$*" >> "$LOG_FILE" 2>/dev/null || true
}

log_start() {
    [[ -z "$LOG_FILE" ]] && return 0
    LOG_START_MS=$(($(date +%s%N 2>/dev/null || echo "$(date +%s)000000000") / 1000000))
    log_meta ""
    log_meta "==== LOCT HOOK: $(date '+%Y-%m-%d %H:%M:%S') ===="
}

log_end() {
    [[ -z "$LOG_FILE" ]] && return 0
    local action="${1:-unknown}"
    local pattern_short="${PATTERN:-?}"
    [[ ${#pattern_short} -gt 40 ]] && pattern_short="${pattern_short:0:37}..."

    local end_ms=$(($(date +%s%N 2>/dev/null || echo "$(date +%s)000000000") / 1000000))
    local duration_ms=$((end_ms - LOG_START_MS))

    log_meta "pattern: $pattern_short"
    log_meta "path:    ${PATH_ARG:-.}"
    log_meta "action:  $action"
    log_meta "time:    ${duration_ms}ms"
    log_meta "----"
}

# Start timing (if logging enabled)
log_start

# Read hook input
HOOK_INPUT=$(cat)
[[ -z "$HOOK_INPUT" ]] && exit 0

# ============================================================================
# PATTERN EXTRACTION
# ============================================================================

if [[ "${1:-}" == "--bash-filter" ]]; then
    # Bash tool with rg/grep command
    COMMAND=$(echo "$HOOK_INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)
    echo "$COMMAND" | grep -qE '(^|[[:space:]])(rg|ripgrep|grep)[[:space:]]' || exit 0

    # Extract quoted pattern first, then unquoted
    PATTERN=$(echo "$COMMAND" | grep -oE '"[^"]+"' | head -1 | tr -d '"')
    [[ -z "$PATTERN" ]] && PATTERN=$(echo "$COMMAND" | grep -oE "'[^']+'" | head -1 | tr -d "'")
    [[ -z "$PATTERN" ]] && PATTERN=$(echo "$COMMAND" | sed -nE 's/.*\b(rg|grep)\b[[:space:]]+([^[:space:]-][^[:space:]]*).*/\2/p')

    # Extract path (last arg if looks like path)
    PATH_ARG=$(echo "$COMMAND" | awk '{print $NF}')
    [[ ! "$PATH_ARG" =~ ^\.?/ ]] && [[ ! -e "$PATH_ARG" ]] && PATH_ARG="."
else
    # Native Grep tool
    PATTERN=$(echo "$HOOK_INPUT" | jq -r '.tool_input.pattern // empty' 2>/dev/null)
    PATH_ARG=$(echo "$HOOK_INPUT" | jq -r '.tool_input.path // "."' 2>/dev/null)
fi

# Change to appropriate directory for loctree context
# Priority: tool_input.path (if absolute) > session_cwd > current dir
if [[ "$PATH_ARG" == /* ]] && [[ -d "$PATH_ARG" ]]; then
    # Absolute directory path - use it directly
    cd "$PATH_ARG"
elif [[ "$PATH_ARG" == /* ]] && [[ -f "$PATH_ARG" ]]; then
    # Absolute file path - use its parent directory
    cd "$(dirname "$PATH_ARG")"
else
    # Fall back to session_cwd
    SESSION_CWD=$(echo "$HOOK_INPUT" | jq -r '.session_cwd // empty' 2>/dev/null)
    [[ -n "$SESSION_CWD" ]] && [[ -d "$SESSION_CWD" ]] && cd "$SESSION_CWD"
fi

# Find repo root (where .loctree exists)
REPO_ROOT=$(pwd)
while [[ "$REPO_ROOT" != "/" ]] && [[ ! -d "$REPO_ROOT/.loctree" ]]; do
    REPO_ROOT=$(dirname "$REPO_ROOT")
done
[[ -d "$REPO_ROOT/.loctree" ]] && cd "$REPO_ROOT"

# Validation
[[ -z "$PATTERN" ]] && exit 0
[[ ${#PATTERN} -lt 3 ]] && exit 0
# Skip heavy regex patterns
echo "$PATTERN" | grep -qE '[\|\*\+\?\[\]\(\)\{\}\\]{3,}' && exit 0

# Clean quotes
PATTERN="${PATTERN%\"}"
PATTERN="${PATTERN#\"}"
PATTERN="${PATTERN%\'}"
PATTERN="${PATTERN#\'}"

# ============================================================================
# OUTPUT HELPER
# ============================================================================

# Max payload size (32KB) to avoid bloating additionalContext
MAX_PAYLOAD_BYTES=32768

truncate_payload() {
    local text="$1"
    local max="$2"
    if [[ ${#text} -gt $max ]]; then
        printf '%s\n\n[...truncated, showing first %d bytes of %d total]' \
            "${text:0:$max}" "$max" "${#text}"
    else
        printf '%s' "$text"
    fi
}

output_json() {
    local header="$1"
    local json_content="$2"

    # Log metadata (if enabled) - uses header as action name
    log_end "$header"

    local msg="
━━━ 🌳 LOCTREE: $header ━━━
$json_content"

    # Truncate if too large (prevents client issues)
    msg="$(truncate_payload "$msg" "$MAX_PAYLOAD_BYTES")"

    # Human-readable for Maciej (stderr)
    echo "$msg" >&2

    # JSON for Claude Code (stdout - CC parses hookSpecificOutput)
    local escaped
    escaped=$(echo "$msg" | jq -Rs .)
    local output
    output=$(cat << EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $escaped
  }
}
EOF
)
    # Output to stdout
    echo "$output"

    # Save to cache (if cache key is set)
    [[ -n "${CACHE_KEY:-}" ]] && echo "$output" > "$CACHE_KEY" 2>/dev/null
}

# ============================================================================
# RESPONSE CACHE (dedup - same query within TTL returns cached response)
# ============================================================================

CACHE_DIR="/tmp/.loct-grep-cache"
CACHE_TTL=60  # seconds

# Compute cache key from: repo_root + git_commit + pattern + path
compute_cache_key() {
    local repo="$1"
    local pattern="$2"
    local path="$3"

    # Get current git commit (short hash) or "nocommit"
    local commit
    commit=$(git -C "$repo" rev-parse --short HEAD 2>/dev/null || echo "nocommit")

    # Create cache dir if needed
    mkdir -p "$CACHE_DIR" 2>/dev/null || true

    # MD5 hash of key components
    local hash
    hash=$(printf '%s:%s:%s:%s' "$repo" "$commit" "$pattern" "$path" | md5 2>/dev/null || md5sum | cut -c1-32)

    echo "${CACHE_DIR}/${hash}.json"
}

# Check cache - returns 0 and outputs cached response if hit, 1 if miss
check_cache() {
    local cache_file="$1"

    [[ ! -f "$cache_file" ]] && return 1

    # Check age
    local cache_age
    if stat -f%m "$cache_file" &>/dev/null; then
        cache_age=$(($(date +%s) - $(stat -f%m "$cache_file")))
    else
        cache_age=$(($(date +%s) - $(stat -c%Y "$cache_file")))
    fi

    if [[ $cache_age -lt $CACHE_TTL ]]; then
        # Cache hit - output cached response
        cat "$cache_file"
        # Debug to log only (not stderr - would pollute UI)
        log_meta "[cache] hit (${cache_age}s old)"
        return 0
    fi

    # Cache expired
    rm -f "$cache_file" 2>/dev/null
    return 1
}

# ============================================================================
# FAST AUGMENTATION FUNCTIONS (<100ms each!)
# ============================================================================

# Symbol lookup via loct find (FAST in 0.8.4: ~75ms with semantic matches!)
augment_symbol() {
    local symbol="$1"
    local result

    # loct find gives: symbol_matches + param_matches + semantic_matches + dead_status
    result=$(loct find "$symbol" --json 2>/dev/null)
    [[ -z "$result" ]] && return 1

    # Check if ANY matches found (symbol, param, or semantic)
    local has_matches
    has_matches=$(echo "$result" | jq '
        (.symbol_matches.total_matches // 0) > 0 or
        (.param_matches | length) > 0 or
        (.semantic_matches | length) > 0
    ' 2>/dev/null)
    [[ "$has_matches" != "true" ]] && return 1

    output_json "find $symbol" "$result"
    exit 0
}

# File context via slice
augment_file() {
    local file="$1"
    [[ ! -f "$file" ]] && return 1

    local result
    result=$(loct slice "$file" --json 2>/dev/null)
    [[ -z "$result" ]] && return 1

    output_json "slice $file" "$result"
    exit 0
}

# File impact analysis
augment_impact() {
    local file="$1"
    [[ ! -f "$file" ]] && return 1

    local result
    result=$(loct impact "$file" --json 2>/dev/null)
    [[ -z "$result" ]] && return 1

    output_json "impact $file" "$result"
    exit 0
}

# Who imports this file
augment_who_imports() {
    local file="$1"
    [[ ! -f "$file" ]] && return 1

    local result
    result=$(loct query who-imports "$file" --json 2>/dev/null)
    [[ -z "$result" ]] && return 1

    output_json "who-imports $file" "$result"
    exit 0
}

# Directory overview via focus
augment_directory() {
    local dir="$1"
    [[ ! -d "$dir" ]] && return 1

    local result
    result=$(loct focus "$dir" --json 2>/dev/null)
    [[ -z "$result" ]] && return 1

    output_json "focus $dir" "$result"
    exit 0
}

# Tauri command bridge
augment_tauri_command() {
    local cmd="$1"

    local result
    result=$(loct commands --json 2>/dev/null | jq --arg cmd "$cmd" '[.[] | select(.name | contains($cmd))]' 2>/dev/null)
    [[ -z "$result" ]] || [[ "$result" == "[]" ]] && return 1

    output_json "commands matching $cmd" "$result"
    exit 0
}

# Health check (only for health-related queries, ~372ms)
augment_health() {
    local result
    result=$(loct health --json 2>/dev/null)
    [[ -z "$result" ]] && return 1

    output_json "health" "$result"
    exit 0
}

# ============================================================================
# CACHE CHECK - Return cached response if available (dedup)
# ============================================================================

CACHE_KEY=$(compute_cache_key "$REPO_ROOT" "$PATTERN" "$PATH_ARG")
if check_cache "$CACHE_KEY"; then
    log_end "cache-hit"
    exit 0
fi

# ============================================================================
# SMART ROUTING - Pattern → Best Augmentation
# ============================================================================

# Priority 1: Exact file path → slice + who-imports
if [[ -f "$PATTERN" ]]; then
    augment_file "$PATTERN"
fi

# Priority 2: Path argument is specific file → impact analysis
if [[ "$PATH_ARG" != "." ]] && [[ -f "$PATH_ARG" ]]; then
    augment_impact "$PATH_ARG"
fi

# Priority 3: Directory path → focus
if [[ -d "$PATTERN" ]] || [[ "$PATTERN" == */ ]]; then
    augment_directory "${PATTERN%/}"
fi
if [[ "$PATH_ARG" != "." ]] && [[ -d "$PATH_ARG" ]]; then
    augment_directory "$PATH_ARG"
fi

# Priority 4: Tauri snake_case commands (mcp_list_integrations, etc.)
if echo "$PATTERN" | grep -qE '^[a-z][a-z0-9]*(_[a-z0-9]+)+$'; then
    augment_tauri_command "$PATTERN"
fi

# Priority 5: File-like pattern (has extension) → find file, then slice
if echo "$PATTERN" | grep -qE '\.(ts|tsx|rs|js|jsx|py|vue|svelte|css|scss)$'; then
    FOUND=$(find . -path "./.git" -prune -o -name "$PATTERN" -type f -print 2>/dev/null | head -1)
    [[ -n "$FOUND" ]] && augment_file "$FOUND"
fi

# Priority 6: Symbol patterns → FAST query where-symbol
# PascalCase: Components, Types, Interfaces (ChatPanel, PatientRecord)
if echo "$PATTERN" | grep -qE '^[A-Z][a-zA-Z0-9]{2,}$'; then
    augment_symbol "$PATTERN"
fi

# camelCase with uppercase: hooks, handlers (useVistaAgent, handleClick)
if echo "$PATTERN" | grep -qE '^[a-z]+[A-Z][a-zA-Z0-9]*$'; then
    augment_symbol "$PATTERN"
fi

# React hooks: useXxx
if echo "$PATTERN" | grep -qE '^use[A-Z][a-zA-Z0-9]+$'; then
    augment_symbol "$PATTERN"
fi

# Event handlers: handleXxx, onXxx
if echo "$PATTERN" | grep -qE '^(handle|on)[A-Z][a-zA-Z0-9]+$'; then
    augment_symbol "$PATTERN"
fi

# snake_case identifiers (Rust, Python)
if echo "$PATTERN" | grep -qE '^[a-z][a-z0-9]*_[a-z_0-9]+$'; then
    augment_symbol "$PATTERN"
fi

# Boolean prefixes: isActive, hasPermission, canEdit
if echo "$PATTERN" | grep -qE '^(is|has|can|should|will)[A-Z][a-zA-Z0-9]*$'; then
    augment_symbol "$PATTERN"
fi

# SCREAMING_CASE constants
if echo "$PATTERN" | grep -qE '^[A-Z][A-Z0-9_]+$'; then
    augment_symbol "$PATTERN"
fi

# Priority 7: Path-like patterns → try to resolve
if [[ "$PATTERN" == *"/"* ]]; then
    # Try as file
    FOUND=$(find . -path "./.git" -prune -o -path "*$PATTERN*" -type f -print 2>/dev/null | head -1)
    [[ -n "$FOUND" ]] && augment_file "$FOUND"

    # Try as directory
    FOUND_DIR=$(find . -path "./.git" -prune -o -path "*$PATTERN*" -type d -print 2>/dev/null | head -1)
    [[ -n "$FOUND_DIR" ]] && augment_directory "$FOUND_DIR"
fi

# Priority 8: Health-related keywords (only case where we use slower command)
if echo "$PATTERN" | grep -qiE 'dead|unused|orphan|stale|deprecated|circular|cycle|duplicate|twin'; then
    augment_health
fi

# Priority 9: CATCH-ALL - Any alphanumeric pattern ≥3 chars → try loct find
# This catches patterns like "passthrough", "handler", "template" that fell through
if echo "$PATTERN" | grep -qE '^[a-zA-Z_][a-zA-Z0-9_]{2,}$'; then
    augment_symbol "$PATTERN"
fi

# No augmentation needed for pure text/regex searches
log_end "no-match"
exit 0
