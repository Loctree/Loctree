#!/bin/sh
set -e

REPO_ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR="$REPO_ROOT/.git/hooks"
TOOLS_HOOKS_DIR="$REPO_ROOT/tools/githooks"

if [ ! -d "$HOOKS_DIR" ]; then
    echo "Error: .git/hooks directory not found. Are you in a git repository?"
    exit 1
fi

echo "Installing git hooks..."

echo "  - pre-commit hook"
cp "$TOOLS_HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"

echo "  - pre-push hook"
cp "$TOOLS_HOOKS_DIR/pre-push" "$HOOKS_DIR/pre-push"
chmod +x "$HOOKS_DIR/pre-push"

echo ""
echo "Hooks installed successfully!"
echo ""
echo "Usage tips:"
echo "  - pre-commit: runs unit tests (use LOCTREE_FAST=1 for quick mode)"
echo "  - pre-push: runs full validation (fmt, clippy, all tests, semgrep)"
