#!/bin/bash
# ============================================================================
# memory-on-compact.sh - Save session context before compaction
# ============================================================================
# Created by M&K (c)2026 VetCoders
#
# TRIGGER: PreCompact hook (manual or auto)
# PURPOSE: Persist conversation context to memex before context window compacts
#
# REQUIRES: rmcp-memex server running (rmcp-memex serve --http-port 8987)
# BENCHMARK: ~50ms (HTTP API)
# ============================================================================

set -uo pipefail
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

# Configuration
MEMEX_URL="${MEMEX_URL:-http://localhost:8987}"
MEMEX_TIMEOUT="${MEMEX_TIMEOUT:-5}"
NAMESPACE="ai-sessions"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Quick exits
command -v curl &>/dev/null || exit 0
command -v jq &>/dev/null || exit 0

# Read hook input
input=$(cat)

# Parse JSON fields
session_id=$(echo "$input" | jq -r '.session_id // "unknown"')
transcript_path=$(echo "$input" | jq -r '.transcript_path // ""')
trigger=$(echo "$input" | jq -r '.trigger // "manual"')
custom_instructions=$(echo "$input" | jq -r '.custom_instructions // ""')

ID="compact-$(date +%Y%m%d-%H%M%S)-${session_id:0:8}"

# Extract summary from transcript if available
summary=""
if [ -n "$transcript_path" ] && [ -f "$transcript_path" ]; then
    summary=$(tail -n 20 "$transcript_path" 2>/dev/null | head -c 5000)
fi

# Add custom instructions if provided
if [ -n "$custom_instructions" ]; then
    summary="CUSTOM: $custom_instructions

$summary"
fi

# Only upsert if we have content
if [ -z "$summary" ]; then
    echo "No content to save for compact" >&2
    exit 0
fi

# Check if server is up
if ! curl -s --max-time 1 "$MEMEX_URL/health" >/dev/null 2>&1; then
    echo "Memex server not available, skipping" >&2
    exit 0
fi

# Build JSON payload
JSON_PAYLOAD=$(jq -n \
    --arg namespace "$NAMESPACE" \
    --arg id "$ID" \
    --arg text "$summary" \
    --arg type "compact" \
    --arg trigger "$trigger" \
    --arg timestamp "$TIMESTAMP" \
    --arg host "$(hostname)" \
    --arg session "$session_id" \
    '{
        namespace: $namespace,
        id: $id,
        text: $text,
        metadata: {
            type: $type,
            trigger: $trigger,
            timestamp: $timestamp,
            host: $host,
            session: $session
        }
    }')

# Upsert to memex via HTTP
RESPONSE=$(curl -s --max-time "$MEMEX_TIMEOUT" \
    -X POST "$MEMEX_URL/upsert" \
    -H "Content-Type: application/json" \
    -d "$JSON_PAYLOAD" 2>/dev/null)

if echo "$RESPONSE" | jq -e '.success' &>/dev/null; then
    echo "Compact memory saved: $ID ($trigger)" >&2
else
    echo "Failed to save compact memory: $RESPONSE" >&2
fi

exit 0
