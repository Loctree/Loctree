#!/bin/bash
# ============================================================================
# memex-startup.sh - Project context loader at session start
# ============================================================================
# Created by M&K (c)2026 VetCoders
#
# TRIGGER: SessionStart hook
# PURPOSE: Load institutional knowledge about the current project from memex
#
# REQUIRES: rmcp-memex server running (rmcp-memex serve --http-port 8987)
# BENCHMARK: ~50ms (HTTP API with 1h caching)
# ============================================================================

set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# Configuration
MEMEX_URL="${MEMEX_URL:-http://localhost:8987}"
MEMEX_LIMIT="${MEMEX_LIMIT:-3}"
MEMEX_TIMEOUT="${MEMEX_TIMEOUT:-3}"
CACHE_TTL=3600  # 1 hour

# Quick exits
command -v curl &>/dev/null || exit 0
command -v jq &>/dev/null || exit 0

# Find repo root (git root or .loctree parent)
REPO_ROOT=$(pwd)
GIT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || true)
if [[ -n "$GIT_ROOT" ]]; then
    REPO_ROOT="$GIT_ROOT"
else
    # Fallback: find .loctree directory
    while [[ "$REPO_ROOT" != "/" ]] && [[ ! -d "$REPO_ROOT/.loctree" ]]; do
        REPO_ROOT=$(dirname "$REPO_ROOT")
    done
    [[ ! -d "$REPO_ROOT/.loctree" ]] && REPO_ROOT=$(pwd)
fi

# Cache uses repo root (not PWD) to avoid subfolder confusion
CACHE_FILE="/tmp/memex-startup-$(echo "$REPO_ROOT" | md5 2>/dev/null || md5sum <<< "$REPO_ROOT" | cut -c1-8).cache"

# Cache check - only load once per project per hour
if [[ -f "$CACHE_FILE" ]]; then
    if stat -f%m "$CACHE_FILE" &>/dev/null; then
        CACHE_AGE=$(($(date +%s) - $(stat -f%m "$CACHE_FILE")))
    else
        CACHE_AGE=$(($(date +%s) - $(stat -c%Y "$CACHE_FILE")))
    fi
    if [[ $CACHE_AGE -lt $CACHE_TTL ]]; then
        exit 0
    fi
fi

# Check if server is up
if ! curl -s --max-time 1 "$MEMEX_URL/health" >/dev/null 2>&1; then
    touch "$CACHE_FILE"
    exit 0
fi

# Project detection (use already-resolved REPO_ROOT)
PROJECT_NAME=$(basename "$REPO_ROOT")
PROJECT_QUERY=$(echo "$PROJECT_NAME" | tr '-_' ' ')

# Search memex
SEARCH_RESULT=$(curl -s --max-time "$MEMEX_TIMEOUT" \
    "$MEMEX_URL/cross-search?q=$(echo "$PROJECT_QUERY" | jq -sRr @uri)&limit=$MEMEX_LIMIT&total_limit=$((MEMEX_LIMIT * 2))" \
    2>/dev/null)

touch "$CACHE_FILE"

[[ -z "$SEARCH_RESULT" ]] && exit 0

# Check if we got results
RESULT_COUNT=$(echo "$SEARCH_RESULT" | jq -r '.total_results // 0' 2>/dev/null)
[[ "$RESULT_COUNT" == "0" ]] && exit 0

# Format output
MEMORIES=$(echo "$SEARCH_RESULT" | jq -r '
    .results[:3] |
    to_entries |
    map("[\(.value.namespace)] \(.value.text | .[0:300] | gsub("\n"; " "))")[]
' 2>/dev/null)

[[ -z "$MEMORIES" ]] && exit 0

NS_SEARCHED=$(echo "$SEARCH_RESULT" | jq -r '.namespaces_searched // 0' 2>/dev/null)

CONTEXT="
--- MEMEX: $PROJECT_NAME (searched $NS_SEARCHED namespaces) ---
$MEMORIES"

# Human-readable (stderr)
echo "$CONTEXT" >&2

# JSON for Claude Code (stdout)
ESCAPED=$(echo "$CONTEXT" | jq -Rs .)
cat << EOF
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": $ESCAPED
  }
}
EOF
exit 0
