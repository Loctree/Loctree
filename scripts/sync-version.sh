#!/bin/bash
# Sync version across all crates and hardcoded strings
# Usage: ./scripts/sync-version.sh [new-version]
# If no version provided, reads from loctree_rs/Cargo.toml

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Get version from Cargo.toml or argument
if [ -n "$1" ]; then
    VERSION="$1"
else
    VERSION=$(grep '^version = ' "$ROOT_DIR/loctree_rs/Cargo.toml" | head -1 | cut -d'"' -f2)
fi

echo "Syncing version to: $VERSION"

# Files to update with patterns
declare -A PATTERNS=(
    ["$ROOT_DIR/loctree_rs/Cargo.toml"]='s/^version = ".*"/version = "'$VERSION'"/'
    ["$ROOT_DIR/loctree_rs/src/lib.rs"]='s|html_root_url = "https://docs.rs/loctree/[^"]*"|html_root_url = "https://docs.rs/loctree/'$VERSION'"|'
    ["$ROOT_DIR/reports/src/components/document.rs"]='s/"loctree v[^"]*"/"loctree v'$VERSION'"/'
    ["$ROOT_DIR/landing/src/sections/easter_eggs.rs"]='s/v[0-9]\+\.[0-9]\+\.[0-9]\+ | loctree.io/v'$VERSION' | loctree.io/'
)

for file in "${!PATTERNS[@]}"; do
    if [ -f "$file" ]; then
        sed -i '' "${PATTERNS[$file]}" "$file"
        echo "  Updated: $file"
    else
        echo "  Skipped (not found): $file"
    fi
done

echo ""
echo "Version sync complete: v$VERSION"
echo ""
echo "Verify with:"
echo "  grep -r 'v$VERSION\|$VERSION' --include='*.rs' --include='Cargo.toml' | grep -v target | grep -v '#'"
