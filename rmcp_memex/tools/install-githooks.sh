#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOKS_DIR="$ROOT/.git/hooks"
SRC_DIR="$ROOT/tools/githooks"

if [[ ! -d "$HOOKS_DIR" ]]; then
  echo "No .git/hooks directory found. Are you in a git repo?" >&2
  exit 1
fi

# Install pre-commit hook
SRC_PRECOMMIT="$SRC_DIR/pre-commit"
if [[ -f "$SRC_PRECOMMIT" ]]; then
  chmod +x "$SRC_PRECOMMIT"
  ln -sf "$SRC_PRECOMMIT" "$HOOKS_DIR/pre-commit"
  echo "✓ Installed pre-commit hook -> $HOOKS_DIR/pre-commit"
else
  echo "⚠️  No pre-commit hook found at $SRC_PRECOMMIT"
fi

# Install pre-push hook
SRC_PREPUSH="$SRC_DIR/pre-push"
if [[ -f "$SRC_PREPUSH" ]]; then
  chmod +x "$SRC_PREPUSH"
  ln -sf "$SRC_PREPUSH" "$HOOKS_DIR/pre-push"
  echo "✓ Installed pre-push hook -> $HOOKS_DIR/pre-push"
else
  echo "⚠️  No pre-push hook found at $SRC_PREPUSH"
fi

echo ""
echo "Done. Git hooks installed."
