#!/bin/bash
set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
HOOK_SRC="$REPO_ROOT/hooks/pre-push"
HOOK_DST="$REPO_ROOT/.git/hooks/pre-push"

chmod +x "$HOOK_SRC"

if [ -L "$HOOK_DST" ] || [ -f "$HOOK_DST" ]; then
  rm "$HOOK_DST"
fi

ln -s "$HOOK_SRC" "$HOOK_DST"

echo "✅ Git hooks installed. Pre-push quality gate active."
