#!/bin/bash
# ============================================================================
# memex-context.sh - Project context loader via HTTP API
# ============================================================================
# Created by M&K (c)2026 VetCoders
#
# TRIGGER: PostToolUse (Grep, Read)
# PURPOSE: Augment tool results with relevant memories from memex
#
# REQUIRES: rmcp-memex server running (rmcp-memex serve --http-port 8987)
# BENCHMARK: ~50ms (HTTP API)
# ============================================================================

set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# Configuration
MEMEX_URL="${MEMEX_URL:-http://localhost:8987}"
MEMEX_LIMIT="${MEMEX_LIMIT:-3}"
MEMEX_TIMEOUT="${MEMEX_TIMEOUT:-2}"

# Quick exits
command -v curl &>/dev/null || exit 0
command -v jq &>/dev/null || exit 0

# Read hook input
HOOK_INPUT=$(cat)
[[ -z "$HOOK_INPUT" ]] && exit 0

# Extract pattern from Grep tool
PATTERN=$(echo "$HOOK_INPUT" | jq -r '.tool_input.pattern // empty' 2>/dev/null)
[[ -z "$PATTERN" ]] && exit 0
[[ ${#PATTERN} -lt 3 ]] && exit 0

# Skip heavy regex patterns
echo "$PATTERN" | grep -qE '[\|\*\+\?\[\]\(\)\{\}\\]{3,}' && exit 0

# Check if server is up
if ! curl -s --max-time 1 "$MEMEX_URL/health" >/dev/null 2>&1; then
    exit 0
fi

# Search memex
SEARCH_RESULT=$(curl -s --max-time "$MEMEX_TIMEOUT" \
    "$MEMEX_URL/cross-search?q=$(echo "$PATTERN" | jq -sRr @uri)&limit=$MEMEX_LIMIT" \
    2>/dev/null)

[[ -z "$SEARCH_RESULT" ]] && exit 0

# Check if we got results
RESULT_COUNT=$(echo "$SEARCH_RESULT" | jq -r '.total_results // 0' 2>/dev/null)
[[ "$RESULT_COUNT" == "0" ]] && exit 0

# Format output
MEMORIES=$(echo "$SEARCH_RESULT" | jq -r '
    .results[:3] |
    to_entries |
    map("[\(.value.namespace)] \(.value.text | .[0:200] | gsub("\n"; " "))")[]
' 2>/dev/null)

[[ -z "$MEMORIES" ]] && exit 0

CONTEXT="
--- MEMEX: $RESULT_COUNT memories for '$PATTERN' ---
$MEMORIES"

# Human-readable (stderr)
echo "$CONTEXT" >&2

# JSON for Claude Code (stdout)
ESCAPED=$(echo "$CONTEXT" | jq -Rs .)
cat << EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": $ESCAPED
  }
}
EOF
exit 0
