#!/bin/bash
# Sync version across all crates and hardcoded strings (Base edition)
# Usage: ./scripts/sync-version.sh [new-version]
# If no version provided, reads from loctree_rs/Cargo.toml
#
# Base edition: loctree_rs, reports, loctree_server
# Pro modules (landing, memex) are NOT included.

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

update_file() {
    local file="$1"
    local pattern="$2"

    if [ -f "$file" ]; then
        # BSD sed (macOS) requires an extension for -i, empty string '' works
        # GNU sed (Linux) treats '' as the filename if provided as a separate arg
        if sed --version 2>/dev/null | grep -q GNU; then
             sed -i "$pattern" "$file"
        else
             sed -i '' "$pattern" "$file"
        fi
        echo "  Updated: $file"
    else
        echo "  Skipped (not found): $file"
    fi
}

# Update loctree_rs Cargo.toml
update_file "$ROOT_DIR/loctree_rs/Cargo.toml" 's/^version = ".*"/version = "'$VERSION'"/'

# Update lib.rs docs link
update_file "$ROOT_DIR/loctree_rs/src/lib.rs" 's|html_root_url = "https://docs.rs/loctree/[^"]*"|html_root_url = "https://docs.rs/loctree/'$VERSION'"|'

# Update reports crate footer
update_file "$ROOT_DIR/reports/src/components/document.rs" 's/"loctree v[^"]*"/"loctree v'$VERSION'"/'

echo ""
echo "Version sync complete: v$VERSION"
echo ""
echo "Verify with:"
echo "  grep -r 'v$VERSION\|$VERSION' --include='*.rs' --include='Cargo.toml' $ROOT_DIR | grep -v target | grep -v '#'"
