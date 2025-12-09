#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOKS_DIR="$ROOT/.git/hooks"
SRC="$ROOT/tools/githooks/pre-commit"

if [[ ! -d "$HOOKS_DIR" ]]; then
  echo "No .git/hooks directory found. Are you in a git repo?" >&2
  exit 1
fi

chmod +x "$SRC"
ln -sf "$SRC" "$HOOKS_DIR/pre-commit"

echo "Installed pre-commit hook -> $HOOKS_DIR/pre-commit"
