#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOKS_DIR="$ROOT/.git/hooks"

if [[ ! -d "$HOOKS_DIR" ]]; then
  echo "No .git/hooks directory found. Are you in a git repo?" >&2
  exit 1
fi

ln -sf "$ROOT/tools/githooks/pre-commit" "$HOOKS_DIR/pre-commit"

if [[ "${1-}" == "--also-pre-push" ]]; then
  ln -sf "$ROOT/tools/githooks/pre-commit" "$HOOKS_DIR/pre-push"
fi

echo "Installed git hooks:"
ls -l "$HOOKS_DIR"/pre-commit 2>/dev/null || true
if [[ "${1-}" == "--also-pre-push" ]]; then
  ls -l "$HOOKS_DIR"/pre-push 2>/dev/null || true
fi
